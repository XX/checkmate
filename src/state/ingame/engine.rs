use bevy::color::Color;
use bevy::ecs::component::Component;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::pbr::PointLight;
use bevy::time::{Time, Timer, TimerMode};
use bevy::transform::components::Transform;

use crate::config::Config;
use crate::state::ingame::GameData;

#[derive(Component)]
pub struct FlickeringLight {
    base_intensity: f32,
    variation: f32,
    timer: Timer,
}

pub fn setup_jet_fire(mut commands: Commands, config: Res<Config>, data: Res<GameData>) {
    if let Some(entity_id) = data.entities.first().cloned() {
        for jet_fire_config in &config.game.flying_model.jet_fires {
            let jet_fire_entity_id = commands
                .spawn((
                    PointLight {
                        intensity: jet_fire_config.intensity,
                        color: Color::srgb_from_array(jet_fire_config.color),
                        radius: jet_fire_config.radius,
                        range: jet_fire_config.range,
                        shadows_enabled: true,
                        ..Default::default()
                    },
                    Transform::from_translation(jet_fire_config.position.into()),
                    FlickeringLight {
                        base_intensity: jet_fire_config.intensity,
                        variation: jet_fire_config.flickering.variation,
                        timer: Timer::from_seconds(jet_fire_config.flickering.frequency, TimerMode::Repeating),
                    },
                ))
                .id();
            commands.entity(entity_id).add_child(jet_fire_entity_id);
        }
    }
}

pub fn flickering_light_system(time: Res<Time>, mut query: Query<(&mut PointLight, &mut FlickeringLight)>) {
    for (mut light, mut flicker) in &mut query {
        flicker.timer.tick(time.delta());
        if flicker.timer.finished() {
            // Псевдослучайный коэффициент [-1.0; 1.0]
            let rand: f32 = (fastrand::f32() - 0.5) * 2.0;
            light.intensity = flicker.base_intensity + rand * flicker.variation;
        }
    }
}
