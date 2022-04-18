use std::f32::consts::PI;

use glam::Vec2;

use crate::engine::renderer::renderer::{Vertex, VertexIndex};
use crate::Mesh;

pub fn generate_circle_mesh(vertex_count: usize, radius: f32) -> Mesh {
    let (vertices, indices) = generate_circle(vertex_count, radius);
    return Mesh {
        vertices,
        indices,
    };
}

pub fn generate_circle(vertex_count: usize, radius: f32) -> (Vec<Vertex>, Vec<VertexIndex>) {
    let mut vertices = Vec::with_capacity(vertex_count + 1);
    let mut indices = Vec::with_capacity(vertices.len() * 3);
    let angle = (360.0 / vertex_count as f32) * (PI / 180.0);

    vertices.push(Vertex {
        position: [0.0, 0.0]
    });

    for i in 1..((vertex_count as VertexIndex) + 1) {
        let angle = i as f32 * angle;
        let x = radius * angle.cos();
        let y = radius * angle.sin();

        vertices.push(Vertex {
            position: [x, y]
        });

        indices.push(0);
        indices.push(i);
        let mut index = (i + 1) % (vertex_count as VertexIndex);
        if index == 0 {
            index = vertex_count as VertexIndex;
        }
        indices.push(index)
    }
    return (vertices, indices);
}


pub fn generate_circle_vertices(slices: i32, radius: f32, center: Vec2) -> Vec<Vertex> {
    let mut vertices: Vec<Vertex> = Vec::with_capacity((slices * 3) as usize);
    let angle_factor = 2.0 * PI / slices as f32;
    let mut vertex_a: Vertex = Vertex { position: [center.x + radius * angle_factor.sin(), center.y + radius * angle_factor.sin()] };
    let mut vertex_b: Vertex = vertex_a;
    let mut finished = false;
    for i in 0..(slices + 1) {
        let angle = i as f32 * angle_factor;
        let vertex_x = center.x + radius * angle.cos();
        let vertex_y = center.y + radius * angle.sin();
        if finished {
            vertex_b = Vertex { position: [vertex_x, vertex_y] };
            finished = false;
        } else {
            vertex_a = Vertex { position: [vertex_x, vertex_y] };
            finished = true;
        }
        vertices.push(Vertex { position: [center.x, center.y] });
        vertices.push(vertex_a);
        vertices.push(vertex_b);
    }
    vertices
}