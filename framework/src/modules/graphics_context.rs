use std::sync::Arc;

use wgpu::SurfaceTexture;
use winit::{dpi::PhysicalSize, window::Window};

use crate::{Handle, Module, WinitMain};

use super::{winit_main, Resize, TokioRuntime};

#[derive(Debug)]
pub struct GraphicsContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub surface_format: wgpu::TextureFormat,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    pub scale_factor: f64,
}

impl GraphicsContext {
    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub const SURFACE_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

    pub fn new_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            })
    }

    pub fn new_surface_texture_and_view(&self) -> (SurfaceTexture, wgpu::TextureView) {
        let output = self
            .surface
            .get_current_texture()
            .expect("wgpu surface error");
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        (output, view)
    }
}

impl Resize for GraphicsContext {
    fn resize(&mut self, new_size: PhysicalSize<u64>) {
        self.surface_config.width = new_size.width as u32;
        self.surface_config.height = new_size.height as u32;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

async fn initialize_graphics_context(window: &Window) -> anyhow::Result<GraphicsContext> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let surface = unsafe { instance.create_surface(&window) }.unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::MULTIVIEW | wgpu::Features::PUSH_CONSTANTS,
                limits: wgpu::Limits {
                    max_push_constant_size: 16,
                    ..Default::default()
                },
            },
            None,
        )
        .await
        .unwrap();

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| *f == GraphicsContext::SURFACE_COLOR_FORMAT)
        .expect("SURFACE_FORMAT not found in surface caps ");

    let size = window.inner_size();
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::AutoNoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
    };
    surface.configure(&device, &surface_config);

    let scale_factor = window.scale_factor();

    let context = GraphicsContext {
        instance: instance,
        adapter: adapter,
        device: device,
        queue: queue,
        surface: surface,
        surface_format,
        surface_config: surface_config,
        size: size,
        scale_factor: scale_factor,
    };

    Ok(context)
}

impl Module for GraphicsContext {
    type Config = ();

    type Dependencies = (Handle<TokioRuntime>, Handle<WinitMain>);

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let tokio_rt = deps.0;
        let winit_main_module = deps.1;
        let window = winit_main_module.window();

        let graphics_context =
            tokio_rt.block_on(async move { initialize_graphics_context(window).await })?;
        Ok(graphics_context)
    }
}
