use std::collections::HashMap;

use bevy::asset::Handle;
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Res, ResMut};
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::state::state::{NextState, State, States};
use bevy::world_serialization::WorldAsset;
use serde::{Deserialize, Serialize};

pub mod hangar;
pub mod ingame;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States, Serialize, Deserialize)]
pub enum AppState {
    #[default]
    Loading,
    Hangar,
    InGame,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum SceneKey {
    Aircraft,
    Terrain,
}

#[derive(Default, Resource)]
pub struct Scenes {
    pub hangar: HashMap<SceneKey, Handle<WorldAsset>>,
    pub game: HashMap<SceneKey, Handle<WorldAsset>>,
}

pub fn change(
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
