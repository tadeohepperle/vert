use std::sync::Arc;

use glam::DVec2;
use wgpu::SurfaceTexture;
use winit::{dpi::PhysicalSize, window::Window};

use crate::{Resize, Resized};

#[derive(Debug)]
pub struct GraphicsContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface: wgpu::Surface,
    pub surface_format: wgpu::TextureFormat,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphicsContextConfig {
    pub features: wgpu::Features,
    pub present_mode: wgpu::PresentMode,
    pub max_push_constant_size: u32,
}

impl Default for GraphicsContextConfig {
    fn default() -> Self {
        Self {
            features: wgpu::Features::MULTIVIEW
                | wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                | wgpu::Features::TEXTURE_BINDING_ARRAY,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            max_push_constant_size: 64,
        }
    }
}

impl Resize for GraphicsContext {
    fn resize(&mut self, event: Resized) {
        println!("Graphics context resized: {event:?}");
        // todo!()
        self.surface_config.width = event.new_size.width;
        self.surface_config.height = event.new_size.height;
        self.size = event.new_size;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

impl GraphicsContext {
    pub fn new(
        config: GraphicsContextConfig,
        rt: &tokio::runtime::Runtime,
        window: &Window,
    ) -> anyhow::Result<Self> {
        let graphics_context =
            rt.block_on(async move { initialize_graphics_context(config, window).await })?;
        Ok(graphics_context)
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

pub async fn initialize_graphics_context(
    config: GraphicsContextConfig,
    window: &Window,
) -> anyhow::Result<GraphicsContext> {
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
                features: config.features,
                limits: wgpu::Limits {
                    max_push_constant_size: config.max_push_constant_size,
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
        present_mode: config.present_mode,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
    };
    surface.configure(&device, &surface_config);

    let context = GraphicsContext {
        instance,
        adapter,
        device: Arc::new(device),
        queue: Arc::new(queue),
        surface,
        surface_format,
        surface_config,
        size,
    };

    Ok(context)
}
