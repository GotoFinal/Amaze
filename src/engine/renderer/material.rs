use std::mem::size_of;
use std::sync::Arc;
use vulkano::device::Device;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::layout::{PipelineLayoutCreateInfo, PushConstantRange};
use vulkano::render_pass::{RenderPass, Subpass};
use vulkano::shader::{ShaderModule, ShaderStage};
use crate::engine::renderer::renderer::{ShaderObjectData, Vertex};
use crate::GraphicEngine;

// TODO: can i use something simpler than mat4 for simple 2d game? how do i calc perspective correctly?
mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
#version 450

layout(location = 0) in vec2 position;
layout(push_constant) uniform constants
{
	mat4 matrix;
} PushConstants;

void main() {
    gl_Position = PushConstants.matrix * vec4(position, 0.0, 1.0);
}
"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
#version 450

layout(location = 0) out vec4 f_color;

void main() {
    f_color = vec4((cos(normalize(gl_FragCoord.xyz) * 100)), 1.0);
}
"
    }
}

// TODO: idk what is material actually made out, so this is probably incorrect, but I imagine a material needs its own shader, so it also means it must be a separate graphic pipeline
// for now i will only use single material, but the struct will be used as point of reference for future code
pub struct Material {
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    pub(crate) graphic_pipeline: Arc<GraphicsPipeline>,
    pub(crate) pipeline_layout: Arc<PipelineLayout>,
}


pub fn load_default_material(graphic_engine: &mut GraphicEngine) -> Material {
    let vertex_shader =
        vertex_shader::load(graphic_engine.device.clone()).expect("failed to create shader module");
    let fragment_shader = fragment_shader::load(graphic_engine.device.clone())
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
    let layout = PipelineLayout::new(graphic_engine.device.clone(), mesh_pipeline_layout_info)
        .expect("failed to create layout instance");

    let pipeline = get_pipeline(
        graphic_engine.device.clone(),
        vertex_shader.clone(),
        fragment_shader.clone(),
        graphic_engine.render_pass.clone(),
        layout.clone(),
        graphic_engine.viewport.clone(),
    );

    return Material {
        vertex_shader,
        fragment_shader,
        graphic_pipeline: pipeline,
        pipeline_layout: layout,
    }
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
        .with_pipeline_layout(device, pipeline_layout)
        .unwrap()
}
