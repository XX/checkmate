use bevy::animation::AnimationPlayer;
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
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
use crate::follow::{Followee, PreviousTransform};
use crate::state::ingame::aircraft::{Aircraft, Movement, Thrust};
use crate::state::ingame::animation::{AdditionalPlayers, attach_animations};
use crate::state::{SceneKey, Scenes};

pub mod aircraft;
pub mod animation;
pub mod engine;
pub mod terrain;

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
    mut camera_params: ResMut<AppCameraParams>,
) {
    let scene = scenes
        .game
        .entry(SceneKey::Aircraft)
        .or_insert_with(|| {
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(config.game.flying_model.path.clone()))
        })
        .clone();

    let altitude = config.game.flight_altitude;
    let transform = Transform::from_translation(Vec3::ZERO.with_y(altitude));
    let entity_id = commands
        .spawn((
            Aircraft::new(),
            Thrust::new(),
            Movement::default(),
            Followee,
            SceneRoot(scene),
            PreviousTransform(transform.clone()),
            transform,
        ))
        .id();

    let ruddervator_left_player = commands
        .spawn((AnimationPlayer::default(), Name::new("RuddervatorLeftPlayer")))
        .id();

    let ruddervator_right_player = commands
        .spawn((AnimationPlayer::default(), Name::new("RuddervatorRightPlayer")))
        .id();

    let elevon_extern_left_player = commands
        .spawn((AnimationPlayer::default(), Name::new("ElevonExternLeftPlayer")))
        .id();

    let elevon_extern_right_player = commands
        .spawn((AnimationPlayer::default(), Name::new("ElevonExternRightPlayer")))
        .id();

    commands.entity(entity_id).add_children(&[
        ruddervator_left_player,
        ruddervator_right_player,
        elevon_extern_left_player,
        elevon_extern_right_player,
    ]);

    commands.insert_resource(AdditionalPlayers {
        ruddervator_left_player,
        ruddervator_right_player,
        elevon_extern_left_player,
        elevon_extern_right_player,
    });

    commands.entity(entity_id).observe(attach_animations);

    commands.insert_resource(GameData {
        entities: vec![entity_id],
        ..Default::default()
    });

    camera_params.follower.followee = Some(entity_id);
    camera::respawn_panorbit(commands, camera_params, camera.entity_id, &config.camera, altitude);
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
