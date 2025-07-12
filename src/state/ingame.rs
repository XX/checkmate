use bevy::asset::AssetServer;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Commands, Res, ResMut};
use bevy::math::Vec3;
use bevy::scene::SceneRoot;
use bevy::transform::components::Transform;

use crate::camera::{self, AppCameraEntity, AppCameraParams};
use crate::config::Config;
use crate::state::Scenes;

pub const FLY_HEIGHT: f32 = 3000.0;

#[derive(Component)]
pub struct PlaneMovement {
    target_pos: Vec3,
    timer: f32,
}

#[derive(Resource)]
pub struct GameData {
    pub entities: Vec<Entity>,
}

pub fn setup(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    mut scenes: ResMut<Scenes>,
    camera: Res<AppCameraEntity>,
    camera_params: ResMut<AppCameraParams>,
) {
    let scene = scenes
        .game
        .get_or_insert_with(|| asset_server.load(format!("{}#Scene0", config.game.flying_model)))
        .clone();

    let entity_id = commands
        .spawn((
            PlaneMovement {
                target_pos: Vec3::ZERO,
                timer: 0.0,
            },
            SceneRoot(scene),
            Transform::from_translation(Vec3::ZERO.with_y(FLY_HEIGHT)),
        ))
        .id();

    commands.insert_resource(GameData {
        entities: vec![entity_id],
    });

    camera::respawn_panorbit(
        commands,
        camera_params.into(),
        camera.entity_id,
        Vec3::new(-3.0, FLY_HEIGHT + 5.0, 15.0),
        Vec3::ZERO.with_y(FLY_HEIGHT),
    );
}

pub fn cleanup(mut commands: Commands, data: Res<GameData>) {
    for entity in &data.entities {
        commands.entity(*entity).despawn();
    }

    commands.remove_resource::<GameData>();
}
