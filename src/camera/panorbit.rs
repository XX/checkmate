use std::f32::consts;

use bevy::camera::Projection;
use bevy::ecs::component::Component;
use bevy::ecs::message::MessageReader;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Query, Res};
use bevy::input::ButtonInput;
use bevy::input::mouse::{MouseButton, MouseMotion, MouseWheel};
use bevy::math::{Mat3, Quat, Vec2, Vec3};
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::time::Time;
use bevy::transform::components::Transform;
use bevy::window::Window;
use bevy_inspector_egui::bevy_egui::EguiContexts;
use big_space::prelude::{CellCoord, Grid};

use crate::camera::LookingAt;
use crate::world::{BigWorld, CellPoint};

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct PanOrbitCameraTarget {
    /// Точка, вокруг которой вращается камера.
    pub focus: CellPoint,

    pub radius: f32,
    pub rotation: Quat,
}

impl Default for PanOrbitCameraTarget {
    fn default() -> Self {
        PanOrbitCameraTarget {
            focus: CellPoint::default(),
            radius: 5.0,
            rotation: Quat::IDENTITY,
        }
    }
}

impl PanOrbitCameraTarget {
    pub fn new(grid: &Grid, position: Vec3, look_at: LookingAt) -> Self {
        let (radius, rotation) = Self::orbit(position, look_at);

        PanOrbitCameraTarget {
            focus: CellPoint::from_position(grid, look_at.target.as_dvec3()),
            radius,
            rotation,
        }
    }

    /// Радиус и поворот орбиты по паре «позиция камеры — точка взгляда».
    ///
    /// Обе точки задаются относительно фокуса, поэтому остаются небольшими и в `f32`
    /// представимы точно.
    pub fn orbit(position: Vec3, look_at: LookingAt) -> (f32, Quat) {
        let radius = (position - look_at.target).length();
        let rotation = Transform::from_translation(position)
            .looking_at(look_at.target, look_at.up)
            .rotation;

        (radius, rotation)
    }
}

#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct PanOrbitCamera {
    /// The "focus point" to orbit around. It is automatically updated when panning the camera
    pub focus: CellPoint,
    pub radius: f32,
    pub upside_down: bool,
    pub smoothness_speed: f32,
    pub orbit_button: MouseButton,
    pub pan_button: MouseButton,
    pub mouse_control_enabled: bool,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        Self {
            focus: CellPoint::default(),
            radius: 5.0,
            upside_down: false,
            smoothness_speed: 8.0,
            orbit_button: MouseButton::Left,
            pan_button: MouseButton::Right,
            mouse_control_enabled: true,
        }
    }
}

impl PanOrbitCamera {
    /// Переносит камеру на орбиту вокруг фокуса.
    ///
    /// Ячейка и смещение задаются вместе и согласованно, поэтому перецентровка `big_space`
    /// (она срабатывает, если смещение вышло за пределы ячейки) ничего не ломает: следующий
    /// кадр всё равно пересчитает позицию от фокуса, а не от текущего трансформа.
    pub fn update_position(&self, transform: &mut Transform, cell: &mut CellCoord) {
        let rot_matrix = Mat3::from_quat(transform.rotation);

        *cell = self.focus.cell;
        transform.translation = self.focus.offset + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, self.radius));
    }
}

pub fn update_input(
    windows: Query<&Window>,
    mut motion_events: MessageReader<MouseMotion>,
    mut scroll_events: MessageReader<MouseWheel>,
    input_mouse: Res<ButtonInput<MouseButton>>,
    world: BigWorld,
    mut query: Query<(&mut PanOrbitCamera, &mut PanOrbitCameraTarget, &Transform, &Projection)>,
) {
    let primary_window = windows.single().expect("Window must be single");
    let Some(grid) = world.grid() else {
        return;
    };

    for (mut camera, mut target, transform, projection) in query.iter_mut() {
        if !camera.mouse_control_enabled {
            continue;
        }

        let mut pan = Vec2::ZERO;
        let mut rotation_move = Vec2::ZERO;
        let mut scroll = 0.0_f32;
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
                let delta = rotation_move.x / window.x * consts::PI * 2.0;
                if camera.upside_down { -delta } else { delta }
            };
            let delta_y = rotation_move.y / window.y * consts::PI;
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

            target.focus.translate(grid, translation);
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
    world: BigWorld,
    mut query: Query<(
        &mut PanOrbitCamera,
        &PanOrbitCameraTarget,
        &mut Transform,
        &mut CellCoord,
    )>,
) {
    let Some(grid) = world.grid() else {
        return;
    };

    for (mut camera, target, mut transform, mut cell) in query.iter_mut() {
        let lerp_factor = 1.0 - (-camera.smoothness_speed * time.delta_secs()).exp();

        // Update camera params
        // Интерполяция идёт по разности точек, а не по их абсолютным координатам: остаток
        // сглаживания на сотнях километров меньше шага `f32` и в лобовом `lerp` просто
        // терялся бы, превращая плавное движение в ступеньки.
        let delta_focus = target.focus.delta_from(grid, &camera.focus);
        camera.focus.translate(grid, delta_focus * lerp_factor);
        camera.radius += (target.radius - camera.radius) * lerp_factor;

        // Interpolate rotation
        transform.rotation = transform.rotation.slerp(target.rotation, lerp_factor);

        camera.update_position(&mut transform, &mut cell);
    }
}

pub fn block_camera_mouse_control_when_hovering_ui(
    mut contexts: EguiContexts,
    mut camera_query: Query<&mut PanOrbitCamera>,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        let camera_mouse_control_enabled = if ctx.is_pointer_over_area() { false } else { true };
        for mut camera in camera_query.iter_mut() {
            camera.mouse_control_enabled = camera_mouse_control_enabled;
        }
    }
}

fn get_window_size(window: &Window) -> Vec2 {
    let window = Vec2::new(window.width() as f32, window.height() as f32);
    window
}
