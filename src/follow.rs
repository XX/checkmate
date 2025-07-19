use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::system::Query;
use bevy::transform::components::Transform;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Follower {
    pub followee: Option<Entity>,
    pub turn_towards: bool,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Followee;

#[derive(Component)]
pub struct PreviousTransform(pub Transform);

pub fn update_previous_transform(mut query: Query<(&Transform, &mut PreviousTransform)>) {
    for (transform, mut prev_transform) in &mut query {
        prev_transform.0 = *transform;
    }
}
