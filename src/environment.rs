use bevy::color::Color;
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::component::Component;
use bevy::ecs::name::Name;
use bevy::ecs::query::Changed;
use bevy::ecs::reflect::{ReflectComponent, ReflectResource};
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::light::{DirectionalLight, GlobalAmbientLight, SunDisk};
use bevy::math::Vec3;
use bevy::prelude::default;
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::transform::components::Transform;

use crate::config::{AmbientSettings, Config, SunSettings, color_from_array, color_to_array};

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

impl From<&SunParams> for SunSettings {
    fn from(params: &SunParams) -> Self {
        Self {
            illuminance: params.illuminance,
            angular_size: params.angular_size,
            intensity: params.intensity,
            shadows_enabled: params.shadows_enabled,
            position: params.position.into(),
            target: params.target.into(),
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

/// Параметры фонового света.
///
/// Источник истины для ресурса [`GlobalAmbientLight`]: изменение полей применяется системой
/// [`apply_ambient`]. Ресурс, а не компонент, потому что фоновый свет в bevy — тоже ресурс.
#[derive(Resource, Reflect, Debug, Clone, Copy)]
#[reflect(Resource)]
pub struct AmbientParams {
    pub enabled: bool,

    pub color: Color,

    pub brightness: f32,

    pub affects_lightmapped_meshes: bool,
}

impl Default for AmbientParams {
    fn default() -> Self {
        Self::from(&AmbientSettings::default())
    }
}

impl From<&AmbientSettings> for AmbientParams {
    fn from(settings: &AmbientSettings) -> Self {
        Self {
            enabled: settings.enabled,
            color: color_from_array(settings.color),
            brightness: settings.brightness,
            affects_lightmapped_meshes: settings.affects_lightmapped_meshes,
        }
    }
}

impl From<&AmbientParams> for AmbientSettings {
    fn from(params: &AmbientParams) -> Self {
        Self {
            enabled: params.enabled,
            color: color_to_array(params.color),
            brightness: params.brightness,
            affects_lightmapped_meshes: params.affects_lightmapped_meshes,
        }
    }
}

impl AmbientParams {
    /// Выключенный фоновый свет оставляет значение bevy по умолчанию — так же, как когда
    /// секция конфига выключена и игра ничего не вставляет в мир сама.
    pub fn global_ambient_light(&self) -> GlobalAmbientLight {
        if self.enabled {
            GlobalAmbientLight {
                color: self.color,
                brightness: self.brightness,
                affects_lightmapped_meshes: self.affects_lightmapped_meshes,
            }
        } else {
            GlobalAmbientLight::default()
        }
    }
}

pub fn apply_ambient(params: Res<AmbientParams>, mut ambient_light: ResMut<GlobalAmbientLight>) {
    if !params.is_changed() {
        return;
    }

    *ambient_light = params.global_ambient_light();
}
