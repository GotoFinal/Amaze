use glam::Vec2;
use legion::{Entity, World};

use crate::engine::renderer::renderer::{Vertex, VertexIndex};

// TODO: Do I try to abstract ECS/legion away or just whatever?
pub trait GameObject {
    fn id(&self) -> Entity;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation: f32, // Do i want 3d rotation?
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderId {
    pub id: u32
}

impl Transform {
    pub(crate) fn at(position: Vec2) -> Transform {
        return Transform {
            position,
            scale: Vec2::ONE,
            rotation: 0.0,
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocity(Vec2);

//TODO: do i need something smarter and just push data to gpu and remove? probably does not mater for 2d game
#[derive(Clone, Debug)]
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