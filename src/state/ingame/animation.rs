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
use bevy::scene::SceneInstanceReady;

use crate::config::Config;

#[derive(Resource)]
pub struct Animations {
    animations: Vec<AnimationNodeIndex>,
    graph: Handle<AnimationGraph>,
}

impl Animations {
    fn get(&self, kind: AnimationKind) -> AnimationNodeIndex {
        self.animations[kind as usize]
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[repr(usize)]
pub enum AnimationKind {
    LeftElevonExternDown = 0,
    LeftElevonExternUp,
    LeftRuddervatorTurnLeft,
    LeftRuddervatorTurnRight,
    Rest,
    RightElevonExternDown,
    RightElevonExternUp,
    RightRuddervatorTurnLeft,
    RightRuddervatorTurnRight,
    PitchDown,
    PitchUp,
}

pub fn setup_animation_graph(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut animations = Vec::new();
    let mut graph = AnimationGraph::new();

    let parent = graph.root;
    let weight = 1.0;
    for i in 0..11 {
        let animation_node = graph.add_clip(
            asset_server.load(GltfAssetLabel::Animation(i).from_asset(config.game.flying_model.path.clone())),
            weight,
            parent,
        );
        animations.push(animation_node);
    }

    let graph = graphs.add(graph);
    commands.insert_resource(Animations {
        animations,
        graph: graph.clone(),
    });
}

#[derive(Resource, Debug)]
pub struct AdditionalPlayers {
    pub ruddervator_left_player: Entity,
    pub ruddervator_right_player: Entity,
    pub elevon_extern_left_player: Entity,
    pub elevon_extern_right_player: Entity,
}

pub fn attach_animations(
    _trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    to_animated_entities: Query<(Entity, &AnimationPlayer)>,
    animation_targets: Query<(&Name, &mut AnimationTarget)>,
    animations: Res<Animations>,
    players: Res<AdditionalPlayers>,
) {
    for (entity, _player) in &to_animated_entities {
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animations.graph.clone()));
    }

    for (name, mut target) in animation_targets {
        let lowercased_name = name.as_str().trim().to_lowercase();
        if lowercased_name.starts_with("ruddervator") {
            let trimmed_name = lowercased_name
                .trim_start_matches("ruddervator")
                .trim_start_matches('_')
                .trim_start_matches('-')
                .trim_start();

            if trimmed_name.starts_with("left") {
                target.player = players.ruddervator_left_player;
            } else if trimmed_name.starts_with("right") {
                target.player = players.ruddervator_right_player;
            }
        } else if lowercased_name.starts_with("elevon") {
            let trimmed_name = lowercased_name
                .trim_start_matches("elevon")
                .trim_start_matches('_')
                .trim_start_matches('-')
                .trim_start()
                .trim_start_matches("extern")
                .trim_start_matches('_')
                .trim_start_matches('-')
                .trim_start();

            if trimmed_name.starts_with("left") {
                target.player = players.elevon_extern_left_player;
            } else if trimmed_name.starts_with("right") {
                target.player = players.elevon_extern_right_player;
            }
        }
    }
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
    pub left_elevon: State,
    pub right_elevon: State,
    pub left_ruddervator: State,
    pub right_ruddervator: State,
    pub pitch: State,
}

pub fn control(
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
    let to_roll_left_pressed = keyboard_input.pressed(KeyCode::ArrowLeft);
    let to_roll_right_pressed = keyboard_input.pressed(KeyCode::ArrowRight);

    if (to_roll_left_pressed && to_roll_right_pressed) || (!to_roll_left_pressed && !to_roll_right_pressed) {
        data.left_elevon.next = Direction::Origin;
        data.right_elevon.next = Direction::Origin;
    } else if to_roll_left_pressed {
        data.left_elevon.next = Direction::SideA;
        data.right_elevon.next = Direction::SideB;
    } else if to_roll_right_pressed {
        data.left_elevon.next = Direction::SideB;
        data.right_elevon.next = Direction::SideA;
    }

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

        if player_entity == players.elevon_extern_left_player {
            if data.left_elevon.need_update() {
                let up_animation_idx = animations.get(AnimationKind::LeftElevonExternUp);
                let down_animation_idx = animations.get(AnimationKind::LeftElevonExternDown);

                if let Some(new_state) = switch_rotate_animation(
                    &mut player,
                    &animation_graph,
                    &animation_clips,
                    up_animation_idx,
                    down_animation_idx,
                    data.left_elevon,
                ) {
                    data.left_elevon = new_state;
                }
            }
        } else if player_entity == players.elevon_extern_right_player {
            if data.right_elevon.need_update() {
                let up_animation_idx = animations.get(AnimationKind::RightElevonExternUp);
                let down_animation_idx = animations.get(AnimationKind::RightElevonExternDown);

                if let Some(new_state) = switch_rotate_animation(
                    &mut player,
                    &animation_graph,
                    &animation_clips,
                    up_animation_idx,
                    down_animation_idx,
                    data.right_elevon,
                ) {
                    data.right_elevon = new_state;
                }
            }
        } else if player_entity == players.ruddervator_left_player {
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
        } else if player_entity == players.ruddervator_right_player {
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
