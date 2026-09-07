use bevy::asset::AssetServer;
use bevy::ecs::component::Component;
use bevy::ecs::name::Name;
use bevy::ecs::query::Changed;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::gltf::GltfAssetLabel;
use bevy::math::{Quat, Vec3};
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::transform::components::Transform;
use bevy::world_serialization::WorldAssetRoot;

use crate::config::{Config, TerrainSettings};
use crate::state::ingame::GameData;
use crate::state::{SceneKey, Scenes};

#[derive(Component, Reflect, Debug, Default, Clone, Copy)]
#[reflect(Component, Default)]
pub struct Terrain;

/// Параметры размещения ландшафта, изменяемые во время игры.
///
/// Источник истины для [`Transform`] сущности ландшафта: изменение полей применяется
/// системой [`apply_terrain`]. Путь к модели остаётся структурным параметром и берётся
/// из конфига при создании сцены.
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct TerrainParams {
    pub position: Vec3,

    pub rotation: Quat,

    pub scale: f32,
}

impl From<&TerrainSettings> for TerrainParams {
    fn from(settings: &TerrainSettings) -> Self {
        Self {
            position: settings.position.into(),
            rotation: settings.get_rotation(),
            scale: settings.scale,
        }
    }
}

impl TerrainParams {
    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.position)
            .with_rotation(self.rotation)
            .with_scale(Vec3::splat(self.scale))
    }
}

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

    let params = TerrainParams::from(&config.game.terrain);
    let terrain_id = commands
        .spawn((
            Name::new("Terrain"),
            Terrain,
            params,
            WorldAssetRoot(scene.clone()),
            params.transform(),
        ))
        .id();

    data.entities.push(terrain_id);
}

pub fn apply_terrain(mut query: Query<(&TerrainParams, &mut Transform), Changed<TerrainParams>>) {
    for (params, mut transform) in &mut query {
        *transform = params.transform();
    }
}
