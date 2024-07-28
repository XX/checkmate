use std::f32::consts::{FRAC_PI_4, PI};

use bevy::animation::{animate_targets, AnimationClip, AnimationPlayer};
use bevy::app::{App, Startup, Update};
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::color::{Color, ColorToComponents, LinearRgba};
use bevy::ecs::component::Component;
use bevy::ecs::query::{Added, With};
use bevy::ecs::system::{Commands, Local, Query, Res, ResMut, Resource};
use bevy::gltf::GltfAssetLabel;
use bevy::input::keyboard::KeyCode;
use bevy::input::ButtonInput;
use bevy::math::primitives::Plane3d;
use bevy::math::{EulerRot, Quat, Vec3};
use bevy::pbr::{
    AmbientLight, DirectionalLight, DirectionalLightBundle, DirectionalLightShadowMap, PbrBundle, StandardMaterial,
};
use bevy::prelude::{default, AnimationGraph, AnimationNodeIndex, Entity, IntoSystemConfigs, MeshBuilder};
use bevy::reflect::Reflect;
use bevy::render::camera::ClearColor;
use bevy::render::mesh::{Mesh, Meshable};
use bevy::scene::SceneBundle;
use bevy::transform::components::Transform;
use bevy::window::Window;
use bevy::{log, DefaultPlugins};
use camera::panorbit::PanOrbitCameraPlugin;
use diagnostics::DiagnosticsPlugin;
use utils::combine_meshes;

mod camera;
mod diagnostics;
// mod old;
mod utils;

pub const LANDSCAPE_SIZE: f32 = 1200.0;
pub const LANDSCAPE_SIZE_HALF: f32 = LANDSCAPE_SIZE * 0.5;

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
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .add_plugins(DiagnosticsPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .add_systems(Startup, (chessboard_land_spawn, setup))
        .add_systems(Update, attach_animations.before(animate_targets))
        .add_systems(Update, control_land_gear_animation)
        .add_systems(Update, close_on_esc)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut graphs: ResMut<Assets<AnimationGraph>>) {
    commands.insert_resource(PlaneSettings {
        move_interval: 1.3,
        box_area: 6.0,
        speed: 1.5,
        wobble_speed: 5.0,
        rotation_speed: 0.7,
    });
    commands.insert_resource(ClearColor(Color::srgb(0.7, 0.92, 0.96)));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(2.0, 0.5, 5.0)).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Build the animation graph
    let mut graph = AnimationGraph::new();
    let animations = graph
        .add_clips(
            [GltfAssetLabel::Animation(0).from_asset("su-75_anim/su-75.gltf")]
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

    commands.spawn((
        PlaneMovement {
            target_pos: Vec3::ZERO,
            timer: 0.0,
        },
        SceneBundle {
            scene: asset_server.load("su-75_anim/su-75.gltf#Scene0"),
            ..default()
        },
    ));
}

fn chessboard_land_spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut mesh_data = Vec::new();
    let cell_mesh = Plane3d::default().mesh().size(2.0, 2.0).build();

    for x in -7..8 {
        for z in -7..250 {
            let transform = Transform::from_xyz(x as f32 * 2.0, -2.31, z as f32 * 2.0);

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
    commands.spawn(PbrBundle {
        mesh,
        material: materials.add(Color::WHITE),
        ..default()
    });
}

/// Attaches the animation graph to the scene
fn attach_animations(
    mut commands: Commands,
    to_animated_entities: Query<(Entity, &AnimationPlayer), Added<AnimationPlayer>>,
    animations: Res<Animations>,
) {
    for (entity, _player) in &to_animated_entities {
        log::info!("Attaching animations");
        commands.entity(entity).insert(animations.graph.clone());
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
                animation_node
                    .clip
                    .as_ref()
                    .and_then(|clip_handle| animation_clips.get(clip_handle).map(|clip| clip.duration()))
                    .unwrap_or_default()
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
