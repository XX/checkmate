use bevy::asset::AssetServer;
use bevy::ecs::component::Component;
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::name::Name;
use bevy::ecs::query::Changed;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::gltf::GltfAssetLabel;
use bevy::math::{DVec3, Quat, Vec3};
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::transform::components::Transform;
use bevy::world_serialization::WorldAssetRoot;
use big_space::prelude::{CellCoord, Grid};

use crate::config::{Config, Rotation, TerrainSettings};
use crate::state::ingame::GameData;
use crate::state::{SceneKey, Scenes};
use crate::world::BigWorld;

#[derive(Component, Reflect, Debug, Default, Clone, Copy)]
#[reflect(Component, Default)]
pub struct Terrain;

/// Параметры размещения ландшафта, изменяемые во время игры.
///
/// Источник истины для [`Transform`] сущности ландшафта: изменение полей применяется
/// системой [`apply_terrain`]. Путь к модели остаётся структурным параметром и берётся
/// из конфига при создании сцены.
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct TerrainParams {
    /// Положение в мире, в метрах.
    pub position: DVec3,

    pub rotation: Quat,

    pub scale: f32,
}

impl From<&TerrainSettings> for TerrainParams {
    fn from(settings: &TerrainSettings) -> Self {
        Self {
            position: DVec3::from(settings.position),
            rotation: settings.get_rotation(),
            scale: settings.scale,
        }
    }
}

impl TerrainParams {
    /// Собирает настройки для сохранения в конфиг.
    ///
    /// Путь к модели структурный, поэтому берётся из `base`, оттуда же берётся исходная ось
    /// `from`: конфиг задаёт поворот парой векторов, и по кватерниону однозначно
    /// восстанавливается только `to`. Крен вокруг оси `from`–`to`, если его задали правкой
    /// кватерниона напрямую, в такой записи не сохраняется.
    pub fn to_settings(&self, base: &TerrainSettings) -> TerrainSettings {
        let from = base
            .rotation
            .map(|rotation| Vec3::from(rotation.from))
            .unwrap_or(Vec3::Z)
            .normalize();

        let rotation = (self.rotation != Quat::IDENTITY).then(|| Rotation {
            from: from.into(),
            to: (self.rotation * from).into(),
        });

        TerrainSettings {
            model: base.model.clone(),
            position: self.position.to_array(),
            rotation,
            scale: self.scale,
        }
    }

    /// Раскладывает положение на ячейку сетки и трансформ внутри неё.
    pub fn cell_transform(&self, grid: &Grid) -> (CellCoord, Transform) {
        let (cell, translation) = grid.translation_to_grid(self.position);
        let transform = Transform::from_translation(translation)
            .with_rotation(self.rotation)
            .with_scale(Vec3::splat(self.scale));

        (cell, transform)
    }
}

pub fn setup(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    world: BigWorld,
    mut scenes: ResMut<Scenes>,
    mut data: ResMut<GameData>,
) {
    let Some((root, grid)) = world.get() else {
        return;
    };

    // Terrain
    let scene = scenes
        .game
        .entry(SceneKey::Terrain)
        .or_insert_with(|| {
            let model_path = config.game.terrain.model.clone();
            if model_path.ends_with(".gltf") || model_path.ends_with(".glb") {
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(model_path))
            } else {
                asset_server.load(model_path)
            }
        })
        .clone();

    let params = TerrainParams::from(&config.game.terrain);
    let (cell, transform) = params.cell_transform(grid);
    let terrain_id = commands
        .spawn((
            Name::new("Terrain"),
            Terrain,
            params,
            WorldAssetRoot(scene.clone()),
            ChildOf(root),
            cell,
            transform,
        ))
        .id();

    data.entities.push(terrain_id);
}

pub fn apply_terrain(
    world: BigWorld,
    mut query: Query<(&TerrainParams, &mut CellCoord, &mut Transform), Changed<TerrainParams>>,
) {
    let Some(grid) = world.grid() else {
        return;
    };

    for (params, mut cell, mut transform) in &mut query {
        let (new_cell, new_transform) = params.cell_transform(grid);

        *cell = new_cell;
        *transform = new_transform;
    }
}
