use winit::{dpi::PhysicalSize, event::WindowEvent};

pub trait Prepare {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    );
}

#[derive(Debug, Clone, Copy)]
pub struct Resized {
    pub new_size: PhysicalSize<u32>,
}
pub trait Resize {
    fn resize(&mut self, resized: Resized);
}

pub trait ReceiveWindowEvent {
    fn receive_window_event(&mut self, event: &WindowEvent);
}
