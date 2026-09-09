use bevy::color::Color;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::query::Changed;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::light::{PointLight, SpotLight};
use bevy::math::Vec3;
use bevy::reflect::Reflect;
use bevy::time::{Time, Timer, TimerMode};
use bevy::transform::components::Transform;

use crate::config::{Config, FlickeringSettings, JetFireKind, JetFireSettings, color_from_array, color_to_array};
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
/// Источник истины для источника света, [`FlickeringLight`] и [`Transform`] сущности пламени:
/// изменение полей применяется системой [`apply_jet_fire`]. Смена [`Self::kind`] меняет и сам
/// тип компонента света — [`SpotLight`] или [`PointLight`].
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct JetFireParams {
    pub intensity: f32,

    pub color: Color,

    pub radius: f32,

    pub range: f32,

    pub position: Vec3,

    pub kind: JetFireKind,

    pub outer_angle: f32,

    pub inner_angle: f32,

    pub shadows_enabled: bool,

    pub flickering: FlickeringParams,
}

impl From<&JetFireSettings> for JetFireParams {
    fn from(settings: &JetFireSettings) -> Self {
        Self {
            intensity: settings.intensity,
            color: color_from_array(settings.color),
            radius: settings.radius,
            range: settings.range,
            position: settings.position.into(),
            kind: settings.kind,
            outer_angle: settings.outer_angle,
            inner_angle: settings.inner_angle,
            shadows_enabled: settings.shadows_enabled,
            flickering: (&settings.flickering).into(),
        }
    }
}

impl From<&FlickeringParams> for FlickeringSettings {
    fn from(params: &FlickeringParams) -> Self {
        Self {
            variation: params.variation,
            frequency: params.frequency,
        }
    }
}

impl From<&JetFireParams> for JetFireSettings {
    fn from(params: &JetFireParams) -> Self {
        Self {
            intensity: params.intensity,
            color: color_to_array(params.color),
            radius: params.radius,
            range: params.range,
            position: params.position.into(),
            kind: params.kind,
            outer_angle: params.outer_angle,
            inner_angle: params.inner_angle,
            shadows_enabled: params.shadows_enabled,
            flickering: (&params.flickering).into(),
        }
    }
}

impl JetFireParams {
    /// Направленный источник света пламени: конус вдоль оси сопла, назад.
    ///
    /// В bevy прожектор светит вдоль локальной оси −Z, а самолёт летит в +Z, поэтому конус
    /// смотрит назад без дополнительного поворота — см. [`Self::transform`].
    pub fn spot_light(&self) -> SpotLight {
        SpotLight {
            intensity: self.intensity,
            color: self.color,
            radius: self.radius,
            range: self.range,
            outer_angle: self.outer_angle,
            inner_angle: self.inner_angle,
            shadow_maps_enabled: self.shadows_enabled,
            ..Default::default()
        }
    }

    /// Всенаправленный источник света пламени.
    pub fn point_light(&self) -> PointLight {
        PointLight {
            intensity: self.intensity,
            color: self.color,
            radius: self.radius,
            range: self.range,
            shadow_maps_enabled: self.shadows_enabled,
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

            let mut jet_fire_entity = commands.spawn((
                Name::new(format!("JetFire{idx}")),
                params,
                params.transform(),
                params.flickering_light(),
            ));

            match params.kind {
                JetFireKind::Spot => jet_fire_entity.insert(params.spot_light()),
                JetFireKind::Point => jet_fire_entity.insert(params.point_light()),
            };

            let jet_fire_entity_id = jet_fire_entity.id();
            commands.entity(entity_id).add_child(jet_fire_entity_id);
        }
    }
}

/// Переносит параметры в компоненты сущности пламени.
///
/// Тип источника — структурный параметр: его смена меняет набор компонентов, поэтому идёт
/// через [`Commands`], а не мутацией на месте.
pub fn apply_jet_fire(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &JetFireParams,
            Option<&mut SpotLight>,
            Option<&mut PointLight>,
            &mut FlickeringLight,
            &mut Transform,
        ),
        Changed<JetFireParams>,
    >,
) {
    for (entity, params, spot, point, mut flickering, mut transform) in &mut query {
        *flickering = params.flickering_light();
        *transform = params.transform();

        match params.kind {
            JetFireKind::Spot => {
                if let Some(mut light) = spot {
                    *light = params.spot_light();
                } else {
                    commands
                        .entity(entity)
                        .remove::<PointLight>()
                        .insert(params.spot_light());
                }
            },
            JetFireKind::Point => {
                if let Some(mut light) = point {
                    *light = params.point_light();
                } else {
                    commands
                        .entity(entity)
                        .remove::<SpotLight>()
                        .insert(params.point_light());
                }
            },
        }
    }
}

pub fn flickering_light_system(
    time: Res<Time>,
    mut query: Query<(Option<&mut SpotLight>, Option<&mut PointLight>, &mut FlickeringLight)>,
) {
    for (spot, point, mut flicker) in &mut query {
        flicker.timer.tick(time.delta());
        if !flicker.timer.is_finished() {
            continue;
        }

        // Псевдослучайный коэффициент [-1.0; 1.0]
        let rand: f32 = (fastrand::f32() - 0.5) * 2.0;
        let intensity = flicker.base_intensity + rand * flicker.variation;

        if let Some(mut light) = spot {
            light.intensity = intensity;
        }
        if let Some(mut light) = point {
            light.intensity = intensity;
        }
    }
}
