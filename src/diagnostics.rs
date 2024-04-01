use bevy::app::{App, Plugin, Startup};
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, SystemInformationDiagnosticsPlugin};
use bevy::ecs::system::Commands;
use iyes_perf_ui::diagnostics::{PerfUiEntryFPS, PerfUiEntryFPSWorst, PerfUiEntryFrameTime, PerfUiEntryFrameTimeWorst};
use iyes_perf_ui::{PerfUiPlugin, PerfUiRoot};

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
