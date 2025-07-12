use bevy::asset::{AssetServer, Assets, Handle};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Commands, Res, ResMut};
use bevy::gltf::GltfAssetLabel;
use bevy::math::Vec3;
use bevy::pbr::StandardMaterial;
use bevy::render::mesh::Mesh;
use bevy::scene::SceneRoot;
use bevy::transform::components::Transform;

use crate::camera::{self, AppCameraEntity, AppCameraParams};
use crate::config::Config;
use crate::state::{SceneKey, Scenes};

pub mod terrain;

#[derive(Component)]
pub struct PlaneMovement {
    target_pos: Vec3,
    timer: f32,
}

#[derive(Default, Resource)]
pub struct GameData {
    pub entities: Vec<Entity>,
    pub meshes: Vec<Handle<Mesh>>,
    pub materials: Vec<Handle<StandardMaterial>>,
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
        .entry(SceneKey::Aircraft)
        .or_insert_with(|| asset_server.load(GltfAssetLabel::Scene(0).from_asset(config.game.flying_model.clone())))
        .clone();

    let altitude = config.game.flight_altitude;
    let entity_id = commands
        .spawn((
            PlaneMovement {
                target_pos: Vec3::ZERO,
                timer: 0.0,
            },
            SceneRoot(scene),
            Transform::from_translation(Vec3::ZERO.with_y(altitude)),
        ))
        .id();

    commands.insert_resource(GameData {
        entities: vec![entity_id],
        ..Default::default()
    });

    camera::respawn_panorbit(
        commands,
        camera_params.into(),
        camera.entity_id,
        Vec3::new(-3.0, altitude + 5.0, 15.0),
        Vec3::ZERO.with_y(altitude),
    );
}

pub fn cleanup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    data: Res<GameData>,
) {
    for entity in &data.entities {
        commands.entity(*entity).despawn();
    }

    for mesh in &data.meshes {
        meshes.remove(mesh);
    }

    for material in &data.materials {
        materials.remove(material);
    }

    commands.remove_resource::<GameData>();
}
