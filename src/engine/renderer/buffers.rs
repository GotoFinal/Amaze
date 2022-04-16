use std::sync::Arc;

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};

use crate::engine::renderer::renderer::Vertex;
use crate::GraphicEngine;

pub trait BufferCreator {
    fn create_cpu_vertex_buffer(&self, data: Vec<Vertex>) -> Arc<CpuAccessibleBuffer<[Vertex]>>;
}

impl BufferCreator for GraphicEngine {
    fn create_cpu_vertex_buffer(&self, data: Vec<Vertex>) -> Arc<CpuAccessibleBuffer<[Vertex]>> {
        CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::vertex_buffer(),
            false,
            data.into_iter(),
        ).unwrap()
    }
}