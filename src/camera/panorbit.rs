use bevy::ecs::component::Component;
use bevy::ecs::event::EventReader;
use bevy::ecs::system::{Query, Res};
use bevy::input::ButtonInput;
use bevy::input::mouse::{MouseButton, MouseMotion, MouseWheel};
use bevy::math::{Mat3, Quat, Vec2, Vec3};
use bevy::render::camera::Projection;
use bevy::time::Time;
use bevy::transform::components::Transform;
use bevy::window::Window;

#[derive(Component, Copy, Clone, Debug)]
pub struct PanOrbitCameraTarget {
    pub focus: Vec3,
    pub radius: f32,
    pub rotation: Quat,
}

impl Default for PanOrbitCameraTarget {
    fn default() -> Self {
        PanOrbitCameraTarget {
            focus: Vec3::ZERO,
            radius: 5.0,
            rotation: Quat::IDENTITY,
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct PanOrbitCamera {
    /// The "focus point" to orbit around. It is automatically updated when panning the camera
    pub focus: Vec3,
    pub radius: f32,
    pub upside_down: bool,
    pub orbit_button: MouseButton,
    pub pan_button: MouseButton,
    pub smoothness_speed: f32,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 5.0,
            upside_down: false,
            orbit_button: MouseButton::Left,
            pan_button: MouseButton::Right,
            smoothness_speed: 8.0,
        }
    }
}

/// Pan the camera with middle mouse click, zoom with scroll wheel, orbit with right mouse click.
pub fn update_input(
    windows: Query<&Window>,
    mut motion_events: EventReader<MouseMotion>,
    mut scroll_events: EventReader<MouseWheel>,
    input_mouse: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut PanOrbitCamera, &mut PanOrbitCameraTarget, &Transform, &Projection)>,
) {
    let primary_window = windows.single().expect("Window must be single");

    for (mut camera, mut target, transform, projection) in query.iter_mut() {
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

        if rotation_move.length_squared() > 0.0 {
            let window = get_window_size(primary_window);
            let delta_x = {
                let delta = rotation_move.x / window.x * std::f32::consts::PI * 2.0;
                if camera.upside_down { -delta } else { delta }
            };
            let delta_y = rotation_move.y / window.y * std::f32::consts::PI;
            let yaw = Quat::from_rotation_y(-delta_x);
            let pitch = Quat::from_rotation_x(-delta_y);

            target.rotation = yaw * target.rotation; // rotate around global y axis
            target.rotation = target.rotation * pitch; // rotate around local x axis
        } else if pan.length_squared() > 0.0 {
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

            target.focus += translation;
        } else if scroll.abs() > 0.0 {
            target.radius -= scroll * target.radius * 0.2;
            // dont allow zoom to reach zero or you get stuck
            target.radius = target.radius.max(0.05);
        }
    }

    // consume any remaining events, so they don't pile up if we don't need them
    // (and also to avoid Bevy warning us about not checking events every frame update)
    motion_events.clear();
}

pub fn interpolate_camera(
    time: Res<Time>,
    mut query: Query<(&mut PanOrbitCamera, &PanOrbitCameraTarget, &mut Transform)>,
) {
    for (mut camera, target, mut transform) in query.iter_mut() {
        let lerp_factor = 1.0 - (-camera.smoothness_speed * time.delta_secs()).exp();

        // Update camera params
        camera.focus = camera.focus.lerp(target.focus, lerp_factor);
        camera.radius += (target.radius - camera.radius) * lerp_factor;

        // Interpolate rotation
        transform.rotation = transform.rotation.slerp(target.rotation, lerp_factor);

        // Update camera position
        let rot_matrix = Mat3::from_quat(transform.rotation);
        transform.translation = camera.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, camera.radius));
    }
}

fn get_window_size(window: &Window) -> Vec2 {
    let window = Vec2::new(window.width() as f32, window.height() as f32);
    window
}
