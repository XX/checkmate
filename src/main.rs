use bevy::DefaultPlugins;
use bevy::app::{App, Startup, Update};
use bevy::camera::{ClearColorConfig, Exposure};
use bevy::color::Color;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::schedule::common_conditions::{resource_exists, run_once};
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::input::ButtonInput;
use bevy::input::common_conditions::input_toggle_active;
use bevy::input::keyboard::KeyCode;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{Atmosphere, DirectionalLightShadowMap};
use bevy::pbr::AtmosphereSettings;
use bevy::prelude::Entity;
use bevy::state::app::AppExtStates;
use bevy::state::condition::in_state;
use bevy::state::state::{NextState, OnEnter, OnExit};
use bevy::window::Window;
use bevy_inspector_egui::bevy_egui::{EguiGlobalSettings, EguiPlugin};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_obj::ObjPlugin;
use clap::Parser;

use crate::camera::{AppCameraParams, AppCameraPlugin, CameraParams};
use crate::config::Config;
use crate::diagnostics::DiagnosticsPlugin;
use crate::environment::AmbientParams;
use crate::params::ParamsPlugin;
use crate::remote::AppRemotePlugin;
use crate::save::SaveConfigPlugin;
use crate::state::ingame::animation::AdditionalPlayers;
use crate::state::{AppState, Scenes, hangar, ingame};

mod camera;
mod cli;
mod config;
mod diagnostics;
mod environment;
mod follow;
mod params;
mod remote;
mod save;
mod state;
mod utils;

fn main() {
    let opts: cli::Opts = cli::Opts::parse();
    let config = Config::load(opts.config).unwrap_or_else(|err| {
        eprintln!("WARNING: config load error: {err}, use default config");
        Config::default()
    });

    let camera_params = AppCameraParams::default()
        .with_smoothness_speed(config.camera.smoothness_speed)
        .with_tonemapping(config.camera.tonemap)
        .with_bloom(config.camera.bloom)
        .with_follower(config.camera.follow.to_follower());

    let camera_params = if config.environment.atmosphere.enabled {
        camera_params
            .with_clear_color_config(ClearColorConfig::Default)
            .with_exposure(Exposure {
                ev100: config.camera.exposure,
            })
            .with_atmosphere((
                Atmosphere::earth(Default::default()),
                ScatteringMedium::default(),
                AtmosphereSettings {
                    // aerial_view_lut_max_distance: 3.2e5,
                    // scene_units_to_m: 1.0, //1e+4,
                    rendering_method: config.environment.atmosphere.render_mode.into(),
                    ..Default::default()
                },
            ))
    } else {
        camera_params.with_custom_clear_color(Color::srgb(0.7, 0.92, 0.96))
    };

    let camera_params = if let Some(auto_exposure) = config.camera.auto_exposure.to_auto_exposure() {
        camera_params.with_auto_exposure(auto_exposure)
    } else {
        camera_params
    };

    let remote_plugin = AppRemotePlugin::new(&config.remote);

    let mut app = App::new();

    app.insert_resource(AmbientParams::from(&config.environment.ambient))
        .insert_resource(CameraParams::from_config(&config))
        .insert_resource(camera_params)
        .insert_resource(DirectionalLightShadowMap {
            size: config.graphics.shadow_map_size,
        })
        .insert_resource(config)
        .insert_resource(Scenes::default())
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::Backquote)),
            ObjPlugin,
            DiagnosticsPlugin,
            AppCameraPlugin,
            ParamsPlugin,
            SaveConfigPlugin,
            remote_plugin,
        ))
        .init_state::<AppState>()
        .add_systems(Startup, (setup, environment::setup))
        .add_systems(
            OnEnter(AppState::Hangar),
            (
                hangar::setup_animation_graph.run_if(run_once),
                hangar::setup,
                hangar::chessboard_land_spawn,
            )
                .chain(),
        )
        .add_systems(OnExit(AppState::Hangar), hangar::cleanup)
        .add_systems(
            OnEnter(AppState::InGame),
            (
                ingame::animation::setup_animation_graph.run_if(run_once),
                ingame::setup,
                ingame::terrain::setup,
                ingame::engine::setup_jet_fire,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                ingame::aircraft::update_thrust,
                ingame::aircraft::movement,
                ingame::aircraft::rotation,
                ingame::animation::control.run_if(resource_exists::<AdditionalPlayers>),
                ingame::engine::flickering_light_system,
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

fn setup(
    config: Res<Config>,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    // Disable the automatic creation of a primary context to set it up manually for the camera we need.
    egui_global_settings.auto_create_primary_context = false;

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
