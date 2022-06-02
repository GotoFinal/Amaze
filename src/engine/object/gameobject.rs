use glam::{Quat, Vec2, Vec3};
use bevy_ecs::prelude::*;

use crate::engine::renderer::renderer::{Vertex, VertexIndex};

#[derive(Component, Clone, Copy)]
pub struct Transform {
    pub position: Vec3,
    pub scale: Vec3,
    pub rotation: Quat
}

#[derive(Component)]
pub struct RenderId {
    pub id: u32
}

#[derive(Component)]
pub struct Camera {
    pub enabled: bool,
    pub far_clip_plane: f32,
    pub near_clip_plane: f32
}

impl Transform {
    pub(crate) fn at(position: Vec3) -> Transform {
        return Transform {
            position,
            scale: Vec3::ONE,
            rotation: Quat::IDENTITY,
        };
    }
}

#[derive(Component, Clone, Copy)]
pub struct Velocity(Vec3);

//TODO: do i need something smarter and just push data to gpu and remove?
#[derive(Component, Clone)]
pub struct Mesh {
    pub id: u32,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<VertexIndex>,
}

fn test() {
    // let mut world = World::default();
    // let entity = world.push((Transform {
    //     position: Vec2::ZERO,
    //     scale: Vec2::ONE,
    //     rotation: 0.0,
    // },
    //                          Mesh {
    //                              vertices: Vec::new()
    //                          }
    // ));
    // world.entry()
}