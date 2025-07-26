use bevy::animation::graph::{AnimationGraph, AnimationGraphHandle, AnimationNodeIndex, AnimationNodeType};
use bevy::animation::{AnimationClip, AnimationPlayer, AnimationTarget};
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::observer::Trigger;
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Commands, Local, Query, Res, ResMut};
use bevy::gltf::GltfAssetLabel;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::math::Vec3;
use bevy::pbr::StandardMaterial;
use bevy::render::mesh::Mesh;
use bevy::scene::{SceneInstanceReady, SceneRoot};
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

    let first_player = commands
        .spawn((AnimationPlayer::default(), Name::new("FirstPlayer")))
        .id();

    let second_player = commands
        .spawn((AnimationPlayer::default(), Name::new("SecondPlayer")))
        .id();

    commands.insert_resource(AdditionalPlayers {
        first_player,
        second_player,
    });

    commands.entity(entity_id).add_child(first_player);
    commands.entity(entity_id).add_child(second_player);
    commands.entity(entity_id).observe(attach_animations);

    commands.insert_resource(GameData {
        entities: vec![entity_id, first_player, second_player],
        ..Default::default()
    });

    camera_params.follower.followee = Some(entity_id);
    camera::respawn_panorbit(commands, camera_params, camera.entity_id, &config.camera, altitude);
}

#[derive(Resource)]
pub struct AdditionalPlayers {
    pub first_player: Entity,
    pub second_player: Entity,
}

fn attach_animations(
    _trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    to_animated_entities: Query<(Entity, &AnimationPlayer)>,
    animation_targets: Query<(&Name, &mut AnimationTarget)>,
    animations: Res<Animations>,
    players: Res<AdditionalPlayers>,
) {
    for (entity, _player) in &to_animated_entities {
        println!("Animated: {entity}");
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animations.graph.clone()));
    }

    for (name, mut target) in animation_targets {
        match name.as_str() {
            "ruddervator_left" | "Bone.001" => target.player = players.first_player,
            "ruddervator_right" | "Bone.002" => target.player = players.second_player,
            _ => (),
        }
        println!("Name: {}, player: {}", name.as_str(), target.player);
    }
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
pub enum Direction {
    SideA,
    SideB,
    #[default]
    Origin,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub current: Direction,
    pub next: Direction,
}

impl State {
    pub fn new(current: Direction, next: Direction) -> Self {
        Self { current, next }
    }

    pub fn need_update(&self) -> bool {
        self.current != self.next
    }

    pub fn to_next(&self) -> Self {
        Self {
            current: self.next,
            next: self.next,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AnimationData {
    pub left_ruddervator: State,
    pub right_ruddervator: State,
    pub pitch: State,
}

pub fn control_animations(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animation_players: Query<(Entity, &mut AnimationPlayer)>,
    players: Res<AdditionalPlayers>,
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
    let to_up_pressed = keyboard_input.pressed(KeyCode::ArrowDown);
    let to_down_pressed = keyboard_input.pressed(KeyCode::ArrowUp);

    if (to_up_pressed && to_down_pressed) || (!to_up_pressed && !to_down_pressed) {
        data.pitch.next = Direction::Origin;
    } else if to_up_pressed {
        data.pitch.next = Direction::SideA;
    } else if to_down_pressed {
        data.pitch.next = Direction::SideB;
    }

    let to_origin = (to_left_pressed && to_right_pressed) || (!to_left_pressed && !to_right_pressed);

    match data.pitch.next {
        Direction::SideA => {
            if to_origin {
                data.left_ruddervator.next = Direction::SideA;
                data.right_ruddervator.next = Direction::SideB;
            } else if to_left_pressed {
                data.left_ruddervator.next = Direction::Origin;
                data.right_ruddervator.next = Direction::SideB;
            } else if to_right_pressed {
                data.left_ruddervator.next = Direction::SideA;
                data.right_ruddervator.next = Direction::Origin;
            }
        },
        Direction::SideB => {
            if to_origin {
                data.left_ruddervator.next = Direction::SideB;
                data.right_ruddervator.next = Direction::SideA;
            } else if to_left_pressed {
                data.left_ruddervator.next = Direction::SideB;
                data.right_ruddervator.next = Direction::Origin;
            } else if to_right_pressed {
                data.left_ruddervator.next = Direction::Origin;
                data.right_ruddervator.next = Direction::SideA;
            }
        },
        Direction::Origin => {
            if to_origin {
                data.left_ruddervator.next = Direction::Origin;
                data.right_ruddervator.next = Direction::Origin;
            } else if to_left_pressed {
                data.left_ruddervator.next = Direction::SideB;
                data.right_ruddervator.next = Direction::SideB;
            } else if to_right_pressed {
                data.left_ruddervator.next = Direction::SideA;
                data.right_ruddervator.next = Direction::SideA;
            }
        },
    }

    let mut players_iter = animation_players.iter_mut();
    for (player_entity, mut player) in &mut players_iter {
        if player.all_finished() {
            player.stop_all();
        }

        if player_entity == players.first_player {
            if data.left_ruddervator.need_update() {
                let left_animation_idx = animations.get(AnimationKind::LeftRuddervatorTurnLeft);
                let right_animation_idx = animations.get(AnimationKind::LeftRuddervatorTurnRight);

                if let Some(new_state) = switch_rotate_animation(
                    &mut player,
                    &animation_graph,
                    &animation_clips,
                    left_animation_idx,
                    right_animation_idx,
                    data.left_ruddervator,
                ) {
                    data.left_ruddervator = new_state;
                }
            }
        } else if player_entity == players.second_player {
            if data.right_ruddervator.need_update() {
                let left_animation_idx = animations.get(AnimationKind::RightRuddervatorTurnLeft);
                let right_animation_idx = animations.get(AnimationKind::RightRuddervatorTurnRight);

                if let Some(new_state) = switch_rotate_animation(
                    &mut player,
                    &animation_graph,
                    &animation_clips,
                    left_animation_idx,
                    right_animation_idx,
                    data.right_ruddervator,
                ) {
                    data.right_ruddervator = new_state;
                }
            }
        } else {
            if data.pitch.need_update() {
                let up_animation_idx = animations.animations[AnimationKind::PitchUp as usize];
                let down_animation_idx = animations.animations[AnimationKind::PitchDown as usize];

                if let Some(new_state) = switch_rotate_animation(
                    &mut player,
                    &animation_graph,
                    &animation_clips,
                    up_animation_idx,
                    down_animation_idx,
                    data.pitch,
                ) {
                    data.pitch = new_state;
                }
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
    state: State,
) -> Option<State> {
    match (state.current, state.next) {
        (Direction::Origin, Direction::SideA) if !player.is_playing_animation(side_b_animation_idx) => {
            let animation = player.play(side_a_animation_idx);
            if animation.is_playback_reversed() {
                animation.set_speed(-1.0 * animation.speed());
            }
            Some(state.to_next())
        },
        (Direction::Origin, Direction::SideB) if !player.is_playing_animation(side_a_animation_idx) => {
            let animation = player.play(side_b_animation_idx);
            if animation.is_playback_reversed() {
                animation.set_speed(-1.0 * animation.speed());
            }

            Some(state.to_next())
        },
        (Direction::SideA, Direction::Origin | Direction::SideB) => {
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

            Some(State::new(Direction::Origin, state.next))
        },
        (Direction::SideB, Direction::Origin | Direction::SideA) => {
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

            Some(State::new(Direction::Origin, state.next))
        },
        _ => None,
    }
}
