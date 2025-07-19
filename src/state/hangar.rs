use bevy::animation::graph::AnimationNodeType;
use bevy::animation::{AnimationClip, AnimationPlayer};
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::color::{Color, ColorToComponents, LinearRgba};
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{Commands, Local, Query, Res, ResMut};
use bevy::gltf::GltfAssetLabel;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::math::Vec3;
use bevy::math::primitives::Plane3d;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::{AnimationGraph, Entity, MeshBuilder};
use bevy::render::mesh::{Mesh, Mesh3d, Meshable};
use bevy::scene::SceneRoot;
use bevy::transform::components::Transform;

use crate::camera::{AppCameraEntity, AppCameraParams};
use crate::config::Config;
use crate::state::{SceneKey, Scenes};
use crate::utils::combine_meshes;
use crate::{Animations, camera};

#[derive(Resource)]
pub struct HangarData {
    pub entities: Vec<Entity>,
    pub meshes: Vec<Handle<Mesh>>,
    pub materials: Vec<Handle<StandardMaterial>>,
}

pub fn setup(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    mut scenes: ResMut<Scenes>,
    camera: Res<AppCameraEntity>,
    camera_params: ResMut<AppCameraParams>,
) {
    let scene = scenes
        .hangar
        .entry(SceneKey::Aircraft)
        .or_insert_with(|| asset_server.load(GltfAssetLabel::Scene(0).from_asset(config.game.hangar_model.clone())))
        .clone();

    let height = 2.31;
    let entity_id = commands
        .spawn((SceneRoot(scene), Transform::from_translation(Vec3::ZERO.with_y(height))))
        .id();

    commands.insert_resource(HangarData {
        entities: vec![entity_id],
        meshes: vec![],
        materials: vec![],
    });

    camera::respawn_panorbit(commands, camera_params, camera.entity_id, &config.camera.follow, height);
}

pub fn cleanup(
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

pub fn chessboard_land_spawn(
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

pub fn control_land_gear_animation(
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
