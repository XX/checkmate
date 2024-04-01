use bevy::app::{App, Plugin, Startup, Update};
use bevy::core_pipeline::core_3d::Camera3dBundle;
use bevy::ecs::component::Component;
use bevy::ecs::event::EventReader;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::input::mouse::{MouseButton, MouseMotion, MouseWheel};
use bevy::input::ButtonInput;
use bevy::math::{EulerRot, Quat, Vec3};
use bevy::prelude::default;
use bevy::render::camera::Camera;
use bevy::time::Time;
use bevy::transform::components::Transform;

pub struct SimpleCameraPlugin;

impl Plugin for SimpleCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn).add_systems(Update, update_input);
    }
}

#[derive(Component)]
pub struct SimpleCamera {
    pub rotation: Quat,
    pub zoom: f32,
}

impl Default for SimpleCamera {
    fn default() -> Self {
        Self {
            rotation: Quat::IDENTITY,
            zoom: 20.0,
        }
    }
}

pub fn spawn(mut commands: Commands) {
    let translation = Vec3::new(0.7, 20.0, 40.0);

    commands.spawn((SimpleCamera::default(), Camera3dBundle {
        camera: Camera { hdr: true, ..default() },
        transform: Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    }));
}

pub fn update_input(
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut SimpleCamera, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut controller, mut transform) in query.iter_mut() {
        for wheel in mouse_wheel.read() {
            controller.zoom -= wheel.y;
        }
        if buttons.pressed(MouseButton::Left) {
            for mouse in mouse_motion.read() {
                let delta = mouse.delta * time.delta_seconds() * 0.1;
                controller.rotation *= Quat::from_euler(EulerRot::XYZ, -delta.y, -delta.x, 0.0);
            }
        }
        transform.translation = controller.rotation * Vec3::Z * controller.zoom;
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}
