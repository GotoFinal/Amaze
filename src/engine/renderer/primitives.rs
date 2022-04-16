use std::f32::consts::PI;
use glam::Vec2;
use crate::engine::renderer::renderer::Vertex;

pub fn generate_circle_vertices(slices: i32, radius: f32, center: Vec2) -> Vec<Vertex> {
    let mut vertices: Vec<Vertex> = Vec::with_capacity((slices * 3) as usize);
    let angle_factor = 2.0 * PI / slices as f32;
    let mut vertex_a: Vertex = Vertex { position: [ center.x + radius * angle_factor.sin(),  center.y + radius * angle_factor.sin()] };
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
        vertices.push( Vertex { position: [center.x, center.y] });
        vertices.push(vertex_a);
        vertices.push(vertex_b);
    }
    vertices
}