use bevy::animation::graph::{AnimationGraph, AnimationNodeType};
use bevy::animation::{AnimationClip, AnimationPlayer};
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::ecs::entity::Entity;
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Commands, Local, Query, Res, ResMut};
use bevy::gltf::GltfAssetLabel;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::math::Vec3;
use bevy::pbr::StandardMaterial;
use bevy::render::mesh::Mesh;
use bevy::scene::SceneRoot;
use bevy::transform::components::Transform;

use crate::Animations;
use crate::camera::{self, AppCameraEntity, AppCameraParams};
use crate::config::Config;
use crate::follow::{Followee, PreviousTransform};
use crate::state::ingame::aircraft::{Aircraft, Movement, Thrust};
use crate::state::{SceneKey, Scenes};

pub mod aircraft;
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
        .or_insert_with(|| asset_server.load(GltfAssetLabel::Scene(0).from_asset(config.game.flying_model.clone())))
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

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[repr(usize)]
pub enum AnimationKind {
    Gears = 0,
    YawLeft,
    YawRight,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum YawState {
    Left,
    Right,
    #[default]
    Center,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AnimationData {
    pub current_yaw: YawState,
    pub next_yaw: YawState,
}

pub fn control_animations(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animation_players: Query<&mut AnimationPlayer>,
    animations: Res<Animations>,
    animation_clips: Res<Assets<AnimationClip>>,
    animation_graphs: Res<Assets<AnimationGraph>>,
    mut data: Local<AnimationData>,
) {
    let Some(animation_graph) = animation_graphs.get(&animations.graph) else {
        return;
    };

    let to_left_pressed = keyboard_input.pressed(KeyCode::KeyA);
    let to_right_pressed = keyboard_input.pressed(KeyCode::KeyD);

    if (to_left_pressed && to_right_pressed) || (!to_left_pressed && !to_right_pressed) {
        data.next_yaw = YawState::Center;
    } else if to_left_pressed {
        data.next_yaw = YawState::Left;
    } else if to_right_pressed {
        data.next_yaw = YawState::Right;
    }

    if let Some(mut player) = animation_players.iter_mut().next() {
        if player.all_finished() {
            player.stop_all();
        }

        if data.current_yaw != data.next_yaw {
            let left_animation_node = animations.animations[AnimationKind::YawLeft as usize];
            let right_animation_node = animations.animations[AnimationKind::YawRight as usize];

            match (data.current_yaw, data.next_yaw) {
                (YawState::Center, YawState::Left) => {
                    if !player.is_playing_animation(right_animation_node) {
                        player.play(left_animation_node);
                        data.current_yaw = YawState::Left;
                    }
                },
                (YawState::Center, YawState::Right) => {
                    if !player.is_playing_animation(left_animation_node) {
                        player.play(right_animation_node);
                        data.current_yaw = YawState::Right;
                    }
                },
                (YawState::Left, YawState::Center | YawState::Right) => {
                    player.play(left_animation_node);
                    data.current_yaw = YawState::Center;

                    let animation_node = &animation_graph[left_animation_node];
                    let animation_start_time = if let AnimationNodeType::Clip(clip_handle) = &animation_node.node_type {
                        animation_clips
                            .get(clip_handle)
                            .map(|clip| clip.duration())
                            .unwrap_or_default()
                    } else {
                        0.0
                    };
                    player.adjust_speeds(-1.0);
                    player.seek_all_by(animation_start_time);
                },
                (YawState::Right, YawState::Center | YawState::Left) => {
                    player.play(right_animation_node);
                    data.current_yaw = YawState::Center;

                    let animation_node = &animation_graph[right_animation_node];
                    let animation_start_time = if let AnimationNodeType::Clip(clip_handle) = &animation_node.node_type {
                        animation_clips
                            .get(clip_handle)
                            .map(|clip| clip.duration())
                            .unwrap_or_default()
                    } else {
                        0.0
                    };
                    player.adjust_speeds(-1.0);
                    player.seek_all_by(animation_start_time);
                },
                _ => (),
            }
        }
    }
}
