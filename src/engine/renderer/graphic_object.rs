use std::sync::Arc;

use glam::Vec2;
use vulkano::buffer::CpuAccessibleBuffer;

use crate::engine::object::gameobject::{Mesh, Transform};
use crate::engine::renderer::buffers::BufferCreator;
use crate::engine::renderer::renderer::Vertex;

#[derive(Debug, Clone)]
pub struct GraphicObjectDesc {
    pub transform: Transform,
    pub mesh: Mesh,
}

// TODO: how do i store the actual stuff to render? for later:
// - create separate struct for objects that consist of vertices and later more stuff
// - process it into one big buffer i guess?
// - should original data become just slices?
// - but will rust even allow that?
// - but do I want that data? i could push it to GPU and forget i guess?
// - but for now i plan 2d game so do i care?
#[derive(Debug, Clone)]
pub struct TheStuffToRender {
    pub transform: Transform,
    pub vertices_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>, // thats bascially the same data..., but lets think about it later
    // TODO: material i guess?
}

pub trait GraphicObject {
    fn set_position(&mut self, pos: Vec2);

    fn create(desc: GraphicObjectDesc, buffers: &dyn BufferCreator) -> Self;
}

impl GraphicObject for TheStuffToRender {
    fn set_position(&mut self, pos: Vec2) {
        self.transform.position = pos
    }

    fn create(desc: GraphicObjectDesc, buffers: &dyn BufferCreator) -> Self {
        return TheStuffToRender {
            transform: desc.transform,
            vertices_buffer: buffers.create_cpu_vertex_buffer(desc.mesh.vertices),
        };
    }
}