use bevy::app::{App, Plugin, Startup, Update};
use bevy::color::Color;
use bevy::core_pipeline::auto_exposure::{AutoExposure, AutoExposurePlugin};
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::core_3d::Camera3d;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::Commands;
use bevy::math::{Dir3, Vec3};
use bevy::pbr::{Atmosphere, AtmosphereSettings};
use bevy::render::camera::{Camera, ClearColorConfig, Exposure, PerspectiveProjection, Projection};
use bevy::transform::components::Transform;

use crate::camera::panorbit::{PanOrbitCamera, PanOrbitCameraTarget};

pub mod panorbit;
pub mod simple;

#[derive(Clone, Copy)]
pub struct LookingAt {
    pub target: Vec3,
    pub up: Dir3,
}

#[derive(Clone, Copy)]
pub struct AppCameraPlugin {
    smoothness_speed: f32,
    clear_color: ClearColorConfig,
    translate: Vec3,
    look_at: LookingAt,
    exposure: Exposure,
    auto_exposure: Option<fn() -> AutoExposure>,
    atmosphere: Option<fn() -> (Atmosphere, AtmosphereSettings)>,
}

impl Default for AppCameraPlugin {
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
        }
    }
}

impl AppCameraPlugin {
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

    pub fn with_auto_exposure(mut self, auto_exposure: fn() -> AutoExposure) -> Self {
        self.auto_exposure = Some(auto_exposure);
        self
    }

    pub fn with_atmosphere(mut self, atmosphere: fn() -> (Atmosphere, AtmosphereSettings)) -> Self {
        self.atmosphere = Some(atmosphere);
        self
    }

    pub fn spawn_panorbit(self, mut commands: Commands) {
        let focus = self.look_at.target;
        let radius = (self.translate - focus).length();
        let transform = Transform::from_translation(self.translate).looking_at(self.look_at.target, self.look_at.up);

        let mut entity = commands.spawn((
            Camera3d::default(),
            Camera {
                hdr: true,
                clear_color: self.clear_color,
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
            self.exposure,
            // Tonemapper chosen just because it looked good with the scene, any
            // tonemapper would be fine :)
            // Tonemapping::AcesFitted,
            Tonemapping::BlenderFilmic,
            // Bloom gives the sun a much more natural look.
            Bloom::NATURAL,
        ));

        if let Some(auto_exposure) = self.auto_exposure {
            entity.insert(auto_exposure());
        }

        if let Some(atmosphere) = self.atmosphere {
            entity.insert(atmosphere());
        }
    }
}

impl Plugin for AppCameraPlugin {
    fn build(&self, app: &mut App) {
        let this = self.clone();

        if this.auto_exposure.is_some() {
            app.add_plugins(AutoExposurePlugin);
        }
        app.add_systems(Startup, move |commands: Commands| this.spawn_panorbit(commands))
            .add_systems(Update, (panorbit::update_input, panorbit::interpolate_camera).chain());
    }
}
