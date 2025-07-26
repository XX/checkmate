use bevy::DefaultPlugins;
use bevy::app::{App, Startup, Update};
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::color::Color;
use bevy::ecs::component::Component;
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::gltf::GltfAssetLabel;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::math::Vec3;
use bevy::pbr::{Atmosphere, AtmosphereSettings, DirectionalLight, DirectionalLightShadowMap};
use bevy::prelude::{AnimationGraph, AnimationNodeIndex, Entity, default};
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
            (hangar::setup, hangar::chessboard_land_spawn).chain(),
        )
        .add_systems(OnExit(AppState::Hangar), hangar::cleanup)
        .add_systems(
            OnEnter(AppState::InGame),
            (ingame::setup, ingame::terrain::setup).chain(),
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
        .add_systems(
            Update,
            hangar::control_land_gear_animation.run_if(in_state(AppState::Hangar)),
        )
        .add_systems(Update, close_on_esc)
        .run();
}

#[derive(Component)]
struct Sun;

#[derive(Resource)]
struct Animations {
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
    Gears = 0,
    LeftRuddervatorTurnLeft,
    LeftRuddervatorTurnRight,
    Rest,
    RightRuddervatorTurnLeft,
    RightRuddervatorTurnRight,
    PitchDown,
    PitchUp,
}

fn setup(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
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
    let mut animations = Vec::new();
    let mut graph = AnimationGraph::new();

    let animation_node = graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(config.game.hangar_model.clone())),
        1.0,
        graph.root,
    );
    animations.push(animation_node);

    let parent = graph.root;
    let weight = 1.0;
    for i in 0..7 {
        let animation_node = graph.add_clip(
            asset_server.load(GltfAssetLabel::Animation(i).from_asset(config.game.flying_model.clone())),
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

    next_state.set(config.game.state);
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
