use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::Query;
use bevy::math::Quat;
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::transform::components::Transform;
use big_space::prelude::CellCoord;

use crate::world::CellPoint;

#[derive(Component, Reflect, Debug, Default, Clone, Copy)]
#[reflect(Component, Default)]
pub struct Follower {
    pub followee: Option<Entity>,
    pub turn_towards: bool,
}

#[derive(Component, Reflect, Debug, Default, Clone, Copy)]
#[reflect(Component, Default)]
pub struct Followee;

/// Положение сущности в предыдущем кадре: по разности с текущим камера догоняет цель.
#[derive(Component, Reflect, Debug, Default, Clone, Copy)]
#[reflect(Component, Default)]
pub struct PreviousTransform {
    pub point: CellPoint,
    pub rotation: Quat,
}

impl PreviousTransform {
    pub fn new(cell: CellCoord, transform: &Transform) -> Self {
        Self {
            point: CellPoint::new(cell, transform.translation),
            rotation: transform.rotation,
        }
    }
}

pub fn update_previous_transform(mut query: Query<(&Transform, &CellCoord, &mut PreviousTransform)>) {
    for (transform, cell, mut previous) in &mut query {
        *previous = PreviousTransform::new(*cell, transform);
    }
}
