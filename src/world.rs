//! Мир с высокой точностью координат.
//!
//! Штатный `TransformPlugin` считает `GlobalTransform` в `f32` от начала координат, а в
//! большом мире он должен считаться относительно камеры.
//! 
//! У `f32` 24 бита мантиссы, поэтому шаг представимых значений растёт вместе с координатой:
//! на 100 км это уже 8 мм, на 500 км — 31 мм. Само по себе это терпимо, но разности больших
//! чисел (матрица вида, дельта между кадрами, сглаживание камеры) теряют младшие разряды,
//! и полёты на сотни километров превращаются в дрожание меша и «плавающую» камеру.
//!
//! Обход — плавающее начало координат из [`big_space`]: положение сущности описывается парой
//! «целочисленный индекс ячейки [`CellCoord`] + [`Transform`] внутри ячейки», а
//! `GlobalTransform` считается относительно сущности с [`FloatingOrigin`] (камеры). Точность
//! `Transform` не зависит от удаления от начала координат, потому что смещение внутри ячейки
//! никогда не превышает её половины, а ошибка `GlobalTransform` всегда мала рядом с камерой.
//!
//! Все видимые сущности сцены должны быть потомками корневого [`BigSpace`]: сущности вне него
//! получают `GlobalTransform` из «сырого» `Transform`, то есть при удалении камеры от начала
//! координат поедут вместе с ней.
//!
//! Разбор задачи и выбор решения: `issues/big world coordinates.md`.

use bevy::app::{App, Plugin, PreStartup};
use bevy::camera::visibility::Visibility;
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::query::With;
use bevy::ecs::system::{Commands, Query, SystemParam};
use bevy::math::{DVec3, Vec3};
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use big_space::prelude::{BigSpace, BigSpaceCommands, BigSpaceDefaultPlugins, CellCoord, Grid};

/// Точка мира с высокой точностью: индекс ячейки сетки и смещение внутри неё.
///
/// Смысл тот же, что у пары [`CellCoord`] + [`Transform`] на сущности, но без сущности —
/// для тех мест, где игре нужно хранить точку самой (фокус орбитальной камеры, положение
/// в предыдущем кадре). Операции сложения и вычитания идут по ячейкам и смещениям отдельно,
/// поэтому точность результата определяется размером ячейки, а не удалением от начала координат.
#[derive(Reflect, Debug, Default, Clone, Copy, PartialEq)]
#[reflect(Default)]
pub struct CellPoint {
    pub cell: CellCoord,

    /// Смещение относительно центра ячейки [`Self::cell`].
    pub offset: Vec3,
}

impl CellPoint {
    pub fn new(cell: CellCoord, offset: Vec3) -> Self {
        Self { cell, offset }
    }

    /// Раскладывает абсолютную позицию на ячейку и смещение внутри неё.
    pub fn from_position(grid: &Grid, position: DVec3) -> Self {
        let (cell, offset) = grid.translation_to_grid(position);
        Self { cell, offset }
    }

    /// Абсолютная позиция точки.
    ///
    /// Возвращает `f64`: в `f32` координата на сотнях километров уже не представима точно,
    /// ради чего вся эта пара и заведена.
    pub fn position(&self, grid: &Grid) -> DVec3 {
        self.cell.as_dvec3(grid) + self.offset.as_dvec3()
    }

    /// Вектор от `origin` до этой точки.
    ///
    /// Для близких точек результат точен независимо от того, как далеко обе от начала
    /// координат: разность ячеек целочисленная, разность смещений — в пределах ячейки.
    pub fn delta_from(&self, grid: &Grid, origin: &Self) -> Vec3 {
        grid.cell_to_float(&(self.cell - origin.cell)).as_vec3() + (self.offset - origin.offset)
    }

    /// Сдвигает точку на `delta`.
    pub fn translate(&mut self, grid: &Grid, delta: Vec3) {
        self.offset += delta;
        self.normalize(grid);
    }

    /// Возвращает смещение в пределы ячейки, перенося избыток в её индекс.
    ///
    /// Вызывается после каждого сдвига: иначе смещение накапливается и точность падает
    /// ровно так же, как у обычного `Vec3`.
    pub fn normalize(&mut self, grid: &Grid) {
        let (cell, offset) = grid.imprecise_translation_to_grid(self.offset);
        self.cell += cell;
        self.offset = offset;
    }
}

/// Доступ к корневой сетке мира.
///
/// В игре ровно один [`BigSpace`], и все игровые сущности — его прямые потомки, поэтому
/// системам достаточно этого параметра, чтобы получить и корень (для порождения потомков),
/// и [`Grid`] (для пересчёта координат).
#[derive(SystemParam)]
pub struct BigWorld<'w, 's> {
    root: Query<'w, 's, (Entity, &'static Grid), With<BigSpace>>,
}

impl BigWorld<'_, '_> {
    /// Корневая сущность мира и её сетка.
    pub fn get(&self) -> Option<(Entity, &Grid)> {
        self.root.single().ok()
    }

    /// Сущность, потомками которой должны быть все игровые сущности.
    pub fn root(&self) -> Option<Entity> {
        self.get().map(|(root, _)| root)
    }

    /// Сетка, в которой заданы координаты игровых сущностей.
    pub fn grid(&self) -> Option<&Grid> {
        self.get().map(|(_, grid)| grid)
    }
}

pub struct BigWorldPlugin;

impl Plugin for BigWorldPlugin {
    fn build(&self, app: &mut App) {
        // `BigSpaceDefaultPlugins` заменяет распространение трансформов своим, поэтому штатный
        // `TransformPlugin` в `main` отключён; при включённом плагин паникует на старте.
        app.add_plugins(BigSpaceDefaultPlugins)
            .register_type::<CellPoint>()
            .add_systems(PreStartup, spawn_world_root);
    }
}

/// Создаёт корень мира до того, как `Startup` начнёт наполнять сцену.
///
/// Размер ячейки — [`Grid::default`], то есть ребро 2 км и порог перехода 100 м. Внутри ячейки
/// координата не превышает километра, где шаг `f32` равен 0.06 мм, а точности `i32`-индекса
/// (фича `i32` у [`big_space`]) хватает на 8.6e12 м — на много порядков больше, чем нужно.
pub fn spawn_world_root(mut commands: Commands) {
    commands.spawn_big_space(Grid::default(), |root| {
        // `Visibility` не входит в корневой набор `big_space` (он добавляет её только под
        // своей фичей `camera`, которая тянет ненужный здесь контроллер камеры), а без неё
        // потомки корня жалуются на разрыв в наследовании видимости — предупреждение B0004.
        root.insert((Name::new("World"), Visibility::default()));
    });
}

#[cfg(test)]
mod tests {
    use bevy::MinimalPlugins;
    use bevy::ecs::hierarchy::ChildOf;
    use bevy::transform::components::{GlobalTransform, Transform};
    use big_space::prelude::FloatingOrigin;

    use super::*;

    /// Удаление, ради которого всё это и затевалось: шаг сетки `f32` здесь 31 мм.
    const FAR: f64 = 300_000.0;

    /// Мелкий шаг: хвост экспоненциального сглаживания камеры, доводка положения,
    /// медленное движение. Именно такие величины теряются первыми.
    const STEP: Vec3 = Vec3::splat(0.01);

    #[test]
    fn small_step_survives_far_from_origin() {
        let grid = Grid::default();
        // Позиция намеренно не кратна ребру ячейки: смещение внутри неё должно быть ненулевым.
        let point = CellPoint::from_position(&grid, DVec3::new(FAR + 137.5, 2137.25, -(FAR + 42.75)));

        let mut moved = point;
        moved.translate(&grid, STEP);

        // Ошибка остаётся на уровне шага `f32` внутри ячейки — это микроны, а не сантиметры.
        let error = (moved.delta_from(&grid, &point) - STEP).abs().max_element();
        assert!(error < 1e-4, "delta error is {error} m");
    }

    /// Контрольный опыт: на абсолютных `f32` тот же шаг на 300 км исчезает целиком —
    /// он меньше половины шага сетки представимых значений.
    #[test]
    fn naive_f32_loses_the_same_step() {
        let position = FAR as f32;

        assert_eq!((position + STEP.x) - position, 0.0);
    }

    /// Сквозная проверка того, ради чего всё затевалось: как бы далеко ни улетела камера,
    /// взаимное положение соседних с ней сущностей в `GlobalTransform` остаётся точным,
    /// а сама камера при этом рендерится рядом с началом координат.
    #[test]
    fn rendering_stays_precise_far_from_origin() {
        let probe_offset = Vec3::new(1.5, 0.25, -0.75);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(BigWorldPlugin);
        app.update();

        let mut roots = app.world_mut().query_filtered::<Entity, With<BigSpace>>();
        let root = roots.single(app.world()).expect("world root must exist");
        let grid = app
            .world()
            .entity(root)
            .get::<Grid>()
            .expect("world root must have a grid")
            .clone();

        let (cell, offset) = grid.translation_to_grid(DVec3::new(FAR, 3000.0, -FAR));

        let camera = app
            .world_mut()
            .spawn((ChildOf(root), FloatingOrigin, cell, Transform::from_translation(offset)))
            .id();
        let probe = app
            .world_mut()
            .spawn((ChildOf(root), cell, Transform::from_translation(offset + probe_offset)))
            .id();

        app.update();

        let position = |entity: Entity| {
            app.world()
                .entity(entity)
                .get::<GlobalTransform>()
                .expect("spatial entity must have a global transform")
                .translation()
        };
        let camera_position = position(camera);

        let error = ((position(probe) - camera_position) - probe_offset).abs().max_element();
        assert!(error < 1e-4, "relative position error is {error} m");

        assert!(
            camera_position.abs().max_element() <= grid.cell_edge_length(),
            "camera must be rendered near the origin, got {camera_position}"
        );
    }

    /// Смещение не должно накапливаться: как бы далеко точка ни уехала, оно остаётся
    /// в пределах ячейки, где у `f32` ещё есть доли миллиметра.
    #[test]
    fn offset_never_leaves_the_cell() {
        let grid = Grid::default();
        let mut point = CellPoint::default();

        for _ in 0..10_000 {
            point.translate(&grid, Vec3::X * 100.0);
        }

        assert!(point.offset.abs().max_element() <= grid.maximum_distance_from_origin());
        assert_eq!(point.position(&grid).x, 1_000_000.0);
    }
}
