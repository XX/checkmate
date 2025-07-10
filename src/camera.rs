use bevy::app::{App, Plugin, Startup, Update};
use bevy::color::Color;
use bevy::core_pipeline::auto_exposure::{AutoExposure, AutoExposurePlugin};
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::core_3d::Camera3d;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::entity::Entity;
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Res};
use bevy::math::{Dir3, Vec3};
use bevy::pbr::{Atmosphere, AtmosphereSettings};
use bevy::render::camera::{Camera, ClearColorConfig, Exposure, PerspectiveProjection, Projection};
use bevy::transform::components::Transform;

use crate::camera::panorbit::{PanOrbitCamera, PanOrbitCameraTarget};

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
