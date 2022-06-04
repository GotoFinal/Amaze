use std::cmp::min;
use std::sync::Arc;
use vulkano::device::{Device, DeviceOwned, DeviceExtensions};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType, QueueFamily};
use vulkano::image::{AttachmentImage, ImageAccess, ImageUsage, SampleCount, SampleCounts, SwapchainImage};
use vulkano::image::view::ImageView;
use vulkano::instance::Instance;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo};
use winit::window::Window;
use crate::engine::renderer::options::{Buffering, Multisampling};
use crate::GraphicOptions;

pub fn combine_sample_counts(a: SampleCounts, b: SampleCounts) -> SampleCounts {
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

pub fn select_physical_device<'a>(
    instance: &'a Arc<Instance>,
    surface: Arc<Surface<Arc<Window>>>,
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
pub fn get_sample_count(sample: Multisampling, max_samples: SampleCounts) -> SampleCount {
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

pub fn get_framebuffers(
    images: &[Arc<SwapchainImage<Arc<Window>>>],
    render_pass: Arc<RenderPass>,
    sample: SampleCount
) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            let intermediary = ImageView::new_default(
                AttachmentImage::transient_multisampled(
                    render_pass.device().clone(),
                    view.image().dimensions().width_height(),
                    sample,
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

pub fn get_render_pass(
    device: Arc<Device>,
    swapchain: Arc<Swapchain<Arc<Window>>>,
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

pub fn create_swapchain(options: GraphicOptions, surface: &Arc<Surface<Arc<Window>>>, physical_device: PhysicalDevice, device: &Arc<Device>) -> (Arc<Swapchain<Arc<Window>>>, Vec<Arc<SwapchainImage<Arc<Window>>>>) {
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
    let (mut swapchain, images): (Arc<Swapchain<Arc<Window>>>, Vec<Arc<SwapchainImage<Arc<Window>>>>) =
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
    (swapchain, images)
}