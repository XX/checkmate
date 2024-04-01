use std::f32::consts::{FRAC_PI_4, PI};

use bevy::app::{App, Startup, Update};
use bevy::asset::{AssetServer, Assets};
use bevy::ecs::component::Component;
use bevy::ecs::query::With;
use bevy::ecs::system::{Commands, Query, Res, ResMut, Resource};
use bevy::math::primitives::Plane3d;
use bevy::math::{EulerRot, Quat, Vec3};
use bevy::pbr::{
    AmbientLight, DirectionalLight, DirectionalLightBundle, DirectionalLightShadowMap, PbrBundle, StandardMaterial,
};
use bevy::prelude::default;
use bevy::reflect::Reflect;
use bevy::render::camera::ClearColor;
use bevy::render::color::Color;
use bevy::render::mesh::{Mesh, Meshable};
use bevy::scene::SceneBundle;
use bevy::time::Time;
use bevy::transform::components::Transform;
use bevy::DefaultPlugins;
use camera::panorbit::PanOrbitCameraPlugin;
use diagnostics::DiagnosticsPlugin;
use utils::combine_meshes;

mod camera;
mod diagnostics;
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
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(PlaneSettings {
        move_interval: 1.3,
        box_area: 6.0,
        speed: 1.5,
        wobble_speed: 5.0,
        rotation_speed: 0.7,
    });
    commands.insert_resource(ClearColor(Color::rgb(0.7, 0.92, 0.96)));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(-2.0, 2.5, 5.0)).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.spawn((
        PlaneMovement {
            target_pos: Vec3::ZERO,
            timer: 0.0,
        },
        SceneBundle {
            scene: asset_server.load("su-75/su-75.gltf#Scene0"),
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
                    Color::RED
                } else {
                    Color::WHITE
                }
                .as_linear_rgba_f32();
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

#[allow(dead_code)]
mod old {
    use super::*;

    fn chessboard_land_spawn(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let black_material = materials.add(StandardMaterial {
            base_color: Color::RED,
            ..default()
        });
        let white_material = materials.add(Color::WHITE);

        let cell_mesh = Plane3d::default().mesh().size(2.0, 2.0).build();
        let cell_mesh_handle = meshes.add(cell_mesh.clone());

        for x in -7..8 {
            for z in -7..250 {
                let transform = Transform::from_xyz(x as f32 * 2.0, -2.31, z as f32 * 2.0);

                commands.spawn(PbrBundle {
                    mesh: cell_mesh_handle.clone(),
                    material: if (x + z) % 2 == 0 {
                        black_material.clone()
                    } else {
                        white_material.clone()
                    },
                    transform,
                    ..default()
                });
            }
        }
    }

    fn animate_light_direction(time: Res<Time>, mut query: Query<&mut Transform, With<DirectionalLight>>) {
        for mut transform in &mut query {
            transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, time.elapsed_seconds() * PI / 5.0, -FRAC_PI_4);
        }
    }
}
