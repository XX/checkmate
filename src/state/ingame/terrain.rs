use bevy::asset::AssetServer;
use bevy::ecs::component::Component;
use bevy::ecs::system::{Commands, Res, ResMut};
use bevy::gltf::GltfAssetLabel;
use bevy::scene::SceneRoot;

use crate::config::Config;
use crate::state::ingame::GameData;
use crate::state::{SceneKey, Scenes};

#[derive(Component)]
struct Terrain;

pub fn setup(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    mut scenes: ResMut<Scenes>,
    mut data: ResMut<GameData>,
) {
    // Terrain
    let scene = scenes
        .game
        .entry(SceneKey::Terrain)
        .or_insert_with(|| {
            let model_path = config.game.terrain.model.clone();
            if model_path.ends_with(".gltf") || model_path.ends_with(".glb") {
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(model_path))
            } else {
                asset_server.load(model_path)
            }
        })
        .clone();
    let terrain_id = commands
        .spawn((Terrain, SceneRoot(scene.clone()), config.game.terrain.get_transform()))
        .id();

    data.entities.push(terrain_id);
}
