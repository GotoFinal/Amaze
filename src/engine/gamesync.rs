use std::sync::Arc;

use vulkano::{
    command_buffer::{CommandBufferExecFuture, PrimaryAutoCommandBuffer},
    swapchain::{PresentFuture, SwapchainAcquireFuture},
    sync::{FenceSignalFuture, GpuFuture, JoinFuture},
};
use winit::window::Window;

pub(crate) type GpuFence = FenceSignalFuture<
    PresentFuture<
        CommandBufferExecFuture<
            JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture<Window>>,
            Arc<PrimaryAutoCommandBuffer>,
        >,
        Window,
    >,
>;

pub(crate) struct GameSync {
    previous_fence_i: usize,
    fences: Vec<Option<Arc<GpuFence>>>,
    current_fence_i: usize,
}

impl GameSync {
    pub fn new(frames: usize) -> GameSync {
        return GameSync { previous_fence_i: 0, fences: vec![None; frames], current_fence_i: 0 };
    }

    pub fn get_prev(&self) -> &Option<Arc<GpuFence>> {
        return &self.fences[self.previous_fence_i];
    }

    pub fn get_current(&self) -> &Option<Arc<GpuFence>> {
        return &self.fences[self.current_fence_i];
    }

    pub fn get_current_i(&self) -> usize {
        return self.current_fence_i;
    }

    pub fn set_current(&mut self, current: usize) {
        self.current_fence_i = current
    }

    pub fn update_fence(&mut self, current: Option<Arc<GpuFence>>) {
        self.fences[self.current_fence_i] = current;
        self.previous_fence_i = self.current_fence_i;
    }
}
