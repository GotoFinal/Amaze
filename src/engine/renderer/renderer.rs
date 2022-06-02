use std::cell::{Ref, RefCell};
use std::collections::HashMap;
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
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo};
use vulkano::device::DeviceExtensions;
use vulkano::device::physical::PhysicalDevice;
use vulkano::image::{SampleCount, SampleCounts, SwapchainImage};
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

use crate::engine::renderer::graphic_object::{GraphicObject, GraphicObjectDesc, RenderMesh, RenderMeshData};
use crate::engine::renderer::material::{load_default_material, MaterialRegistry, Materials};
use crate::engine::renderer::options::GraphicOptions;
use crate::engine::renderer::vulkan::{combine_sample_counts, create_swapchain, get_framebuffers, get_render_pass, get_sample_count, select_physical_device};
use crate::{Camera, GameSync, Mesh, Transform};
use crate::engine::renderer::camera::RendererCamera;

// TODO: wanted to split code to keep it more clean and this file is already a mess

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 3],
}

pub type VertexIndex = u16;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct ShaderObjectData {
    pub matrix: Mat4,
}

vulkano::impl_vertex!(Vertex, position);

pub(crate) trait Renderer {
    fn init(options: GraphicOptions, event_loop: &EventLoop<()>) -> Self;

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
    swapchain: Arc<Swapchain<Window>>,
    surface: Arc<Surface<Window>>,
    pub(crate) materials: Rc<RefCell<dyn Materials>>,
    // because who needs more than one? TODO: or something
    objects: Vec<RenderMesh>,
    window_resized: bool,
    recreate_swapchain: bool,
    old_size: PhysicalSize<u32>,
    images: Vec<Arc<SwapchainImage<Window>>>,
    framebuffers: Vec<Arc<Framebuffer>>,
    sync: GameSync,
    frame: u64,
    pub(crate) mesh_cache: RefCell<HashMap<u32, RenderMeshData>>
}

#[derive(Debug, Default, Copy, Clone)]
struct PhysicalDeviceProperties {
    color_samples: SampleCounts,
    depth_samples: SampleCounts,
    max_samples: SampleCounts
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
        let physical_device_properties = PhysicalDeviceProperties {
            color_samples, depth_samples, max_samples
        };
        let sample_count = get_sample_count(options.multisampling, max_samples);

        let render_pass = get_render_pass(device.clone(), swapchain.clone(), sample_count);
        let framebuffers = get_framebuffers(&images, render_pass.clone(), sample_count);

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: surface.window().inner_size().into(),
            depth_range: 0.0..1.0,
        };

        let images_size = images.len();
        let materials = Rc::new(RefCell::new(MaterialRegistry::create(device.clone(), render_pass.clone(), viewport.clone())));

        // aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa this is bad
        let mut engine = GraphicEngine {
            instance,
            options,
            viewport,
            device,
            physical_device_properties,
            queue,
            render_pass,
            camera: RendererCamera {
                camera: Camera {
                    far_clip_plane: 100.0,
                    near_clip_plane: 0.1,
                    field_of_view: 90.0
                },
                transform: Transform::at(Vec3::ZERO)
            },
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
            mesh_cache: RefCell::new(HashMap::new())
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
        self.framebuffers = get_framebuffers(self.images.as_ref(), self.render_pass.clone(), Self::get_samples(self.physical_device_properties, self.options));

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
        return size.height != 0 && size.width != 0
    }

    fn validate(&mut self) {
        if self.window_resized || self.recreate_swapchain {
            if self.should_render() {
                self.rebuild()
            }
        }
    }

    fn render(&mut self) {
        if !self.should_render() {
            return
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
        let projection_view = Self::create_projection_view(graphic_engine);
        graphic_engine.command_buffers = graphic_engine
            .framebuffers
            .iter()
            .map(|framebuffer| {
                Self::record_command_buffer(&graphic_engine, projection_view, framebuffer)
            })
            .collect()
    }

    fn record_command_buffers(graphic_engine: &mut GraphicEngine, image_i: usize) {
        let projection_view = Self::create_projection_view(graphic_engine);
        let framebuffer = graphic_engine.framebuffers.get(image_i).unwrap();
        graphic_engine.command_buffers[image_i] = Self::record_command_buffer(&graphic_engine, projection_view, framebuffer)
    }

    // TODO: should be made out of camera after migration to 3d
    fn create_projection_view(graphic_engine: &mut GraphicEngine) -> Mat4 {
// TODO: simplify
        let dimensions: Vec2 = graphic_engine.viewport.dimensions.into();

        let mut view = Mat4::look_at_lh(
            graphic_engine.camera.transform.position,
            Vec3::ZERO,
            Vec3::new(0.0, 0.5, 0.0)
        );
        view.y_axis.y *= -1.0;

        let aspect = dimensions.x / dimensions.y;
        let perspective = Mat4::perspective_lh(
            graphic_engine.camera.camera.field_of_view.to_radians(), aspect, graphic_engine.camera.camera.near_clip_plane, graphic_engine.camera.camera.far_clip_plane
        );
        // let orto = Mat4::orthographic_lh(-aspect, aspect, -1.0, 1.0, 0.1, 1.1);
        let mut projection = perspective;
        let projection_view = projection * view;
        projection_view
    }

    fn record_command_buffer(graphic_engine: &&mut GraphicEngine, projection_view: Mat4, framebuffer: &Arc<Framebuffer>) -> Arc<PrimaryAutoCommandBuffer> {
        let mut builder = AutoCommandBufferBuilder::primary(
            graphic_engine.device.clone(),
            graphic_engine.queue.family(), // do i not need to clone or vsc is retarded
            CommandBufferUsage::MultipleSubmit, // don't forget to write the correct buffer usage
        )
            .unwrap();


        let mut commands: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> = builder
            .begin_render_pass(
                framebuffer.clone(),
                SubpassContents::Inline,
                vec![[0.0, 0.0, 0.0, 0.0].into(), [0.0, 0.0, 0.0, 0.0].into()],
            )
            .unwrap();

        for object in &graphic_engine.objects {
            let material = object.material.clone();
            commands = graphic_engine.materials.borrow().get(material).deref().borrow().draw(object, projection_view, commands)
        }

        commands.end_render_pass().unwrap();

        Arc::new(builder.build().unwrap())
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
