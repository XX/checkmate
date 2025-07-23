use bevy::DefaultPlugins;
use bevy::animation::graph::AnimationGraphHandle;
use bevy::animation::{AnimationPlayer, animate_targets};
use bevy::app::{App, Startup, Update};
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::color::Color;
use bevy::ecs::component::Component;
use bevy::ecs::query::Added;
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::gltf::GltfAssetLabel;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::math::Vec3;
use bevy::pbr::{Atmosphere, AtmosphereSettings, DirectionalLight, DirectionalLightShadowMap};
use bevy::prelude::{AnimationGraph, AnimationNodeIndex, Entity, default};
use bevy::reflect::Reflect;
use bevy::render::camera::{ClearColorConfig, Exposure};
use bevy::state::app::AppExtStates;
use bevy::state::condition::in_state;
use bevy::state::state::{NextState, OnEnter, OnExit};
use bevy::transform::components::Transform;
use bevy::window::Window;
use bevy_obj::ObjPlugin;
use clap::Parser;

use crate::camera::{AppCameraParams, AppCameraPlugin};
use crate::config::Config;
use crate::diagnostics::DiagnosticsPlugin;
use crate::state::{AppState, Scenes, hangar, ingame};

mod camera;
mod cli;
mod config;
mod diagnostics;
mod follow;
mod state;
mod utils;

#[derive(Resource, Reflect)]
pub struct PlaneSettings {
    wobble_speed: f32,
    rotation_speed: f32,
    move_interval: f32,
    box_area: f32,
    speed: f32,
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
        .with_tonemapping(config.camera.tonemap)
        .with_follower(config.camera.follow.to_follower());

    let camera_params = if config.environment.atmosphere.enabled {
        camera_params
            .with_clear_color_config(ClearColorConfig::Default)
            .with_exposure(Exposure {
                ev100: config.camera.exposure,
            })
            .with_atmosphere((Atmosphere::EARTH, AtmosphereSettings {
                // aerial_view_lut_max_distance: 3.2e5,
                scene_units_to_m: 1.0, //1e+4,
                ..Default::default()
            }))
    } else {
        camera_params.with_custom_clear_color(Color::srgb(0.7, 0.92, 0.96))
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
        .add_plugins((DefaultPlugins, ObjPlugin, DiagnosticsPlugin, AppCameraPlugin))
        .init_state::<AppState>()
        .add_systems(Startup, setup)
        .add_systems(
            OnEnter(AppState::Hangar),
            (hangar::setup, hangar::chessboard_land_spawn.after(hangar::setup)),
        )
        .add_systems(OnExit(AppState::Hangar), hangar::cleanup)
        .add_systems(
            OnEnter(AppState::InGame),
            (ingame::setup, ingame::terrain::setup.after(ingame::setup)),
        )
        .add_systems(
            Update,
            (
                ingame::aircraft::update_thrust,
                ingame::aircraft::movement,
                ingame::aircraft::rotation,
                ingame::control_animations,
                camera::follow_toggle,
                camera::follow_move,
                follow::update_previous_transform,
                camera::preset_toggle,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(OnExit(AppState::InGame), ingame::cleanup)
        .add_systems(Update, state::change)
        .add_systems(Update, attach_animations.before(animate_targets))
        .add_systems(
            Update,
            hangar::control_land_gear_animation.run_if(in_state(AppState::Hangar)),
        )
        .add_systems(Update, close_on_esc)
        .run();
}

#[derive(Component)]
struct Sun;

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
        Sun,
        DirectionalLight {
            shadows_enabled: config.environment.sun.shadows_enabled,
            illuminance: config.environment.sun.illuminance,
            ..default()
        },
        Transform::from_translation(config.environment.sun.position.into())
            .looking_at(config.environment.sun.target.into(), Vec3::Y),
    ));

    // Build the animation graph
    let mut graph = AnimationGraph::new();
    let animations = graph
        .add_clips(
            [
                GltfAssetLabel::Animation(0).from_asset(config.game.hangar_model.clone()),
                GltfAssetLabel::Animation(0).from_asset(config.game.flying_model.clone()),
                GltfAssetLabel::Animation(1).from_asset(config.game.flying_model.clone()),
            ]
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
