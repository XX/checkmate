use bevy::animation::graph::{AnimationGraph, AnimationNodeIndex, AnimationNodeType};
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

use crate::camera::{self, AppCameraEntity, AppCameraParams};
use crate::config::Config;
use crate::follow::{Followee, PreviousTransform};
use crate::state::ingame::aircraft::{Aircraft, Movement, Thrust};
use crate::state::{SceneKey, Scenes};
use crate::{AnimationKind, Animations};

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RotateState {
    SideA,
    SideB,
    #[default]
    Origin,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AnimationData {
    pub current_yaw: RotateState,
    pub next_yaw: RotateState,
    pub current_pitch: RotateState,
    pub next_pitch: RotateState,
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
    let to_up_pressed = keyboard_input.pressed(KeyCode::ArrowUp);
    let to_down_pressed = keyboard_input.pressed(KeyCode::ArrowDown);

    if (to_left_pressed && to_right_pressed) || (!to_left_pressed && !to_right_pressed) {
        data.next_yaw = RotateState::Origin;
    } else if to_left_pressed {
        data.next_yaw = RotateState::SideA;
    } else if to_right_pressed {
        data.next_yaw = RotateState::SideB;
    }

    if (to_up_pressed && to_down_pressed) || (!to_up_pressed && !to_down_pressed) {
        data.next_pitch = RotateState::Origin;
    } else if to_up_pressed {
        data.next_pitch = RotateState::SideA;
    } else if to_down_pressed {
        data.next_pitch = RotateState::SideB;
    }

    if let Some(mut player) = animation_players.iter_mut().next() {
        if player.all_finished() {
            player.stop_all();
        }

        if data.current_yaw != data.next_yaw {
            let left_animation_idx = animations.animations[AnimationKind::YawLeft as usize];
            let right_animation_idx = animations.animations[AnimationKind::YawRight as usize];

            if let Some(new_state) = switch_rotate_animation(
                &mut player,
                &animation_graph,
                &animation_clips,
                left_animation_idx,
                right_animation_idx,
                data.current_yaw,
                data.next_yaw,
            ) {
                data.current_yaw = new_state;
            }
        }

        if data.current_pitch != data.next_pitch {
            let up_animation_idx = animations.animations[AnimationKind::PitchUp as usize];
            let down_animation_idx = animations.animations[AnimationKind::PitchDown as usize];

            if let Some(new_state) = switch_rotate_animation(
                &mut player,
                &animation_graph,
                &animation_clips,
                up_animation_idx,
                down_animation_idx,
                data.current_pitch,
                data.next_pitch,
            ) {
                data.current_pitch = new_state;
            }
        }
    }
}

fn switch_rotate_animation(
    player: &mut AnimationPlayer,
    animation_graph: &AnimationGraph,
    animation_clips: &Assets<AnimationClip>,
    side_a_animation_idx: AnimationNodeIndex,
    side_b_animation_idx: AnimationNodeIndex,
    current_state: RotateState,
    next_state: RotateState,
) -> Option<RotateState> {
    match (current_state, next_state) {
        (RotateState::Origin, RotateState::SideA) if !player.is_playing_animation(side_b_animation_idx) => {
            let animation = player.play(side_a_animation_idx);
            if animation.is_playback_reversed() {
                animation.set_speed(-1.0 * animation.speed());
            }
            Some(RotateState::SideA)
        },
        (RotateState::Origin, RotateState::SideB) if !player.is_playing_animation(side_a_animation_idx) => {
            let animation = player.play(side_b_animation_idx);
            if animation.is_playback_reversed() {
                animation.set_speed(-1.0 * animation.speed());
            }

            Some(RotateState::SideB)
        },
        (RotateState::SideA, RotateState::Origin | RotateState::SideB) => {
            let animation = player.play(side_a_animation_idx);
            if !animation.is_playback_reversed() {
                let animation_node = &animation_graph[side_a_animation_idx];
                let animation_start_time = if let AnimationNodeType::Clip(clip_handle) = &animation_node.node_type {
                    animation_clips
                        .get(clip_handle)
                        .map(|clip| clip.duration())
                        .unwrap_or_default()
                } else {
                    0.0
                };

                animation.set_speed(-1.0 * animation.speed());
                animation.seek_to(animation.seek_time() + animation_start_time);
            }

            Some(RotateState::Origin)
        },
        (RotateState::SideB, RotateState::Origin | RotateState::SideA) => {
            let animation = player.play(side_b_animation_idx);
            if !animation.is_playback_reversed() {
                let animation_node = &animation_graph[side_b_animation_idx];
                let animation_start_time = if let AnimationNodeType::Clip(clip_handle) = &animation_node.node_type {
                    animation_clips
                        .get(clip_handle)
                        .map(|clip| clip.duration())
                        .unwrap_or_default()
                } else {
                    0.0
                };

                animation.set_speed(-1.0 * animation.speed());
                animation.seek_to(animation.seek_time() + animation_start_time);
            }

            Some(RotateState::Origin)
        },
        _ => None,
    }
}
