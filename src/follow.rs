use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::Query;
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::transform::components::Transform;

#[derive(Component, Reflect, Debug, Default, Clone, Copy)]
#[reflect(Component, Default)]
pub struct Follower {
    pub followee: Option<Entity>,
    pub turn_towards: bool,
}

#[derive(Component, Reflect, Debug, Default, Clone, Copy)]
#[reflect(Component, Default)]
pub struct Followee;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PreviousTransform(pub Transform);

pub fn update_previous_transform(mut query: Query<(&Transform, &mut PreviousTransform)>) {
    for (transform, mut prev_transform) in &mut query {
        prev_transform.0 = *transform;
    }
}
