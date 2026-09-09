use bevy::app::{App, Plugin, PostUpdate, Startup, Update};
use bevy::asset::Assets;
use bevy::camera::{Camera, Camera3d, ClearColorConfig, Exposure, PerspectiveProjection, Projection};
use bevy::color::Color;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::name::Name;
use bevy::ecs::query::{With, Without};
use bevy::ecs::reflect::ReflectResource;
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::light::Atmosphere;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::math::{DVec3, Dir3, Quat, Vec3};
use bevy::pbr::{AtmosphereMode, AtmosphereSettings};
use bevy::post_process::auto_exposure::{AutoExposure, AutoExposurePlugin};
use bevy::post_process::bloom::Bloom;
use bevy::reflect::Reflect;
use bevy::transform::TransformSystems;
use bevy::transform::components::Transform;
use bevy_inspector_egui::bevy_egui::PrimaryEguiContext;
use big_space::prelude::{CellCoord, FloatingOrigin, Grid};

use crate::camera::panorbit::{PanOrbitCamera, PanOrbitCameraTarget};
use crate::config::{CameraSettings, Config};
use crate::follow::{Followee, Follower, PreviousTransform};
use crate::world::{BigWorld, CellPoint};

pub mod panorbit;
pub mod simple;

/// Корень мира создаётся в `PreStartup`, то есть раньше любой системы, которая наполняет сцену.
const WORLD_ROOT_EXPECTED: &str = "world root must be spawned in PreStartup";

#[derive(Clone, Copy, Debug)]
pub struct LookingAt {
    pub target: Vec3,
    pub up: Dir3,
}

#[derive(Clone, Copy, Debug, Resource)]
pub struct AppCameraEntity {
    pub entity_id: Entity,
}

#[derive(Clone, Resource)]
pub struct AppCameraParams {
    pub smoothness_speed: f32,
    pub clear_color: ClearColorConfig,
    pub position: Vec3,
    pub look_at: LookingAt,
    pub exposure: Exposure,
    pub auto_exposure: Option<AutoExposure>,
    pub atmosphere: Option<(Atmosphere, ScatteringMedium, AtmosphereSettings)>,
    pub tonemapping: Tonemapping,
    pub bloom: Bloom,
    pub follower: Follower,
}

impl Default for AppCameraParams {
    fn default() -> Self {
        Self {
            smoothness_speed: 8.0,
            clear_color: ClearColorConfig::None,
            position: Vec3::ZERO,
            look_at: LookingAt {
                target: Vec3::ZERO,
                up: Dir3::Y,
            },
            exposure: Exposure::default(),
            auto_exposure: None,
            atmosphere: None,
            tonemapping: Tonemapping::default(),
            bloom: Bloom::NATURAL,
            follower: Follower::default(),
        }
    }
}

impl AppCameraParams {
    pub fn with_smoothness_speed(mut self, smoothness_speed: f32) -> Self {
        self.smoothness_speed = smoothness_speed;
        self
    }

    pub fn with_clear_color_config(mut self, clear_color: ClearColorConfig) -> Self {
        self.clear_color = clear_color;
        self
    }

    pub fn with_custom_clear_color(mut self, color: Color) -> Self {
        self.clear_color = ClearColorConfig::Custom(color);
        self
    }

    pub fn width_translate(mut self, translate: Vec3) -> Self {
        self.position = translate;
        self
    }

    pub fn width_look_at(mut self, look_at: LookingAt) -> Self {
        self.look_at = look_at;
        self
    }

    pub fn with_exposure(mut self, exposure: Exposure) -> Self {
        self.exposure = exposure;
        self
    }

    pub fn with_auto_exposure(mut self, auto_exposure: AutoExposure) -> Self {
        self.auto_exposure = Some(auto_exposure);
        self
    }

    pub fn with_atmosphere(mut self, atmosphere: (Atmosphere, ScatteringMedium, AtmosphereSettings)) -> Self {
        self.atmosphere = Some(atmosphere);
        self
    }

    pub fn with_tonemapping(mut self, tonemapping: impl Into<Tonemapping>) -> Self {
        self.tonemapping = tonemapping.into();
        self
    }

    pub fn with_bloom(mut self, bloom: impl Into<Bloom>) -> Self {
        self.bloom = bloom.into();
        self
    }

    pub fn with_follower(mut self, follower: Follower) -> Self {
        self.follower = follower;
        self
    }
}

/// Параметры камеры, изменяемые во время игры.
///
/// Источник истины для [`Exposure`], [`Tonemapping`], [`Bloom`], [`AtmosphereSettings`]
/// и сглаживания [`PanOrbitCamera`] активной камеры: изменение полей применяется системой
/// [`apply_camera_params`]. Ресурс, а не компонент, чтобы правки переживали пересоздание
/// камеры в [`respawn_panorbit`].
// `Debug` не выводится: `AtmosphereMode` его не реализует.
#[derive(Clone, Copy, Reflect, Resource)]
#[reflect(Resource)]
pub struct CameraParams {
    pub smoothness_speed: f32,

    pub exposure_ev100: f32,

    pub bloom_intensity: f32,

    pub bloom_low_frequency_boost: f32,

    pub tonemapping: Tonemapping,

    pub atmosphere_mode: AtmosphereMode,
}

impl Default for CameraParams {
    fn default() -> Self {
        Self {
            smoothness_speed: PanOrbitCamera::default().smoothness_speed,
            exposure_ev100: Exposure::default().ev100,
            bloom_intensity: Bloom::NATURAL.intensity,
            bloom_low_frequency_boost: Bloom::NATURAL.low_frequency_boost,
            tonemapping: Tonemapping::default(),
            atmosphere_mode: AtmosphereMode::default(),
        }
    }
}

impl CameraParams {
    pub fn from_config(config: &Config) -> Self {
        Self {
            smoothness_speed: config.camera.smoothness_speed,
            exposure_ev100: config.camera.exposure,
            bloom_intensity: config.camera.bloom.intensity,
            bloom_low_frequency_boost: config.camera.bloom.low_frequency_boost,
            tonemapping: config.camera.tonemap.into(),
            atmosphere_mode: config.environment.atmosphere.render_mode.into(),
            ..Default::default()
        }
    }
}

impl CameraParams {
    /// Переносит параметры в снимок настроек для сохранения в файл конфигурации.
    pub fn write_to(&self, config: &mut Config) {
        config.camera.smoothness_speed = self.smoothness_speed;
        config.camera.exposure = self.exposure_ev100;
        config.camera.bloom.intensity = self.bloom_intensity;
        config.camera.bloom.low_frequency_boost = self.bloom_low_frequency_boost;
        config.camera.tonemap = self.tonemapping.into();
        config.environment.atmosphere.render_mode = self.atmosphere_mode.into();
    }
}

pub fn apply_camera_params(
    params: Res<CameraParams>,
    mut query: Query<(
        &mut PanOrbitCamera,
        &mut Exposure,
        &mut Tonemapping,
        &mut Bloom,
        Option<&mut AtmosphereSettings>,
    )>,
) {
    let params_changed = params.is_changed();

    for (mut camera, mut exposure, mut tonemapping, mut bloom, atmosphere) in &mut query {
        // Камера пересоздаётся при смене состояния игры, поэтому параметры применяются
        // не только при их изменении, но и к только что появившейся камере.
        if !params_changed && !camera.is_added() {
            continue;
        }

        camera.smoothness_speed = params.smoothness_speed;
        exposure.ev100 = params.exposure_ev100;
        *tonemapping = params.tonemapping;
        bloom.intensity = params.bloom_intensity;
        bloom.low_frequency_boost = params.bloom_low_frequency_boost;

        if let Some(mut atmosphere) = atmosphere {
            atmosphere.rendering_method = params.atmosphere_mode;
        }
    }
}

#[derive(Clone, Copy)]
pub struct AppCameraPlugin;

impl Plugin for AppCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AppCameraParams>();

        if let Some(params) = app.world().get_resource::<AppCameraParams>()
            && params.auto_exposure.is_some()
        {
            app.add_plugins(AutoExposurePlugin);
        }

        app.add_systems(Startup, (spawn_atmosphere, spawn_panorbit))
            .add_systems(Update, (panorbit::update_input, panorbit::interpolate_camera).chain())
            .add_systems(Update, panorbit::block_camera_mouse_control_when_hovering_ui)
            // Центр планеты должен встать под камеру до того, как посчитаются
            // `GlobalTransform`, иначе атмосфера отстаёт от камеры на кадр.
            .add_systems(PostUpdate, follow_atmosphere.before(TransformSystems::Propagate));
    }
}

pub fn spawn_atmosphere(
    mut commands: Commands,
    params: Res<AppCameraParams>,
    world: BigWorld,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
) {
    let Some((mut atmosphere, medium, _)) = params.atmosphere.clone() else {
        return;
    };
    let (root, grid) = world.get().expect(WORLD_ROOT_EXPECTED);
    atmosphere.medium = scattering_mediums.add(medium);

    // Радиус планеты — 6.36e6 м: в `f32` такая координата представима с точностью 0.5 м,
    // поэтому центр раскладывается на ячейку и смещение, как и всё остальное в мире.
    let center = CellPoint::from_position(grid, DVec3::NEG_Y * atmosphere.inner_radius as f64);

    commands.spawn((
        Name::new("Atmosphere"),
        ChildOf(root),
        center.cell,
        Transform::from_translation(center.offset),
        atmosphere,
    ));
}

/// Держит центр планеты под камерой.
///
/// Атмосфера в bevy — сфера с центром в `GlobalTransform` своей сущности, а мир в игре плоский.
/// Без поправки плоскость расходится со сферой на d²/2R: 786 м на 100 км от старта и 7.1 км
/// на 300 км, то есть в дальнем конце маршрута небо темнело бы, как на большой высоте.
/// Перенос центра под камеру оставляет высоту над атмосферой равной высоте над ландшафтом.
/// Честная альтернатива — настоящая кривизна Земли, она отдельным пунктом в бэклоге.
pub fn follow_atmosphere(
    world: BigWorld,
    origin: Query<(&CellCoord, &Transform), With<FloatingOrigin>>,
    mut atmospheres: Query<(&Atmosphere, &mut CellCoord, &mut Transform), Without<FloatingOrigin>>,
) {
    let Some(grid) = world.grid() else {
        return;
    };
    let Ok((origin_cell, origin_transform)) = origin.single() else {
        return;
    };

    let origin_position = CellPoint::new(*origin_cell, origin_transform.translation).position(grid);

    for (atmosphere, mut cell, mut transform) in &mut atmospheres {
        let center = CellPoint::from_position(
            grid,
            DVec3::new(origin_position.x, -(atmosphere.inner_radius as f64), origin_position.z),
        );

        // Присваивание через сравнение: пока камера стоит, атмосфера не должна дёргать
        // change detection и пересчёт таблиц рассеяния.
        if *cell != center.cell {
            *cell = center.cell;
        }
        if transform.translation != center.offset {
            transform.translation = center.offset;
        }
    }
}

pub fn spawn_panorbit(mut commands: Commands, params: Res<AppCameraParams>, world: BigWorld) {
    let (root, grid) = world.get().expect(WORLD_ROOT_EXPECTED);

    spawn_camera(&mut commands, &params, root, grid);
}

/// Создаёт камеру в мире с плавающим началом координат.
///
/// Камера — потомок корневого `BigSpace` и носитель [`FloatingOrigin`]: `GlobalTransform`
/// всех сущностей мира считается относительно её ячейки, поэтому ошибка `f32` при рендере
/// всегда мала рядом с камерой, куда бы она ни улетела.
fn spawn_camera(commands: &mut Commands, params: &AppCameraParams, root: Entity, grid: &Grid) {
    let target = PanOrbitCameraTarget::new(grid, params.position, params.look_at);
    let camera = PanOrbitCamera {
        radius: target.radius,
        focus: target.focus,
        ..Default::default()
    };

    let mut cell = CellCoord::default();
    let mut transform = Transform::from_rotation(target.rotation);
    camera.update_position(&mut transform, &mut cell);

    let mut entity = commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: params.clear_color,
            ..Default::default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 45.0_f32.to_radians(),
            ..Default::default()
        }),
        camera,
        PrimaryEguiContext,
        target,
        ChildOf(root),
        FloatingOrigin,
        cell,
        transform,
        params.follower,
        // The directional light illuminance used in this scene
        // (the one recommended for use with this feature) is
        // quite bright, so raising the exposure compensation helps
        // bring the scene to a nicer brightness range.
        params.exposure.clone(),
        // Tonemapper chosen just because it looked good with the scene, any
        // tonemapper would be fine :)
        params.tonemapping,
        // Bloom gives the sun a much more natural look.
        params.bloom.clone(),
    ));

    if let Some(auto_exposure) = params.auto_exposure.clone() {
        entity.insert(auto_exposure);
    }

    if let Some((_, _, settings)) = params.atmosphere.clone() {
        entity.insert(settings);
    }

    let entity_id = entity.id();
    commands.insert_resource(AppCameraEntity { entity_id });
}

pub fn respawn_panorbit(
    commands: &mut Commands,
    params: &mut AppCameraParams,
    world: &BigWorld,
    camera: Entity,
    settings: &CameraSettings,
    height: f32,
) {
    let Some((root, grid)) = world.get() else {
        return;
    };

    commands.entity(camera).despawn();

    let (position, target) = if let Some(preset) = settings.presets.first() {
        let (position, target) = preset.to_vec3s();
        let additional_translate = Vec3::ZERO.with_y(height);

        (position + additional_translate, target + additional_translate)
    } else {
        let x = settings.follow.distance / 3.0;
        let y = x / 2.0;
        let z = settings.follow.distance * 31_f32.sqrt() / 6.0;
        let position = Vec3::new(x, y, z);
        let additional_translate = Vec3::ZERO.with_y(height + settings.follow.height);

        (position + additional_translate, additional_translate)
    };

    params.position = position;
    params.look_at.target = target;

    spawn_camera(commands, params, root, grid);
}

pub fn preset_toggle(
    config: Res<Config>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    world: BigWorld,
    followee_query: Query<(&CellCoord, &Transform), With<Followee>>,
    mut camera_query: Query<(&mut PanOrbitCameraTarget, &Follower), With<PanOrbitCamera>>,
) {
    if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        let mut preset_idx = None;
        if keyboard_input.just_pressed(KeyCode::Digit1) {
            preset_idx = Some(0);
        }
        if keyboard_input.just_pressed(KeyCode::Digit2) {
            preset_idx = Some(1);
        }
        if keyboard_input.just_pressed(KeyCode::Digit3) {
            preset_idx = Some(2);
        }
        if keyboard_input.just_pressed(KeyCode::Digit4) {
            preset_idx = Some(3);
        }
        if keyboard_input.just_pressed(KeyCode::Digit5) {
            preset_idx = Some(4);
        }

        if let Some(preset) = preset_idx.and_then(|idx| config.camera.presets.get(idx))
            && let Some(grid) = world.grid()
            && let Some((mut camera_target, follower)) = camera_query.iter_mut().next()
        {
            let (position, target) = preset.to_vec3s();
            let (radius, rotation) = PanOrbitCameraTarget::orbit(position, LookingAt { target, up: Dir3::Y });

            // Пресет задаёт орбиту относительно цели слежения: точка взгляда прибавляется
            // к её положению, поворот — к её повороту. Без цели точкой отсчёта остаётся
            // текущий фокус камеры.
            let (mut focus, delta_rotation) = follower
                .followee
                .and_then(|followee_entity| followee_query.get(followee_entity).ok())
                .map(|(cell, transform)| (CellPoint::new(*cell, transform.translation), transform.rotation))
                .unwrap_or((camera_target.focus, Quat::IDENTITY));

            focus.translate(grid, target);

            camera_target.focus = focus;
            camera_target.radius = radius;
            camera_target.rotation = delta_rotation * rotation;
        }
    }
}

pub fn follow_toggle(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut follower_query: Query<&mut Follower, (With<Camera3d>, Without<Followee>)>,
    followee_query: Query<Entity, With<Followee>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyF) {
        for mut follower in &mut follower_query {
            if follower.followee.is_none() {
                follower.followee = followee_query.iter().next();
            } else {
                follower.followee = None;
            }
        }
    }

    if keyboard_input.just_pressed(KeyCode::KeyT) {
        for mut follower in &mut follower_query {
            follower.turn_towards = !follower.turn_towards;
        }
    }
}

pub fn follow_move(
    world: BigWorld,
    followee_query: Query<(&CellCoord, &Transform, &PreviousTransform), With<Followee>>,
    mut follower_query: Query<
        (
            &mut PanOrbitCamera,
            &mut PanOrbitCameraTarget,
            &mut Transform,
            &mut CellCoord,
            &Follower,
        ),
        Without<Followee>,
    >,
) {
    let Some(grid) = world.grid() else {
        return;
    };

    for (mut camera, mut target, mut transform, mut cell, follower) in &mut follower_query {
        if let Some(target_entity) = follower.followee {
            if let Ok((followee_cell, followee_transform, followee_prev_transform)) = followee_query.get(target_entity)
            {
                if follower.turn_towards {
                    let delta_rotation = followee_transform.rotation * followee_prev_transform.rotation.inverse();
                    target.rotation = delta_rotation * target.rotation;
                }

                // Разность абсолютных координат здесь была бы катастрофическим сокращением:
                // на 300 км шаг `f32` — 15 мм при кадровом шаге порядка метра. Дельта по
                // ячейкам и смещениям точна на любом удалении и не врёт на границе ячейки.
                let followee_point = CellPoint::new(*followee_cell, followee_transform.translation);
                let delta_focus = followee_point.delta_from(grid, &followee_prev_transform.point);

                target.focus.translate(grid, delta_focus);
                camera.focus.translate(grid, delta_focus);
                camera.update_position(&mut transform, &mut cell);
            }
        }
    }
}
