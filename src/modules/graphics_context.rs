use std::sync::Arc;

use crate::{utils::Timing, Dependencies};
use glam::DVec2;
use wgpu::SurfaceTexture;
use winit::{dpi::PhysicalSize, window::Window};

use crate::{modules::WinitMain, Handle, Module};

use super::{input::ResizeEvent, Input, TokioRuntime};

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
    /// todo! add scale_factor to resize event and make sure it is updated.
    pub scale_factor: f64,
    deps: Deps,
}

#[derive(Debug, Dependencies)]
pub struct Deps {
    tokio: Handle<TokioRuntime>,
    winit: Handle<WinitMain>,
    input: Handle<Input>,
}

impl Module for GraphicsContext {
    type Config = ();

    type Dependencies = Deps;

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let tokio = deps.tokio;
        let graphics_context =
            tokio.block_on(async move { initialize_graphics_context(deps).await })?;
        Ok(graphics_context)
    }

    fn intialize(handle: Handle<Self>) -> anyhow::Result<()> {
        let mut input = handle.deps.input;
        input.register_resize_listener(handle, Self::resize, Timing::EARLY - 10);
        Ok(())
    }
}

impl GraphicsContext {
    fn resize(&mut self, event: ResizeEvent) {
        println!("Graphics context resized: {event:?}");
        // todo!()
        self.surface_config.width = event.new_size.width;
        self.surface_config.height = event.new_size.height;
        self.size = event.new_size;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub fn size_dvec2(&self) -> DVec2 {
        DVec2 {
            x: self.size.width as f64,
            y: self.size.height as f64,
        }
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
        let view = output.texture.create_view(&Default::default());
        (output, view)
    }
}

async fn initialize_graphics_context(deps: Deps) -> anyhow::Result<GraphicsContext> {
    let window = deps.winit.window();

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
        instance,
        adapter,
        device,
        queue,
        surface,
        surface_format,
        surface_config,
        size,
        scale_factor,
        deps,
    };

    Ok(context)
}
