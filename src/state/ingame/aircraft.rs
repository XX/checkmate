use bevy::ecs::component::Component;
use bevy::ecs::system::{Query, Res};
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::math::{EulerRot, Quat, Vec3};
use bevy::time::Time;
use bevy::transform::components::Transform;

#[derive(Component, Debug, Clone)]
pub struct Aircraft {
    pub max_speed: f32,

    // Скорость крена
    pub roll_speed: f32,

    // Скорость тангажа
    pub pitch_speed: f32,

    // Скорость рыскания
    pub yaw_speed: f32,
}

impl Aircraft {
    pub fn new() -> Self {
        Self {
            max_speed: 100.0,
            roll_speed: 3.0,
            pitch_speed: 2.0,
            yaw_speed: 1.0,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct Thrust {
    // Текущая тяга (0..1)
    pub current: f32,

    // Целевая тяга (0..1)
    pub target: f32,

    // Максимальная сила тяги
    pub max_force: f32,

    // Скорость изменения тяги
    pub change_speed: f32,
}

impl Thrust {
    pub fn new() -> Self {
        Self {
            current: 0.0,
            target: 20.0,
            max_force: 100.0,
            change_speed: 2.0,
        }
    }
}

#[derive(Component, Default, Clone, Copy)]
pub struct Movement {
    pub velocity: Vec3,
    pub rotation_speed: Vec3,
}

pub fn movement(mut query: Query<(&mut Transform, &mut Movement, &Thrust, &Aircraft)>, time: Res<Time>) {
    for (mut transform, mut movement, thrust, aircraft) in &mut query {
        // Направление самолета (вперед по локальной оси Z)
        let direction = transform.rotation * Vec3::Z;

        // Сила тяги
        let acceleration = direction * thrust.current * thrust.max_force;
        movement.velocity += acceleration * time.delta_secs();

        // Аэродинамическое сопротивление (упрощенное)
        let drag = movement.velocity * 0.1;
        movement.velocity -= drag * time.delta_secs();

        // Ограничиваем максимальную скорость
        if movement.velocity.length() > aircraft.max_speed {
            movement.velocity = movement.velocity.normalize() * aircraft.max_speed;
        }

        // Применяем скорость к позиции
        transform.translation += movement.velocity * time.delta_secs();
    }
}

pub fn rotation(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Movement, &Aircraft)>,
    time: Res<Time>,
) {
    for (mut transform, mut movement, aircraft) in &mut query {
        let mut rotation = Vec3::ZERO;

        // Управление рысканием (A/D)
        if keyboard_input.pressed(KeyCode::KeyA) {
            rotation.y += aircraft.yaw_speed;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            rotation.y -= aircraft.yaw_speed;
        }

        // Управление тангажом (Up/Down)
        if keyboard_input.pressed(KeyCode::ArrowUp) {
            rotation.x += aircraft.pitch_speed;
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) {
            rotation.x -= aircraft.pitch_speed;
        }

        // Управление креном (Left/Right)
        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            rotation.z -= aircraft.roll_speed;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) {
            rotation.z += aircraft.roll_speed;
        }

        // Применяем поворот
        if movement.rotation_speed != Vec3::ZERO || rotation != Vec3::ZERO {
            let smoothness_speed = 1.2;
            let lerp_factor = 1.0 - (-smoothness_speed * time.delta_secs()).exp();
            movement.rotation_speed = movement.rotation_speed.lerp(rotation, lerp_factor);

            let rotation_delta = Quat::from_euler(
                EulerRot::XYZ,
                movement.rotation_speed.x * time.delta_secs(),
                movement.rotation_speed.y * time.delta_secs(),
                movement.rotation_speed.z * time.delta_secs(),
            );

            transform.rotation *= rotation_delta;
        }
    }
}

pub fn update_thrust(keyboard_input: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Thrust>, time: Res<Time>) {
    for mut thrust in &mut query {
        // Управление тягой клавишами W/S или PageUp/PageDown
        if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::PageUp) {
            thrust.target = (thrust.target + time.delta_secs()).min(1.0);
        }
        if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::PageDown) {
            thrust.target = (thrust.target - time.delta_secs()).max(0.0);
        }

        // Плавное изменение тяги
        thrust.current = thrust.current + (thrust.target - thrust.current) * thrust.change_speed * time.delta_secs();
    }
}
