use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use bevy_ecs::prelude::Entity;
use bevy_ecs::world::EntityRef;

use bytemuck::{Pod, Zeroable};
use glam::{EulerRot, Mat4, Quat, Vec2, Vec3};
use vulkano::buffer::TypedBufferAccess;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage,
    PrimaryAutoCommandBuffer, SubpassContents,
};
use vulkano::device::{Device, DeviceCreateInfo, Features, Queue, QueueCreateInfo};
use vulkano::device::DeviceExtensions;
use vulkano::device::physical::PhysicalDevice;
use vulkano::format::ClearValue;
use vulkano::format::ClearValue::Depth;
use vulkano::image::{SampleCount, SampleCounts, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, RenderPass};
use vulkano::swapchain::{
    acquire_next_image, AcquireError, Surface, Swapchain,
    SwapchainCreateInfo, SwapchainCreationError,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano_win::{create_surface_from_winit, VkSurfaceBuild};
use winit::dpi::PhysicalSize;
use winit::event::VirtualKeyCode::{A, B};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::engine::renderer::graphic_object::{GraphicObject, GraphicObjectDesc, RenderMesh, RenderMeshData};
use crate::engine::renderer::material::{load_default_material, MaterialRegistry, Materials};
use crate::engine::renderer::options::{GraphicOptions, Multisampling};
use crate::engine::renderer::vulkan::{combine_sample_counts, create_swapchain, get_framebuffers, get_render_pass, get_sample_count, select_physical_device};
use crate::{Camera, GameSync, Mesh, Transform};
use crate::engine::renderer::camera::RendererCamera;
use crate::engine::renderer::options::Multisampling::Disable;

// TODO: wanted to split code to keep it more clean and this file is already a mess

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

pub type VertexIndex = u16;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct ShaderObjectData {
    pub matrix: Mat4,
    // pub normal_matrix: Mat4,
    pub id: u32
}

vulkano::impl_vertex!(Vertex, position, normal);

pub(crate) trait Renderer {
    fn init(options: GraphicOptions, window: Arc<Window>) -> Self;

    fn should_render(&self) -> bool;

    fn validate(&mut self);

    fn rebuild(&mut self);

    fn on_resize(&mut self, new_size: PhysicalSize<u32>);

    fn render(&mut self);

    fn create_graphic_object(&mut self, desc: GraphicObjectDesc) -> u32;

    fn move_object(&mut self, index: u32, pos: Vec3);

    fn translate_position(&self, position: Vec2) -> Vec2;

    fn get_sync(&self) -> &GameSync;
}

pub struct GraphicEngine {
    options: GraphicOptions,
    instance: Arc<Instance>,
    pub(crate) device: Arc<Device>,
    physical_device_properties: PhysicalDeviceProperties,
    queue: Arc<Queue>,
    pub(crate) render_pass: Arc<RenderPass>,
    pub(crate) camera: RendererCamera,
    pub(crate) viewport: Viewport,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
    // TODO: what they belong to?
    swapchain: Arc<Swapchain<Arc<Window>>>,
    pub(crate) surface: Arc<Surface<Arc<Window>>>,
    pub(crate) materials: Arc<RefCell<dyn Materials>>,
    // because who needs more than one? TODO: or something
    objects: Vec<RenderMesh>,
    window_resized: bool,
    recreate_swapchain: bool,
    old_size: PhysicalSize<u32>,
    images: Vec<Arc<SwapchainImage<Arc<Window>>>>,
    framebuffers: Vec<Arc<Framebuffer>>,
    sync: GameSync,
    frame: u64,
    clear_values: Vec<ClearValue>,
    pub(crate) mesh_cache: RefCell<HashMap<u32, RenderMeshData>>,
}

#[derive(Debug, Default, Copy, Clone)]
struct PhysicalDeviceProperties {
    color_samples: SampleCounts,
    depth_samples: SampleCounts,
    max_samples: SampleCounts,
}

impl Renderer for GraphicEngine {
    // TODO: this is all passed by value, is that ok?
    // TODO: creating and adding probably should be separate operations
    // TODO: reduce copies?
    fn create_graphic_object(&mut self, desc: GraphicObjectDesc) -> u32 {
        let object = GraphicObject::create(desc, self);
        self.objects.push(object);
        return self.objects.len() as u32 - 1;
    }

    // fn init(options: GraphicOptions, event_loop: &mut EventLoop<()>) -> GraphicEngine {
    fn init(options: GraphicOptions, window: Arc<Window>) -> Self {
        // can this even work?
        let required_extensions = vulkano_win::required_extensions();
        let instance = Instance::new(InstanceCreateInfo {
            enabled_extensions: required_extensions,
            ..Default::default()
        })
            .expect("failed to create instance");

        let surface = create_surface_from_winit(window, instance.clone()).unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };
        let (physical_device, queue_family) =
            select_physical_device(&instance, surface.clone(), &device_extensions);

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
                enabled_extensions: physical_device
                    .required_extensions()
                    .union(&device_extensions),
                enabled_features: Features { fill_mode_non_solid: true, ..Features::none() },
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
        let physical_device_properties = PhysicalDeviceProperties {
            color_samples,
            depth_samples,
            max_samples,
        };
        let sample_count = get_sample_count(options.multisampling, max_samples);

        let render_pass = get_render_pass(device.clone(), swapchain.clone(), sample_count);
        let framebuffers = get_framebuffers(device.clone(), &images, render_pass.clone(), sample_count);

        let size = surface.window().inner_size();
        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: size.into(),
            depth_range: 0.0..1.0,
        };
        let aspect_ratio = size.width as f32 / size.height as f32;

        let images_size = images.len();
        let materials = Arc::new(RefCell::new(MaterialRegistry::create(device.clone(), render_pass.clone(), viewport.clone())));

        // aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa this is bad
        let mut engine = GraphicEngine {
            instance,
            options,
            viewport,
            device,
            physical_device_properties,
            queue,
            render_pass,
            camera: RendererCamera::create(Vec3::ZERO,
                                           Camera {
                                               aspect_ratio: aspect_ratio,
                                               far_clip_plane: 100.0,
                                               near_clip_plane: 0.1,
                                               field_of_view: 70.0,
                                           },
            ),
            materials,
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
            clear_values: Vec::new(),
            mesh_cache: RefCell::new(HashMap::new()),
        };
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
        self.framebuffers = get_framebuffers(self.device.clone(), self.images.as_ref(), self.render_pass.clone(), Self::get_samples(self.physical_device_properties, self.options));

        // TODO: this should be remembered once when recreating frame buffers and then re-used each frame
        let mut clear_values: Vec<ClearValue> = Vec::with_capacity(3);
        clear_values.push([0.0, 0.0, 0.0, 1.0].into());
        if self.options.multisampling != Disable {
            clear_values.push([0.0, 0.0, 0.0, 1.0].into());
        }
        clear_values.push(Depth(1.0));
        self.clear_values = clear_values;

        if self.window_resized {
            self.window_resized = false;
            self.viewport.dimensions = new_dimensions.into();
            self.materials.borrow_mut().reload(self);
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

    fn should_render(&self) -> bool {
        // TODO: invalid surface flag to avoid this call? does it matter? probably not?
        let size = self.surface.window().inner_size();
        return size.height != 0 && size.width != 0;
    }

    #[profiling::function]
    fn validate(&mut self) {
        if self.window_resized || self.recreate_swapchain {
            if self.should_render() {
                self.rebuild()
            }
        }
    }

    #[profiling::function]
    fn render(&mut self) {
        if !self.should_render() {
            return;
        }
        self.frame += 1;
        Self::record_command_buffers(self, self.sync.get_current_i());
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

    fn move_object(&mut self, index: u32, pos: Vec3) {
        self.objects[index as usize].set_position(pos);
    }

    fn get_sync(&self) -> &GameSync {
        return &self.sync;
    }
}

impl GraphicEngine {
    pub(crate) fn get_cached_mesh(&self, id: u32) -> Option<RenderMeshData> {
        return self.mesh_cache.borrow().get(&id).cloned();
    }

    fn get_samples(physical_device_properties: PhysicalDeviceProperties, options: GraphicOptions) -> SampleCount {
        return get_sample_count(options.multisampling, physical_device_properties.max_samples);
    }

    fn generate_command_buffers(graphic_engine: &mut GraphicEngine) {
        if graphic_engine.objects.is_empty() {
            println!("Nothing to render so i will spam instead");
            if !graphic_engine.command_buffers.is_empty() {
                graphic_engine.command_buffers = Vec::new()
            }
            return;
        }
        Self::create_command_buffers(graphic_engine);
    }

    fn create_command_buffers(graphic_engine: &mut GraphicEngine) {
        let projection_view = graphic_engine.camera.projection * graphic_engine.camera.view;
        graphic_engine.command_buffers = graphic_engine
            .framebuffers
            .iter()
            .map(|framebuffer| {
                Self::record_command_buffer(&graphic_engine, projection_view, framebuffer)
            })
            .collect()
    }

    #[profiling::function]
    fn record_command_buffers(graphic_engine: &mut GraphicEngine, image_i: usize) {
        let projection_view = graphic_engine.camera.projection * graphic_engine.camera.view;
        let framebuffer = graphic_engine.framebuffers.get(image_i).unwrap();
        graphic_engine.command_buffers[image_i] = Self::record_command_buffer(&graphic_engine, projection_view, framebuffer)
    }

    fn record_command_buffer(graphic_engine: &&mut GraphicEngine, projection_view: Mat4, framebuffer: &Arc<Framebuffer>) -> Arc<PrimaryAutoCommandBuffer> {
        let mut builder = AutoCommandBufferBuilder::primary(
            graphic_engine.device.clone(),
            graphic_engine.queue.family(), // do i not need to clone or vsc is retarded
            CommandBufferUsage::OneTimeSubmit, // don't forget to write the correct buffer usage
        )
            .unwrap();
        let clear_values = &graphic_engine.clear_values;

        let mut commands: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> = builder
            .begin_render_pass(
                framebuffer.clone(),
                SubpassContents::Inline,
                clear_values.iter().copied(),
            )
            .unwrap();

        Self::render_objects(&graphic_engine, projection_view, commands);

        commands.end_render_pass().unwrap();

        Arc::new(builder.build().unwrap())
    }

    #[profiling::function]
    fn render_objects(graphic_engine: &&&mut GraphicEngine, projection_view: Mat4, mut commands: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        let materials = graphic_engine.materials.borrow();
        let material = materials.get(0);
        let whatever = material.borrow();
        for object in &graphic_engine.objects {
            let material = object.material;
            commands =  whatever.draw(object, projection_view, commands)
        }
    }

    fn adjust_physical_side(size: PhysicalSize<u32>, old_size: PhysicalSize<u32>) -> PhysicalSize<u32> {
        return size;
        // we no longer care about aspect ratio, but for future reference:
        // let bigger: bool = size.height > old_size.height || size.width > old_size.width;
        //
        // let expected_width = 3;
        // let expected_height = 4;
        //
        // let width_ratio = size.width as f32 / expected_width as f32;
        // let height_ratio = size.height as f32 / expected_height as f32;
        // let size_multi = if bigger {
        //     width_ratio.max(height_ratio)
        // } else {
        //     width_ratio.min(height_ratio)
        // }
        //     .floor() as u32;
        //
        // return PhysicalSize::new(size_multi * expected_width, size_multi * expected_height);
    }
}
