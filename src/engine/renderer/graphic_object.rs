use std::sync::Arc;

use glam::{Vec2, Vec3};
use vulkano::buffer::CpuAccessibleBuffer;

use crate::engine::object::gameobject::{Mesh, Transform};
use crate::engine::renderer::buffers::BufferCreator;
use crate::engine::renderer::material::{Material, MaterialKey};
use crate::engine::renderer::renderer::{Vertex, VertexIndex};

pub struct GraphicObjectDesc {
    pub transform: Transform,
    pub mesh: Mesh,
    pub material: MaterialKey
}

// TODO: how do i store the actual stuff to render? for later:
// - create separate struct for objects that consist of vertices and later more stuff
// - process it into one big buffer i guess?
// - should original data become just slices?
// - but will rust even allow that?
// - but do I want that data? i could push it to GPU and forget i guess?
// - but for now i plan 2d game so do i care?
pub struct RenderMesh {
    pub transform: Transform,
    pub data: RenderMeshData,
    pub material: MaterialKey
}

#[derive(Clone)]
pub struct RenderMeshData {
    pub vertices_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    pub indices_buffer: Arc<CpuAccessibleBuffer<[VertexIndex]>>,
}

pub trait GraphicObject {
    fn set_position(&mut self, pos: Vec3);

    fn create(desc: GraphicObjectDesc, buffers: &dyn BufferCreator) -> Self;
}

impl GraphicObject for RenderMesh {
    fn set_position(&mut self, pos: Vec3) {
        self.transform.position = pos
    }

    fn create(desc: GraphicObjectDesc, buffers: &dyn BufferCreator) -> Self {
        return RenderMesh {
            transform: desc.transform,
            data: buffers.create_cpu_buffer(&desc.mesh),
            material: desc.material
        };
    }
}