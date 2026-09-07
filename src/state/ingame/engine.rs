use bevy::color::Color;
use bevy::ecs::component::Component;
use bevy::ecs::name::Name;
use bevy::ecs::query::Changed;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::light::PointLight;
use bevy::math::Vec3;
use bevy::reflect::Reflect;
use bevy::time::{Time, Timer, TimerMode};
use bevy::transform::components::Transform;

use crate::config::{Config, FlickeringSettings, JetFireSettings};
use crate::state::ingame::GameData;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct FlickeringLight {
    base_intensity: f32,
    variation: f32,
    timer: Timer,
}

/// Параметры мерцания сопла, изменяемые во время игры.
#[derive(Reflect, Debug, Clone, Copy)]
pub struct FlickeringParams {
    pub variation: f32,

    pub frequency: f32,
}

impl From<&FlickeringSettings> for FlickeringParams {
    fn from(settings: &FlickeringSettings) -> Self {
        Self {
            variation: settings.variation,
            frequency: settings.frequency,
        }
    }
}

/// Параметры пламени сопла, изменяемые во время игры.
///
/// Источник истины для [`PointLight`], [`FlickeringLight`] и [`Transform`] сущности пламени:
/// изменение полей применяется системой [`apply_jet_fire`].
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct JetFireParams {
    pub intensity: f32,

    pub color: Color,

    pub radius: f32,

    pub range: f32,

    pub position: Vec3,

    pub flickering: FlickeringParams,
}

impl From<&JetFireSettings> for JetFireParams {
    fn from(settings: &JetFireSettings) -> Self {
        Self {
            intensity: settings.intensity,
            color: Color::srgb_from_array(settings.color),
            radius: settings.radius,
            range: settings.range,
            position: settings.position.into(),
            flickering: (&settings.flickering).into(),
        }
    }
}

impl JetFireParams {
    pub fn point_light(&self) -> PointLight {
        PointLight {
            intensity: self.intensity,
            color: self.color,
            radius: self.radius,
            range: self.range,
            shadow_maps_enabled: true,
            ..Default::default()
        }
    }

    pub fn flickering_light(&self) -> FlickeringLight {
        FlickeringLight {
            base_intensity: self.intensity,
            variation: self.flickering.variation,
            timer: Timer::from_seconds(self.flickering.frequency, TimerMode::Repeating),
        }
    }

    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.position)
    }
}

pub fn setup_jet_fire(mut commands: Commands, config: Res<Config>, data: Res<GameData>) {
    if let Some(entity_id) = data.entities.first().cloned() {
        for (idx, jet_fire_config) in config.game.flying_model.jet_fires.iter().enumerate() {
            let params = JetFireParams::from(jet_fire_config);

            let jet_fire_entity_id = commands
                .spawn((
                    Name::new(format!("JetFire{idx}")),
                    params,
                    params.point_light(),
                    params.transform(),
                    params.flickering_light(),
                ))
                .id();
            commands.entity(entity_id).add_child(jet_fire_entity_id);
        }
    }
}

pub fn apply_jet_fire(
    mut query: Query<(&JetFireParams, &mut PointLight, &mut FlickeringLight, &mut Transform), Changed<JetFireParams>>,
) {
    for (params, mut light, mut flickering, mut transform) in &mut query {
        *light = params.point_light();
        *flickering = params.flickering_light();
        *transform = params.transform();
    }
}

pub fn flickering_light_system(time: Res<Time>, mut query: Query<(&mut PointLight, &mut FlickeringLight)>) {
    for (mut light, mut flicker) in &mut query {
        flicker.timer.tick(time.delta());
        if flicker.timer.is_finished() {
            // Псевдослучайный коэффициент [-1.0; 1.0]
            let rand: f32 = (fastrand::f32() - 0.5) * 2.0;
            light.intensity = flicker.base_intensity + rand * flicker.variation;
        }
    }
}
