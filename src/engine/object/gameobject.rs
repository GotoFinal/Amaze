use glam::Vec2;
use legion::{Entity, World};

trait GameObject {
    fn id() -> u64;
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Transform {
    position: Vec2,
    scale: Vec2,
    rotation: f32, // Do i want 3d rotation?
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity(Vec2);

fn test() {
    let mut world = World::default();
    let entity = world.push((Transform {
        position: Vec2::ZERO,
        scale: Vec2::ONE,
        rotation: 0.0
    },));
}