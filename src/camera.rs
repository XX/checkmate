use bevy::app::{App, Plugin, Startup, Update};
use bevy::color::Color;
use bevy::core_pipeline::auto_exposure::{AutoExposure, AutoExposurePlugin};
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::core_3d::Camera3d;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::entity::Entity;
use bevy::ecs::query::{With, Without};
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::math::{Dir3, FloatPow, Vec3};
use bevy::pbr::{Atmosphere, AtmosphereSettings};
use bevy::render::camera::{Camera, ClearColorConfig, Exposure, PerspectiveProjection, Projection};
use bevy::transform::components::Transform;

use crate::camera::panorbit::{PanOrbitCamera, PanOrbitCameraTarget};
use crate::config::CameraFollowSettings;
use crate::follow::{Followee, Follower, PreviousTransform};

pub mod panorbit;
pub mod simple;

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
    pub translate: Vec3,
    pub look_at: LookingAt,
    pub exposure: Exposure,
    pub auto_exposure: Option<AutoExposure>,
    pub atmosphere: Option<(Atmosphere, AtmosphereSettings)>,
    pub tonemapping: Tonemapping,
    pub follower: Follower,
}

impl Default for AppCameraParams {
    fn default() -> Self {
        Self {
            smoothness_speed: 8.0,
            clear_color: ClearColorConfig::None,
            translate: Vec3::ZERO,
            look_at: LookingAt {
                target: Vec3::ZERO,
                up: Dir3::Y,
            },
            exposure: Exposure::default(),
            auto_exposure: None,
            atmosphere: None,
            tonemapping: Tonemapping::default(),
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
        self.translate = translate;
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

    pub fn with_atmosphere(mut self, atmosphere: (Atmosphere, AtmosphereSettings)) -> Self {
        self.atmosphere = Some(atmosphere);
        self
    }

    pub fn with_tonemapping(mut self, tonemapping: impl Into<Tonemapping>) -> Self {
        self.tonemapping = tonemapping.into();
        self
    }

    pub fn with_follower(mut self, follower: Follower) -> Self {
        self.follower = follower;
        self
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

        app.add_systems(Startup, spawn_panorbit)
            .add_systems(Update, (panorbit::update_input, panorbit::interpolate_camera).chain());
    }
}

pub fn spawn_panorbit(mut commands: Commands, params: Res<AppCameraParams>) {
    let focus = params.look_at.target;
    let radius = (params.translate - focus).length();
    let transform = Transform::from_translation(params.translate).looking_at(params.look_at.target, params.look_at.up);

    let mut entity = commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            clear_color: params.clear_color,
            ..Default::default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 45.0_f32.to_radians(),
            ..Default::default()
        }),
        PanOrbitCamera {
            radius,
            focus,
            ..Default::default()
        },
        PanOrbitCameraTarget {
            focus,
            radius,
            rotation: transform.rotation,
        },
        params.follower,
        transform,
        // The directional light illuminance used in this scene
        // (the one recommended for use with this feature) is
        // quite bright, so raising the exposure compensation helps
        // bring the scene to a nicer brightness range.
        params.exposure.clone(),
        // Tonemapper chosen just because it looked good with the scene, any
        // tonemapper would be fine :)
        params.tonemapping,
        // Bloom gives the sun a much more natural look.
        Bloom::NATURAL,
    ));

    if let Some(auto_exposure) = params.auto_exposure.clone() {
        entity.insert(auto_exposure);
    }

    if let Some(atmosphere) = params.atmosphere.clone() {
        entity.insert(atmosphere);
    }

    let entity_id = entity.id();
    commands.insert_resource(AppCameraEntity { entity_id });
}

pub fn respawn_panorbit(
    mut commands: Commands,
    mut params: ResMut<AppCameraParams>,
    camera: Entity,
    settings: &CameraFollowSettings,
    height: f32,
) {
    commands.entity(camera).despawn();

    let height = height + settings.height;
    let x = settings.distance / 3.0;
    let y = x / 2.0;
    let z = settings.distance * 31_f32.sqrt() / 6.0;
    let translate = Vec3::new(x, height + y, z);
    let target = Vec3::ZERO.with_y(height);

    params.translate = translate;
    params.look_at.target = target;

    spawn_panorbit(commands, params.into());
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
    followee_query: Query<(&Transform, &PreviousTransform), With<Followee>>,
    mut follower_query: Query<
        (
            &mut PanOrbitCamera,
            &mut PanOrbitCameraTarget,
            &mut Transform,
            &Follower,
        ),
        Without<Followee>,
    >,
) {
    for (mut camera, mut target, mut transform, follower) in &mut follower_query {
        if let Some(target_entity) = follower.followee {
            if let Ok((followee_transform, followee_prev_transform)) = followee_query.get(target_entity) {
                if follower.turn_towards {
                    let delta_rotation = followee_transform.rotation * followee_prev_transform.0.rotation.inverse();
                    target.rotation = delta_rotation * target.rotation;
                }

                let focus = followee_transform.translation;
                camera.focus = focus;
                target.focus = focus;
                camera.update_position(&mut transform);
            }
        }
    }
}
