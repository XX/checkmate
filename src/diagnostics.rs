use bevy::app::{App, Plugin, Startup};
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, SystemInformationDiagnosticsPlugin};
use bevy::ecs::system::Commands;
use iyes_perf_ui::prelude::{
    PerfUiEntryFPS, PerfUiEntryFPSWorst, PerfUiEntryFrameTime, PerfUiEntryFrameTimeWorst, PerfUiRoot,
};
use iyes_perf_ui::PerfUiPlugin;

pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin,
            SystemInformationDiagnosticsPlugin,
            PerfUiPlugin,
        ))
        .add_systems(Startup, spawn);
    }
}

pub fn spawn(mut commands: Commands) {
    commands.spawn((
        PerfUiRoot::default(),
        PerfUiEntryFPS::default(),
        PerfUiEntryFPSWorst::default(),
        PerfUiEntryFrameTime::default(),
        PerfUiEntryFrameTimeWorst::default(),
    ));
}
