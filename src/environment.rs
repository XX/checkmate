use bevy::ecs::component::Component;
use bevy::ecs::name::Name;
use bevy::ecs::query::Changed;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::light::{DirectionalLight, SunDisk};
use bevy::math::Vec3;
use bevy::prelude::default;
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::transform::components::Transform;

use crate::config::{Config, SunSettings};

#[derive(Component, Reflect, Debug, Default, Clone, Copy)]
#[reflect(Component, Default)]
pub struct Sun;

/// Параметры солнца, изменяемые во время игры.
///
/// Источник истины для [`DirectionalLight`], [`SunDisk`] и [`Transform`] сущности солнца:
/// изменение полей применяется системой [`apply_sun`].
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct SunParams {
    pub illuminance: f32,

    pub angular_size: f32,

    pub intensity: f32,

    pub shadows_enabled: bool,

    pub position: Vec3,

    pub target: Vec3,
}

impl From<&SunSettings> for SunParams {
    fn from(settings: &SunSettings) -> Self {
        Self {
            illuminance: settings.illuminance,
            angular_size: settings.angular_size,
            intensity: settings.intensity,
            shadows_enabled: settings.shadows_enabled,
            position: settings.position.into(),
            target: settings.target.into(),
        }
    }
}

impl SunParams {
    pub fn directional_light(&self) -> DirectionalLight {
        DirectionalLight {
            shadow_maps_enabled: self.shadows_enabled,
            illuminance: self.illuminance,
            ..default()
        }
    }

    pub fn sun_disk(&self) -> SunDisk {
        SunDisk {
            angular_size: self.angular_size,
            intensity: self.intensity,
        }
    }

    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.position).looking_at(self.target, Vec3::Y)
    }
}

pub fn setup(mut commands: Commands, config: Res<Config>) {
    let params = SunParams::from(&config.environment.sun);

    commands.spawn((
        Name::new("Sun"),
        Sun,
        params,
        params.directional_light(),
        params.sun_disk(),
        params.transform(),
    ));
}

pub fn apply_sun(
    mut query: Query<(&SunParams, &mut DirectionalLight, &mut SunDisk, &mut Transform), Changed<SunParams>>,
) {
    for (params, mut light, mut sun_disk, mut transform) in &mut query {
        *light = params.directional_light();
        *sun_disk = params.sun_disk();
        *transform = params.transform();
    }
}
