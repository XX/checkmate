use bevy::app::{App, Plugin, Startup, Update};
use bevy::core_pipeline::bloom::BloomSettings;
use bevy::core_pipeline::core_3d::Camera3dBundle;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::component::Component;
use bevy::ecs::event::EventReader;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::input::mouse::{MouseButton, MouseMotion, MouseWheel};
use bevy::input::ButtonInput;
use bevy::math::{Mat3, Quat, Vec2, Vec3};
use bevy::prelude::default;
use bevy::render::camera::{Camera, PerspectiveProjection, Projection};
use bevy::transform::components::Transform;
use bevy::window::Window;

pub struct PanOrbitCameraPlugin;

impl Plugin for PanOrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn).add_systems(Update, update_input);
    }
}

#[derive(Component)]
pub struct PanOrbitCamera {
    /// The "focus point" to orbit around. It is automatically updated when panning the camera
    pub focus: Vec3,
    pub radius: f32,
    pub upside_down: bool,
    pub orbit_button: MouseButton,
    pub pan_button: MouseButton,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: 5.0,
            upside_down: false,
            orbit_button: MouseButton::Left,
            pan_button: MouseButton::Right,
        }
    }
}

pub fn spawn(mut commands: Commands) {
    let translation = Vec3::new(-3.0, 5.0, 15.0);
    let radius = translation.length();

    commands.spawn((
        PanOrbitCamera { radius, ..default() },
        Camera3dBundle {
            camera: Camera { hdr: true, ..default() },
            tonemapping: Tonemapping::BlenderFilmic,
            projection: PerspectiveProjection {
                fov: 45.0_f32.to_radians(),
                ..default()
            }
            .into(),
            transform: Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        BloomSettings::NATURAL,
    ));
}

/// Pan the camera with middle mouse click, zoom with scroll wheel, orbit with right mouse click.
pub fn update_input(
    windows: Query<&Window>,
    mut motion_events: EventReader<MouseMotion>,
    mut scroll_events: EventReader<MouseWheel>,
    input_mouse: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut PanOrbitCamera, &mut Transform, &Projection)>,
) {
    let primary_window = windows.single();

    for (mut camera, mut transform, projection) in query.iter_mut() {
        let mut pan = Vec2::ZERO;
        let mut rotation_move = Vec2::ZERO;
        let mut scroll = 0.0;
        let mut orbit_button_changed = false;

        if input_mouse.pressed(camera.orbit_button) {
            for motion in motion_events.read() {
                rotation_move += motion.delta;
            }
        } else if input_mouse.pressed(camera.pan_button) {
            // Pan only if we're not rotating at the moment
            for motion in motion_events.read() {
                pan += motion.delta;
            }
        }
        for wheel in scroll_events.read() {
            scroll += wheel.y;
        }
        if input_mouse.just_released(camera.orbit_button) || input_mouse.just_pressed(camera.orbit_button) {
            orbit_button_changed = true;
        }

        if orbit_button_changed {
            // only check for upside down when orbiting started or ended this frame
            // if the camera is "upside" down, panning horizontally would be inverted, so invert the input to make it
            // correct
            let up = transform.rotation * Vec3::Y;
            camera.upside_down = up.y <= 0.0;
        }

        let mut any = false;
        if rotation_move.length_squared() > 0.0 {
            any = true;
            let window = get_window_size(primary_window);
            let delta_x = {
                let delta = rotation_move.x / window.x * std::f32::consts::PI * 2.0;
                if camera.upside_down {
                    -delta
                } else {
                    delta
                }
            };
            let delta_y = rotation_move.y / window.y * std::f32::consts::PI;
            let yaw = Quat::from_rotation_y(-delta_x);
            let pitch = Quat::from_rotation_x(-delta_y);
            transform.rotation = yaw * transform.rotation; // rotate around global y axis
            transform.rotation = transform.rotation * pitch; // rotate around local x axis
        } else if pan.length_squared() > 0.0 {
            any = true;
            // make panning distance independent of resolution and FOV,
            let window = get_window_size(primary_window);
            if let Projection::Perspective(projection) = projection {
                pan *= Vec2::new(projection.fov * projection.aspect_ratio, projection.fov) / window;
            }
            // translate by local axes
            let right = transform.rotation * Vec3::X * -pan.x;
            let up = transform.rotation * Vec3::Y * pan.y;
            // make panning proportional to distance away from focus point
            let translation = (right + up) * camera.radius;
            camera.focus += translation;
        } else if scroll.abs() > 0.0 {
            any = true;
            camera.radius -= scroll * camera.radius * 0.2;
            // dont allow zoom to reach zero or you get stuck
            camera.radius = f32::max(camera.radius, 0.05);
        }

        if any {
            // emulating parent/child to make the yaw/y-axis rotation behave like a turntable
            // parent = x and y rotation
            // child = z-offset
            let rot_matrix = Mat3::from_quat(transform.rotation);
            transform.translation = camera.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, camera.radius));
        }
    }

    // consume any remaining events, so they don't pile up if we don't need them
    // (and also to avoid Bevy warning us about not checking events every frame update)
    motion_events.clear();
}

fn get_window_size(window: &Window) -> Vec2 {
    let window = Vec2::new(window.width() as f32, window.height() as f32);
    window
}
