use std::cmp::min;
use std::collections::BTreeMap;
use std::f32::consts::PI;
use std::mem::size_of;
use std::ops::Div;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{IVec2, Vec2, Mat4, Vec3, EulerRot, Quat};
use num_traits::ToPrimitive;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage,
    PrimaryAutoCommandBuffer, SubpassContents,
};
use vulkano::descriptor_set::layout::DescriptorSetLayoutBinding;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType, QueueFamily};
use vulkano::device::DeviceExtensions;
use vulkano::device::{Device, DeviceCreateInfo, DeviceOwned, Queue, QueueCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{
    AttachmentImage, ImageAccess, ImageUsage, SampleCount, SampleCounts, SwapchainImage,
};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::{BuffersDefinition, VertexInputBindingDescription};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::layout::{PipelineLayoutCreateInfo, PushConstantRange};
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::{ShaderModule, ShaderStage, ShaderStages};
use vulkano::swapchain::{
    acquire_next_image, AcquireError, PresentFuture, Surface, Swapchain, SwapchainAcquireFuture,
    SwapchainCreateInfo, SwapchainCreationError,
};
use vulkano::sync::{self, FenceSignalFuture, FlushError, GpuFuture, JoinFuture};
use vulkano_win::VkSurfaceBuild;
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::window::{Window, WindowBuilder};

use super::gamesync::GameSync;

pub mod primitives;

// TODO: wanted to split code to keep it more clean and this file is already a mess

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 2],
}

vulkano::impl_vertex!(Vertex, position);

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
struct Material {
    vertex_shader: Arc<ShaderModule>,
    fragment_shader: Arc<ShaderModule>,
    graphic_pipeline: Arc<GraphicsPipeline>,
    pipeline_layout: Arc<PipelineLayout>,
}

#[derive(Debug, Clone, Copy)]
pub enum Buffering {
    Double,
    Triple,
}

#[derive(Debug, Clone, Copy)]
pub enum Multisampling {
    Disable,
    Sample2,
    Sample4,
    Sample8,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphicOptions {
    pub multisampling: Multisampling,
    pub buffering: Buffering,
}

pub(crate) trait Renderer {
    fn init(options: GraphicOptions, event_loop: &mut EventLoop<()>) -> Self;

    fn validate(&mut self);

    fn buffer_size(&self) -> usize;

    fn rebuild(&mut self);

    fn on_resize(&mut self, new_size: PhysicalSize<u32>);

    fn render(&mut self, gamesync: &mut GameSync);

    fn create_graphic_object(&mut self, desc: GraphicObjectDesc) -> u32;

    fn move_object(&mut self, index: u32, pos: Vec2);

    fn translate_position(&self, position: Vec2) -> Vec2;
}

#[derive(Debug, Clone)]
pub struct GraphicObjectDesc {
    pub position: Vec2,
    pub vertices: Vec<Vertex>,
}

// TODO: how do i store the actual stuff to render? for later:
// - create separate struct for objects that consist of vertices and later more stuff
// - process it into one big buffer i guess?
// - should original data become just slices?
// - but will rust even allow that?
// - but do I want that data? i could push it to GPU and forget i guess?
// - but for now i plan 2d game so do i care?
#[derive(Debug, Clone)]
struct TheStuffToRender {
    position: Vec2,
    vertices: Vec<Vertex>,
    vertices_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>, // thats bascially the same data..., but lets think about it later
                                                         // TODO: material i guess?
}

pub trait GraphicObject {
    fn set_position(&mut self, pos: Vec2);
}

impl GraphicObject for TheStuffToRender {
    fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }
}

// TODO: can i just bind the lifetime of all this shit to the GraphicEngine?
pub struct GraphicEngine {
    options: GraphicOptions,
    instance: Arc<Instance>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    viewport: Viewport,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>, // TODO: what they belong to?
    swapchain: Arc<Swapchain<Window>>,
    surface: Arc<Surface<Window>>,
    materials: Option<Material>, // because who needs more than one? TODO: or something
    objects: Vec<TheStuffToRender>, // yes
    window_resized: bool,
    recreate_swapchain: bool,
    old_size: PhysicalSize<u32>,
    images: Vec<Arc<SwapchainImage<Window>>>,
    framebuffers: Vec<Arc<Framebuffer>>, // TODO: understand how the hell &' works, do i need it? do i want it? E,
    frame: u64,
}

impl Renderer for GraphicEngine {
    // TODO: this is all passed by value, is that ok?
    // TODO: creating and adding probably should be separate operations
    // TODO: reduce copies?
    fn create_graphic_object(&mut self, desc: GraphicObjectDesc) -> u32 {
        // if (self.objects.is_some()) {
        //     let object = self.objects.clone().unwrap();
        //     if let Ok(mut x) = object.vertices_buffer.write() {
        //         x.copy_from_slice(desc.vertices.as_slice())
        //     } else {
        //         println!("Welp...")
        //     }
        //     return
        // }
        let cp = desc.vertices.clone();
        let vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>> = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::vertex_buffer(),
            false,
            desc.vertices.into_iter(),
        )
        .unwrap();
        let result = TheStuffToRender {
            position: desc.position,
            vertices: cp,
            vertices_buffer: vertex_buffer,
        };
        self.objects.push(result);
        generate_command_buffers(self);
        return self.objects.len() as u32;
    }

    // fn init(options: GraphicOptions, event_loop: &mut EventLoop<()>) -> GraphicEngine {
    fn init(options: GraphicOptions, event_loop: &mut EventLoop<()>) -> Self {
        // can this even work?
        let required_extensions = vulkano_win::required_extensions();
        let instance = Instance::new(InstanceCreateInfo {
            enabled_extensions: required_extensions,
            ..Default::default()
        })
        .expect("failed to create instance");

        // this is wrong place for this salmfkjbkjresbrbkle aaaaa
        let surface = WindowBuilder::new()
            .with_transparent(true)
            // .with_decorations(false)
            .with_resizable(true)
            .with_min_inner_size(adjust_physical_side(
                PhysicalSize::new(400, 400),
                PhysicalSize::new(400, 400),
            ))
            .with_title("Game or something idk yet")
            .build_vk_surface(&event_loop, instance.clone())
            .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };
        let (physical_device, queue_family) =
            select_physical_device(&instance, surface.clone(), &device_extensions);

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                // here we pass the desired queue families that we want to use
                queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
                enabled_extensions: physical_device
                    .required_extensions()
                    .union(&device_extensions),
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let queue = queues.next().unwrap();

        let caps = physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        let dimensions = surface.window().inner_size();
        let composite_alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let image_format = Some(
            physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        let target_image_count = match options.buffering {
            Buffering::Double => 2,
            Buffering::Triple => 3,
        };
        let image_count = min(target_image_count, caps.min_image_count);
        println!("Creating swapchain with {} images", image_count);
        let (mut swapchain, images): (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) =
            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: image_count,
                    image_format,
                    image_extent: dimensions.into(),
                    image_usage: ImageUsage::color_attachment(), // What the images are going to be used for
                    composite_alpha,
                    ..Default::default()
                },
            )
            .unwrap();

        let color_samples = physical_device.properties().framebuffer_color_sample_counts;
        let depth_samples = physical_device.properties().framebuffer_depth_sample_counts;
        let max_samples: vulkano::image::SampleCounts =
            combine_sample_counts(color_samples, depth_samples);
        let sample_count = get_sample_count(options.multisampling, max_samples);

        let render_pass = get_render_pass(device.clone(), swapchain.clone(), sample_count);
        let framebuffers = get_framebuffers(&images, render_pass.clone());

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: surface.window().inner_size().into(),
            depth_range: 0.0..1.0,
        };

        // aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa this is bad
        let mut engine = GraphicEngine {
            // TODO: asnkjdnveksf how do I materials
            instance: instance,
            options: options,
            viewport: viewport,
            device: device,
            queue: queue,
            render_pass: render_pass,
            materials: None,
            objects: Vec::new(),
            command_buffers: Vec::new(),
            framebuffers: framebuffers,
            swapchain: swapchain,
            surface: surface.clone(),
            window_resized: false,
            recreate_swapchain: false,
            old_size: surface.window().inner_size(),
            images: images,
            frame: 0
        };
        load_materials(&mut engine);
        return engine;
    }

    fn rebuild(&mut self) {
        self.recreate_swapchain = false;

        let new_dimensions = self.surface.window().inner_size();

        // TODO: does rust care about me referencing this multiple times? like self.swapchain
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: new_dimensions.into(),
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };
        self.swapchain = new_swapchain;
        self.images = new_images;
        self.framebuffers = get_framebuffers(self.images.as_ref(), self.render_pass.clone());

        if self.window_resized {
            self.window_resized = false;
            self.viewport.dimensions = new_dimensions.into();
            load_materials(self);
            generate_command_buffers(self);
        }
    }

    fn on_resize(&mut self, new_size: PhysicalSize<u32>) {
        // TODO: is new size needed? can we just get it from window?
        let adjusted_size = adjust_physical_side(new_size, self.old_size);
        if new_size != adjusted_size {
            self.surface.window().set_inner_size(adjusted_size);
            self.old_size = adjusted_size;
        }
        self.window_resized = true;
    }

    fn validate(&mut self) {
        if self.window_resized || self.recreate_swapchain {
            self.rebuild()
        }
    }

    fn buffer_size(&self) -> usize {
        self.images.len()
    }

    // TODO: if i want to join everything in single spot I need to be able to run render loop once and return future?
    // or i can pass the future as argument, but why would render engine need to have access to that
    // maybe i should know rust or something
    fn render(&mut self, gamesync: &mut GameSync) {
        self.frame += 1;
        create_command_buffers(self);
        let (image_i, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        gamesync.set_current(image_i);
        if suboptimal {
            self.recreate_swapchain = true
        }

        if let Some(image_fence) = gamesync.get_current() {
            image_fence.wait(None).unwrap();
        }
        let previous_future = match gamesync.get_prev().clone() {
            // Create a NowFuture
            None => {
                let mut now = sync::now(self.device.clone());
                now.cleanup_finished();

                now.boxed()
            }
            // Use the existing FenceSignalFuture
            Some(fence) => fence.boxed(),
        };

        let future = previous_future
            .join(acquire_future)
            .then_execute(self.queue.clone(), self.command_buffers[image_i].clone())
            .unwrap()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_i)
            .then_signal_fence_and_flush();

        gamesync.update_fence(match future {
            Ok(value) => Some(Arc::new(value)),
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                None
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                None
            }
        });
    }

    fn translate_position(&self, position: Vec2) -> Vec2 {

        let size = self.surface.window().inner_size();
        let surface_size: Vec2 = Vec2::new(size.width as f32, size.height as f32);
        let aspect = surface_size.x / surface_size.y;
        let x = (position.x / (surface_size.x / (aspect * 2.0))) - aspect;
        let y = (position.y / (surface_size.y / 2.0)) - 1.0;
        return Vec2::new(x, y);
    }

    fn move_object(&mut self, index: u32, pos: Vec2) {
        self.objects[index as usize].set_position(pos);
    }
}

fn load_materials(graphic_engine: &mut GraphicEngine) {
    let vertex_shader =
        vertex_shader::load(graphic_engine.device.clone()).expect("failed to create shader module");
    let fragment_shader = fragment_shader::load(graphic_engine.device.clone())
        .expect("failed to create shader module");

    // i have no idea what im doing
    let mut mesh_pipeline_layout_info = PipelineLayoutCreateInfo::default();
    let push_constant = PushConstantRange {
        stages: ShaderStage::Vertex.into(),
        offset: 0,
        size: size_of::<ShaderDataOrSomething>() as u32,
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

    graphic_engine.materials = Some(Material {
        vertex_shader: vertex_shader,
        fragment_shader: fragment_shader,
        graphic_pipeline: pipeline,
        pipeline_layout: layout,
    });
}

fn generate_command_buffers(graphic_engine: &mut GraphicEngine) {
    if graphic_engine.materials.is_none() || graphic_engine.objects.is_empty() {
        println!("Nothing to render so i will spam instead");
        if !graphic_engine.command_buffers.is_empty() {
            graphic_engine.command_buffers = Vec::new()
        }
        return;
    }
    create_command_buffers(graphic_engine);
}

fn combine_sample_counts(a: SampleCounts, b: SampleCounts) -> SampleCounts {
    return SampleCounts {
        sample1: a.sample1 & b.sample1,
        sample2: a.sample2 & b.sample2,
        sample4: a.sample4 & b.sample4,
        sample8: a.sample8 & b.sample8,
        sample16: a.sample16 & b.sample16,
        sample32: a.sample32 & b.sample32,
        sample64: a.sample64 & b.sample64,
    };
}

fn select_physical_device<'a>(
    instance: &'a Arc<Instance>,
    surface: Arc<Surface<Window>>,
    device_extensions: &DeviceExtensions,
) -> (PhysicalDevice<'a>, QueueFamily<'a>) {
    let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
        .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
        .filter_map(|p| {
            p.queue_families()
                .find(|&q| q.supports_graphics() && q.supports_surface(&surface).unwrap_or(false))
                .map(|q| (p, q))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
        })
        .expect("no device available");
    (physical_device, queue_family)
}

// aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, there must be better way
fn get_sample_count(sample: Multisampling, max_samples: SampleCounts) -> SampleCount {
    let mut vulkan_sample = match sample {
        Multisampling::Disable => SampleCount::Sample1,
        Multisampling::Sample2 => SampleCount::Sample2,
        Multisampling::Sample4 => SampleCount::Sample4,
        Multisampling::Sample8 => SampleCount::Sample8,
    };
    if vulkan_sample == SampleCount::Sample1 {
        return SampleCount::Sample1;
    }
    if vulkan_sample == SampleCount::Sample8 && !max_samples.sample8 {
        vulkan_sample = SampleCount::Sample4;
    }
    if vulkan_sample == SampleCount::Sample4 && !max_samples.sample4 {
        vulkan_sample = SampleCount::Sample2;
    }
    if vulkan_sample == SampleCount::Sample2 && !max_samples.sample2 {
        return SampleCount::Sample1;
    }
    return vulkan_sample;
}

fn get_framebuffers(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            let intermediary = ImageView::new_default(
                AttachmentImage::transient_multisampled(
                    render_pass.device().clone(),
                    view.image().dimensions().width_height(),
                    SampleCount::Sample2,
                    image.format(),
                )
                .unwrap(),
            )
            .unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![intermediary, view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}

fn get_render_pass(
    device: Arc<Device>,
    swapchain: Arc<Swapchain<Window>>,
    sample: SampleCount,
) -> Arc<RenderPass> {
    match sample {
        SampleCount::Sample1 => vulkano::single_pass_renderpass!(
            device.clone(),
              attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap(),
        _ => vulkano::single_pass_renderpass!(
            device.clone(),
              attachments: {
                intermediary: {
                    load: Clear,
                    store: DontCare,
                    format: swapchain.image_format(),
                    samples: sample as u32,
                },
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                }
            },
            pass: {
                color: [intermediary],
                depth_stencil: {},
                resolve: [color]
            }
        )
        .unwrap(),
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

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct ShaderDataOrSomething {
    matrix: Mat4,
}

fn create_command_buffers(graphic_engine: &mut GraphicEngine) {
    let object = &graphic_engine.objects[0]; // TODO, support more objects
    let material = graphic_engine.materials.as_ref().unwrap();
    let pipeline = material.graphic_pipeline.clone();
    let vertex_buffer = object.vertices_buffer.clone();

    let dimensions: Vec2 = graphic_engine.viewport.dimensions.into();

    // TODO: simplify
    let cam_pos = Vec3::new(0.0, 0.0, -1.0);
    let view = Mat4::look_at_lh(cam_pos, Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
    let aspect = dimensions.x / dimensions.y;
    let orto = Mat4::orthographic_lh(-aspect, aspect, -1.0, 1.0, 0.1, 1.1);
    let mut projection = orto; // Mat4::perspective_infinite_lh(70.0_f32.to_radians(), dimensions.x / dimensions.y, 0.1);
    let rotation = (graphic_engine.frame as f32 * 0.4).to_radians();
    let quat = Quat::from_axis_angle(Vec3::new(0.1, 0.03, 0.1).normalize(), rotation);
    let model = Mat4::from_scale_rotation_translation(Vec3::ONE, quat, object.position.extend(0.0));
    let matrix = projection * view * model;

    graphic_engine.command_buffers = graphic_engine
        .framebuffers
        .iter()
        .map(|framebuffer| {
            let mut builder = AutoCommandBufferBuilder::primary(
                graphic_engine.device.clone(),
                graphic_engine.queue.family(), // do i not need to clone or vsc is retarded
                CommandBufferUsage::MultipleSubmit, // don't forget to write the correct buffer usage
            )
            .unwrap();

            let layout = material.pipeline_layout.clone();

            builder
                .begin_render_pass(
                    framebuffer.clone(),
                    SubpassContents::Inline,
                    vec![[0.0, 0.0, 0.0, 0.0].into(), [0.0, 0.0, 0.0, 0.0].into()],
                )
                .unwrap()
                .bind_pipeline_graphics(pipeline.clone())
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .push_constants(
                    layout,
                    0,
                    ShaderDataOrSomething {
                        matrix: matrix,
                    },
                )
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass()
                .unwrap();

            Arc::new(builder.build().unwrap())
        })
        .collect()
}

fn adjust_physical_side(size: PhysicalSize<u32>, old_size: PhysicalSize<u32>) -> PhysicalSize<u32> {
    let bigger: bool = size.height > old_size.height || size.width > old_size.width;

    let expected_width = 3;
    let expected_height = 4;

    let width_ratio = size.width as f32 / expected_width as f32;
    let height_ratio = size.height as f32 / expected_height as f32;
    let size_multi = if bigger {
        width_ratio.max(height_ratio)
    } else {
        width_ratio.min(height_ratio)
    }
    .floor() as u32;

    return PhysicalSize::new(size_multi * expected_width, size_multi * expected_height);
}
