use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::mem::size_of;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

use glam::{quat, Mat4, Quat, Vec3};
use vulkano::buffer::TypedBufferAccess;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::device::Device;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::{RasterizationState, PolygonMode};
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::layout::{PipelineLayoutCreateInfo, PushConstantRange};
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::render_pass::{RenderPass, Subpass};
use vulkano::shader::{ShaderModule, ShaderStage};

use crate::engine::renderer::graphic_object::RenderMesh;
use crate::engine::renderer::renderer::{ShaderObjectData, Vertex};
use crate::GraphicEngine;

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 fragColor;
layout(push_constant) uniform constants
{
	mat4 matrix;
    mat4 normal_matrix;
} PushConstants;

const vec3 DIRECTION_TO_LIGHT = normalize(vec3(1.0, 3.0, -1.0));
const float AMBIENT = 0.06;

void main() {
  gl_Position = PushConstants.matrix * vec4(position, 1.0);

  vec3 normalWorldSpace = normalize(mat3(PushConstants.normal_matrix) * normal);

  float lightIntensity = AMBIENT + max(dot(normalWorldSpace, DIRECTION_TO_LIGHT), 0);

  fragColor = lightIntensity * vec3(0.9, 0.8, 1.0);
}
"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
#version 450


layout(location = 0) in vec3 fragColor;
layout(location = 0) out vec4 f_color;

void main() {
    f_color = vec4(fragColor, 1.0);
}
"
    }
}

// TODO: idk what is material actually made out, so this is probably incorrect, but I imagine a material needs its own shader, so it also means it must be a separate graphic pipeline
// for now i will only use single material, but the struct will be used as point of reference for future code

pub struct MaterialData {
    key: MaterialKey,
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    pub(crate) graphic_pipeline: Arc<GraphicsPipeline>,
    pub(crate) pipeline_layout: Arc<PipelineLayout>,
}

type PrimaryCommandBuilder = AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>;
pub type MaterialKey = u16;

pub trait Material {
    fn key(&self) -> MaterialKey;
    fn recreate(&mut self, engine: &GraphicEngine);
    fn draw<'a>(&self, mesh: &RenderMesh, projection_view: Mat4, commands: &'a mut PrimaryCommandBuilder) -> &'a mut PrimaryCommandBuilder;
}

impl Material for MaterialData {
    fn key(&self) -> MaterialKey {
        return self.key;
    }

    fn recreate(&mut self, engine: &GraphicEngine) {
        self.graphic_pipeline = get_pipeline(
            engine.device.clone(),
            self.vertex_shader.clone(),
            self.fragment_shader.clone(),
            engine.render_pass.clone(),
            self.pipeline_layout.clone(),
            engine.viewport.clone(),
        );
    }

    fn draw<'a>(&self, mesh: &RenderMesh, projection_view: Mat4, commands: &'a mut PrimaryCommandBuilder) -> &'a mut PrimaryCommandBuilder {
        let transform = mesh.transform;
        let matrix = projection_view * transform.matrix();

        let indices_count = mesh.data.indices_buffer.len();
        let pipeline = self.graphic_pipeline.clone();
        return commands.bind_pipeline_graphics(pipeline)
            .bind_vertex_buffers(0, mesh.data.vertices_buffer.clone())
            .bind_index_buffer(mesh.data.indices_buffer.clone())
            .push_constants(
                self.pipeline_layout.clone(),
                0,
                ShaderObjectData {
                    matrix,
                    normal_matrix: transform.matrix().inverse(),
                },
            )
            .draw_indexed(indices_count as u32, 1, 0, 0, 0)
            .unwrap()
    }
}

pub struct MaterialRegistry {
    last_key: MaterialKey,
    materials: HashMap<MaterialKey, Rc<RefCell<dyn Material>>>,
}

impl MaterialRegistry {
    pub fn create(device: Arc<Device>, render_pass: Arc<RenderPass>, viewport: Viewport) -> Self {
        let def = load_default_material(device, render_pass, viewport);
        let mut map = HashMap::new();
        map.insert(0, def);
        return MaterialRegistry {
            materials: map,
            last_key: 0,
        };
    }
}

pub trait Materials {
    fn get(&self, key: MaterialKey) -> Rc<RefCell<dyn Material>>;
    fn get_default(&self) -> MaterialKey;
    fn reload(&mut self, engine: &GraphicEngine);
}

impl Materials for MaterialRegistry {
    fn get(&self, key: MaterialKey) -> Rc<RefCell<dyn Material>> {
        return self.materials.get(&key).unwrap().clone();
    }

    fn get_default(&self) -> MaterialKey {
        return 0;
    }

    fn reload(&mut self, engine: &GraphicEngine) {
        for (key, material) in self.materials.iter() {
            let borrow = &mut *material.borrow_mut();
            borrow.recreate(engine)
        }
    }
}

pub fn load_default_material(device: Arc<Device>, render_pass: Arc<RenderPass>, viewport: Viewport) -> Rc<RefCell<dyn Material>> {
    let vertex_shader =
        vertex_shader::load(device.clone()).expect("failed to create shader module");
    let fragment_shader = fragment_shader::load(device.clone())
        .expect("failed to create shader module");

    // i have no idea what im doing
    let mut mesh_pipeline_layout_info = PipelineLayoutCreateInfo::default();
    let push_constant = PushConstantRange {
        stages: ShaderStage::Vertex.into(),
        offset: 0,
        size: size_of::<ShaderObjectData>() as u32,
    };
    mesh_pipeline_layout_info
        .push_constant_ranges
        .push(push_constant);
    let layout = PipelineLayout::new(device.clone(), mesh_pipeline_layout_info)
        .expect("failed to create layout instance");

    let pipeline = get_pipeline(
        device.clone(),
        vertex_shader.clone(),
        fragment_shader.clone(),
        render_pass.clone(),
        layout.clone(),
        viewport,
    );

    return Rc::new(RefCell::new(MaterialData {
        key: 0,
        vertex_shader,
        fragment_shader,
        graphic_pipeline: pipeline,
        pipeline_layout: layout,
    }));
}

fn get_pipeline(
    device: Arc<Device>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    render_pass: Arc<RenderPass>,
    pipeline_layout: Arc<PipelineLayout>,
    viewport: Viewport,
) -> Arc<GraphicsPipeline> {
    GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .rasterization_state(RasterizationState { polygon_mode: PolygonMode::Fill, ..Default::default() })
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .with_pipeline_layout(device, pipeline_layout)
        .unwrap()
}
