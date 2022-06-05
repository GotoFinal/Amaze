use glam::{Mat4, Quat, Vec2, Vec3};
use bevy_ecs::prelude::*;

use crate::engine::renderer::renderer::{Vertex, VertexIndex};

#[derive(Component)]
pub struct RenderId {
    pub id: u32
}

#[derive(Component, Copy, Clone, PartialEq)]
pub struct Camera {
    pub aspect_ratio: f32,
    pub far_clip_plane: f32,
    pub near_clip_plane: f32,
    pub field_of_view: f32
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