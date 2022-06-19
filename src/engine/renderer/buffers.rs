use std::sync::Arc;
use bytemuck::Pod;

use vulkano::buffer::{BufferContents, BufferUsage, CpuAccessibleBuffer, CpuBufferPool, DeviceLocalBuffer};

use crate::engine::renderer::renderer::{Vertex, VertexIndex};
use crate::{GraphicEngine, Mesh};
use crate::engine::renderer::graphic_object::RenderMeshData;

pub trait BufferCreator {
    fn create_cpu_buffer(&self, data: &Mesh) -> RenderMeshData;
    fn create_cpu_vertex_buffer(&self, data: Vec<Vertex>) -> Arc<CpuAccessibleBuffer<[Vertex]>>;
    fn create_cpu_indices_buffer(&self, data: Vec<VertexIndex>) -> Arc<CpuAccessibleBuffer<[VertexIndex]>>;
    // fn create_cpu_ubo_buffer<T>(&self, data: Vec<T>, frames_in_flight: u32) -> Arc<CpuAccessibleBuffer<[T]>>
    //     where T: Pod + Send + Sync + Sized;
}

impl BufferCreator for GraphicEngine {
    fn create_cpu_buffer(&self, data: &Mesh) -> RenderMeshData {
        let cached = self.get_cached_mesh(data.id);
        if cached.is_some() {
            return cached.unwrap().clone();
        }
        let data = data.clone();
        let cached = RenderMeshData {
            vertices_buffer: self.create_cpu_vertex_buffer(data.vertices),
            indices_buffer: self.create_cpu_indices_buffer(data.indices),
            id: data.id
        };
        self.mesh_cache.borrow_mut().insert(data.id, cached.clone());
        return cached;
    }

    fn create_cpu_vertex_buffer(&self, data: Vec<Vertex>) -> Arc<CpuAccessibleBuffer<[Vertex]>> {
        CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::vertex_buffer(),
            false,
            data.into_iter(),
        ).unwrap()
    }

    fn create_cpu_indices_buffer(&self, data: Vec<VertexIndex>) -> Arc<CpuAccessibleBuffer<[VertexIndex]>> {
        CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::index_buffer(),
            false,
            data.into_iter(),
        ).unwrap()
    }
    //
    // fn create_cpu_ubo_buffer<T>(&self, data: Vec<T>, frames_in_flight: u32) -> Arc<CpuAccessibleBuffer<[T]>> where T: Pod + Send + Sync + Sized {
    //     CpuAccessibleBuffer::from_iter(
    //         self.device.clone(),
    //         BufferUsage::uniform_buffer(),
    //         false,
    //         data.into_iter(),
    //     ).unwrap()
    // }
}