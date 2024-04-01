#![allow(dead_code)]
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
