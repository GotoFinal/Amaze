use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec2, Vec3};
use vulkano::buffer::{TypedBufferAccess};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage,
    PrimaryAutoCommandBuffer, SubpassContents,
};
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo};
use vulkano::device::DeviceExtensions;
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, RenderPass};
use vulkano::swapchain::{
    acquire_next_image, AcquireError, Surface, Swapchain,
    SwapchainCreateInfo, SwapchainCreationError,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano_win::VkSurfaceBuild;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::engine::renderer::graphic_object::{GraphicObject, GraphicObjectDesc, TheStuffToRender};
use crate::engine::renderer::material::{load_default_material, Material};
use crate::engine::renderer::options::GraphicOptions;
use crate::engine::renderer::vulkan::{combine_sample_counts, create_swapchain, get_framebuffers, get_render_pass, get_sample_count, select_physical_device};
use crate::GameSync;

// TODO: wanted to split code to keep it more clean and this file is already a mess

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct ShaderObjectData {
    pub matrix: Mat4,
}

vulkano::impl_vertex!(Vertex, position);

pub(crate) trait Renderer {
    fn init(options: GraphicOptions, event_loop: &EventLoop<()>) -> Self;

    fn validate(&mut self);

    fn rebuild(&mut self);

    fn on_resize(&mut self, new_size: PhysicalSize<u32>);

    fn render(&mut self);

    fn create_graphic_object(&mut self, desc: GraphicObjectDesc) -> u32;

    fn move_object(&mut self, index: u32, pos: Vec2);

    fn translate_position(&self, position: Vec2) -> Vec2;

    fn get_sync(&self) -> &GameSync;
}

pub struct GraphicEngine {
    options: GraphicOptions,
    instance: Arc<Instance>,
    pub(crate) device: Arc<Device>,
    queue: Arc<Queue>,
    pub(crate) render_pass: Arc<RenderPass>,
    pub(crate) viewport: Viewport,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
    // TODO: what they belong to?
    swapchain: Arc<Swapchain<Window>>,
    surface: Arc<Surface<Window>>,
    materials: Option<Material>,
    // because who needs more than one? TODO: or something
    objects: Vec<TheStuffToRender>,
    window_resized: bool,
    recreate_swapchain: bool,
    old_size: PhysicalSize<u32>,
    images: Vec<Arc<SwapchainImage<Window>>>,
    framebuffers: Vec<Arc<Framebuffer>>,
    sync: GameSync,
    frame: u64,
}

impl Renderer for GraphicEngine {
    // TODO: this is all passed by value, is that ok?
    // TODO: creating and adding probably should be separate operations
    // TODO: reduce copies?
    fn create_graphic_object(&mut self, desc: GraphicObjectDesc) -> u32 {
        let object = GraphicObject::create(desc, self);
        self.objects.push(object);
        Self::generate_command_buffers(self);
        return self.objects.len() as u32;
    }

    // fn init(options: GraphicOptions, event_loop: &mut EventLoop<()>) -> GraphicEngine {
    fn init(options: GraphicOptions, event_loop: &EventLoop<()>) -> Self {
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
            .with_min_inner_size(Self::adjust_physical_side(
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

        let (swapchain, images) = create_swapchain(options, &surface, physical_device, &device);

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

        let images_size = images.len();

        // aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa this is bad
        let mut engine = GraphicEngine {
            instance,
            options,
            viewport,
            device,
            queue,
            render_pass,
            materials: None,
            objects: Vec::new(),
            command_buffers: Vec::new(),
            framebuffers,
            swapchain,
            surface: surface.clone(),
            window_resized: false,
            recreate_swapchain: false,
            old_size: surface.window().inner_size(),
            images,
            sync: GameSync::new(images_size),
            frame: 0,
        };
        Self::load_materials(&mut engine);
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
            Self::load_materials(self);
            Self::generate_command_buffers(self);
        }
    }

    fn on_resize(&mut self, new_size: PhysicalSize<u32>) {
        // TODO: is new size needed? can we just get it from window?
        let adjusted_size = Self::adjust_physical_side(new_size, self.old_size);
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

    // TODO: if i want to join everything in single spot I need to be able to run render loop once and return future?
    // or i can pass the future as argument, but why would render engine need to have access to that
    // maybe i should know rust or something
    fn render(&mut self) {
        self.frame += 1;
        Self::create_command_buffers(self);
        let (image_i, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        self.sync.set_current(image_i);
        if suboptimal {
            self.recreate_swapchain = true
        }

        if let Some(image_fence) = self.sync.get_current() {
            image_fence.wait(None).unwrap();
        }
        let previous_future = match self.sync.get_prev().clone() {
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

        self.sync.update_fence(match future {
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

    fn get_sync(&self) -> &GameSync {
        return &self.sync
    }
}

impl GraphicEngine {
    fn load_materials(graphic_engine: &mut GraphicEngine) {
        graphic_engine.materials = Some(load_default_material(graphic_engine));
    }

    fn generate_command_buffers(graphic_engine: &mut GraphicEngine) {
        if graphic_engine.materials.is_none() || graphic_engine.objects.is_empty() {
            println!("Nothing to render so i will spam instead");
            if !graphic_engine.command_buffers.is_empty() {
                graphic_engine.command_buffers = Vec::new()
            }
            return;
        }
        Self::create_command_buffers(graphic_engine);
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

        let transform = object.transform;

        // let rotation = (graphic_engine.frame as f32 * 0.4).to_radians();
        let quat = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), transform.rotation);
        let model = Mat4::from_scale_rotation_translation(transform.scale.extend(0.0), quat, transform.position.extend(0.0));
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
                        ShaderObjectData {
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
}
