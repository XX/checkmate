use bevy::DefaultPlugins;
use bevy::animation::graph::{AnimationGraphHandle, AnimationNodeType};
use bevy::animation::{AnimationClip, AnimationPlayer, animate_targets};
use bevy::app::{App, Startup, Update};
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::color::{Color, ColorToComponents, LinearRgba};
use bevy::ecs::component::Component;
use bevy::ecs::query::Added;
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Local, Query, Res, ResMut};
use bevy::gltf::GltfAssetLabel;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::math::primitives::Plane3d;
use bevy::math::{Dir3, Vec3};
use bevy::pbr::{
    Atmosphere, AtmosphereSettings, DirectionalLight, DirectionalLightShadowMap, MeshMaterial3d, StandardMaterial,
};
use bevy::prelude::{AnimationGraph, AnimationNodeIndex, Entity, MeshBuilder, default};
use bevy::reflect::Reflect;
use bevy::render::camera::Exposure;
use bevy::render::mesh::{Mesh, Mesh3d, Meshable};
use bevy::scene::{Scene, SceneRoot};
use bevy::state::app::AppExtStates;
use bevy::state::condition::in_state;
use bevy::state::state::{NextState, OnEnter, OnExit, State, States};
use bevy::transform::components::Transform;
use bevy::window::Window;
use clap::Parser;
use diagnostics::DiagnosticsPlugin;
use serde::{Deserialize, Serialize};
use utils::combine_meshes;

use crate::camera::{AppCameraEntity, AppCameraParams, AppCameraPlugin, LookingAt};
use crate::config::Config;

mod camera;
mod cli;
mod config;
mod diagnostics;
mod utils;

pub const LANDSCAPE_SIZE: f32 = 1200.0;
pub const LANDSCAPE_SIZE_HALF: f32 = LANDSCAPE_SIZE * 0.5;
pub const FLY_HEIGHT: f32 = 3000.0;

#[derive(Resource, Reflect)]
pub struct PlaneSettings {
    wobble_speed: f32,
    rotation_speed: f32,
    move_interval: f32,
    box_area: f32,
    speed: f32,
}

#[derive(Component)]
pub struct PlaneMovement {
    target_pos: Vec3,
    timer: f32,
}

#[derive(Resource)]
struct Animations {
    animations: Vec<AnimationNodeIndex>,
    graph: Handle<AnimationGraph>,
}

fn main() {
    let opts: cli::Opts = cli::Opts::parse();
    let config = Config::load(opts.config).unwrap_or_else(|err| {
        eprintln!("WARNING: config load error: {err}, use default config");
        Config::default()
    });

    let camera_params = AppCameraParams::default()
        .with_smoothness_speed(8.0)
        .with_custom_clear_color(Color::srgb(0.7, 0.92, 0.96))
        .width_translate(Vec3::new(-3.0, 5.0, 15.0))
        .width_look_at(LookingAt {
            target: Vec3::ZERO.with_y(2.31),
            up: Dir3::Y,
        })
        .with_tonemapping(config.camera.tonemap);

    let camera_params = if config.environment.atmosphere.enabled {
        camera_params
            .with_exposure(Exposure {
                ev100: config.camera.exposure,
            })
            .with_atmosphere((Atmosphere::EARTH, AtmosphereSettings {
                // aerial_view_lut_max_distance: 3.2e5,
                scene_units_to_m: 1.0, //1e+4,
                ..Default::default()
            }))
    } else {
        camera_params
    };

    let camera_params = if let Some(auto_exposure) = config.camera.auto_exposure.to_auto_exposure() {
        camera_params.with_auto_exposure(auto_exposure)
    } else {
        camera_params
    };

    let mut app = App::new();

    if let Some(ambient_light) = config.environment.ambient.to_ambient_light() {
        app.insert_resource(ambient_light);
    }

    app.insert_resource(camera_params)
        .insert_resource(DirectionalLightShadowMap {
            size: config.graphics.shadow_map_size,
        })
        .insert_resource(config)
        .insert_resource(Scenes::default())
        .add_plugins(DefaultPlugins)
        .add_plugins(DiagnosticsPlugin)
        .add_plugins(AppCameraPlugin)
        .init_state::<AppState>()
        .add_systems(Startup, setup)
        .add_systems(
            OnEnter(AppState::Hangar),
            (setup_hangar, chessboard_land_spawn.after(setup_hangar)),
        )
        .add_systems(OnExit(AppState::Hangar), cleanup_hangar)
        .add_systems(OnExit(AppState::InGame), cleanup_game)
        .add_systems(OnEnter(AppState::InGame), setup_game)
        .add_systems(Update, change_state)
        .add_systems(Update, attach_animations.before(animate_targets))
        .add_systems(Update, control_land_gear_animation.run_if(in_state(AppState::Hangar)))
        .add_systems(Update, close_on_esc)
        .run();
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Serialize, Deserialize)]
enum AppState {
    #[default]
    Loading,
    Hangar,
    InGame,
}

#[derive(Default, Resource)]
pub struct Scenes {
    pub hangar: Option<Handle<Scene>>,
    pub game: Option<Handle<Scene>>,
}

#[derive(Resource)]
pub struct HangarData {
    pub entities: Vec<Entity>,
    pub meshes: Vec<Handle<Mesh>>,
    pub materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Resource)]
pub struct GameData {
    pub entities: Vec<Entity>,
}

fn setup(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    commands.insert_resource(PlaneSettings {
        move_interval: 1.3,
        box_area: 6.0,
        speed: 1.5,
        wobble_speed: 5.0,
        rotation_speed: 0.7,
    });

    commands.spawn((
        DirectionalLight {
            shadows_enabled: config.environment.light.shadows_enabled,
            illuminance: config.environment.light.illuminance,
            ..default()
        },
        Transform::from_translation(config.environment.light.position.into())
            .looking_at(config.environment.light.target.into(), Vec3::Y),
    ));

    // Build the animation graph
    let mut graph = AnimationGraph::new();
    let animations = graph
        .add_clips(
            [GltfAssetLabel::Animation(0).from_asset(config.game.hangar_model.clone())]
                .into_iter()
                .map(|path| asset_server.load(path)),
            1.0,
            graph.root,
        )
        .collect();

    // Insert a resource with the current scene information
    let graph = graphs.add(graph);
    commands.insert_resource(Animations {
        animations,
        graph: graph.clone(),
    });

    next_state.set(config.game.state);
}

fn setup_hangar(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    mut scenes: ResMut<Scenes>,
    camera: Res<AppCameraEntity>,
    camera_params: ResMut<AppCameraParams>,
) {
    let scene = scenes
        .hangar
        .get_or_insert_with(|| asset_server.load(format!("{}#Scene0", config.game.hangar_model)))
        .clone();

    let entity_id = commands
        .spawn((SceneRoot(scene), Transform::from_translation(Vec3::ZERO.with_y(2.31))))
        .id();

    commands.insert_resource(HangarData {
        entities: vec![entity_id],
        meshes: vec![],
        materials: vec![],
    });

    reinit_camera(commands, camera_params.into(), camera.entity_id, 2.31);
}

fn cleanup_hangar(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    data: Res<HangarData>,
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

    commands.remove_resource::<HangarData>();
}

fn setup_game(
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

    reinit_camera(commands, camera_params.into(), camera.entity_id, FLY_HEIGHT);
}

fn cleanup_game(mut commands: Commands, data: Res<GameData>) {
    for entity in &data.entities {
        commands.entity(*entity).despawn();
    }

    commands.remove_resource::<GameData>();
}

fn chessboard_land_spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut data: ResMut<HangarData>,
) {
    let mut mesh_data = Vec::new();
    let cell_mesh = Plane3d::default().mesh().size(2.0, 2.0).build();

    for x in -7..8 {
        for z in -7..250 {
            let transform = Transform::from_xyz(x as f32 * 2.0, 0.0, z as f32 * 2.0);

            let mut mesh = cell_mesh.clone();
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![
                if (x + z) % 2 == 0 {
                    Color::LinearRgba(LinearRgba::RED)
                } else {
                    Color::WHITE
                }
                .to_linear()
                .to_f32_array();
                mesh.count_vertices()
            ]);
            mesh_data.push((mesh, transform));
        }
    }

    let mesh = meshes.add(combine_meshes(&mesh_data, true, false, false, true));
    let material = materials.add(Color::WHITE);

    let entity_id = commands
        .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())))
        .id();

    data.entities.push(entity_id);
    data.meshes.push(mesh);
    data.materials.push(material);
}

/// Attaches the animation graph to the scene
fn attach_animations(
    mut commands: Commands,
    to_animated_entities: Query<(Entity, &AnimationPlayer), Added<AnimationPlayer>>,
    animations: Res<Animations>,
) {
    for (entity, _player) in &to_animated_entities {
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animations.graph.clone()));
    }
}

fn control_land_gear_animation(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animation_players: Query<&mut AnimationPlayer>,
    animations: Res<Animations>,
    animation_clips: Res<Assets<AnimationClip>>,
    animation_graphs: Res<Assets<AnimationGraph>>,
    mut reverse: Local<bool>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyG) {
        let Some(animation_graph) = animation_graphs.get(&animations.graph) else {
            return;
        };

        for (node_index, mut player) in [animations.animations[0]].into_iter().zip(&mut animation_players) {
            let animation_node = &animation_graph[node_index];
            let animation_start_time = if *reverse {
                if let AnimationNodeType::Clip(clip_handle) = &animation_node.node_type {
                    animation_clips
                        .get(clip_handle)
                        .map(|clip| clip.duration())
                        .unwrap_or_default()
                } else {
                    0.0
                }
            } else {
                0.0
            };

            if player.all_finished() {
                for (_, playing_animation) in player.playing_animations_mut() {
                    playing_animation.replay();
                }
                player.seek_all_by(animation_start_time);
            }
            player.adjust_speeds(-1.0);
            player.play(node_index);
        }
        *reverse = !*reverse;
    }
}

fn change_state(
    input: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input.just_pressed(KeyCode::Tab) {
        match state.get() {
            AppState::Loading => {},
            AppState::Hangar => {
                next_state.set(AppState::InGame);
            },
            AppState::InGame => {
                next_state.set(AppState::Hangar);
            },
        }
    }
}

pub fn close_on_esc(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if !focus.focused {
            continue;
        }

        if input.just_pressed(KeyCode::Escape) {
            commands.entity(window).despawn();
        }
    }
}

fn reinit_camera(mut commands: Commands, mut params: ResMut<AppCameraParams>, camera: Entity, height: f32) {
    commands.entity(camera).despawn();

    params.translate = Vec3::new(-3.0, height + 5.0, 15.0);
    params.look_at.target = Vec3::ZERO.with_y(height);

    camera::spawn_panorbit(commands, params.into());
}
