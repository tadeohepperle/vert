use std::{borrow::BorrowMut, fmt::Write, sync::Arc};

use atomic_refcell::AtomicRefCell;
use tokio::sync::watch;
use wgpu::{SurfaceConfiguration, SurfaceTexture};
use winit::{dpi::PhysicalSize, window::Window};

use crate::utils::{Reader, Writer};

pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

/// not too expensive to clone
#[derive(Debug, Clone)]
pub struct GraphicsContext {
    pub instance: Arc<wgpu::Instance>,
    pub adapter: Arc<wgpu::Adapter>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface: Arc<wgpu::Surface>,
    pub surface_format: wgpu::TextureFormat,
    pub surface_config: Reader<wgpu::SurfaceConfiguration>,
    pub size: Reader<PhysicalSize<u32>>,
    pub scale_factor: Reader<f64>,
}

impl GraphicsContext {
    pub fn size(&self) -> PhysicalSize<u32> {
        self.size.get().clone()
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor.get().clone()
    }
}

pub struct GraphicsOwner {
    pub context: GraphicsContext,
    pub surface_config: Writer<wgpu::SurfaceConfiguration>,
    pub size: Writer<PhysicalSize<u32>>,
    pub scale_factor: Writer<f64>,
}

impl GraphicsOwner {
    pub async fn intialize(window: &Window) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = unsafe { instance.create_surface(window) }.unwrap();
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
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
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
            .find(|f| *f == COLOR_FORMAT)
            .expect("SURFACE_FORMAT not found in surface caps ");

        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        // let (surface_config_tx, surface_config_rx) = watch::channel(surface_config);
        // let (size_tx, size_rx) = watch::channel(size);

        let surface_config = Writer::new(surface_config);
        let size = Writer::new(size);
        let scale_factor = Writer::new(window.scale_factor());

        let context = GraphicsContext {
            instance: Arc::new(instance),
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
            surface: Arc::new(surface),
            surface_format,
            surface_config: surface_config.reader(),
            size: size.reader(),
            scale_factor: scale_factor.reader(),
        };

        let context_updater = GraphicsOwner {
            surface_config,
            size,
            context,
            scale_factor,
        };

        Ok(context_updater)
    }

    pub fn new_encoder(&self) -> wgpu::CommandEncoder {
        self.context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            })
    }

    pub fn new_surface_texture_and_view(&self) -> (SurfaceTexture, wgpu::TextureView) {
        let output = self
            .context
            .surface
            .get_current_texture()
            .expect("wgpu surface error");
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        (output, view)
    }
}

impl GraphicsOwner {
    pub fn resize(&self, new_size: PhysicalSize<u32>) {
        let mut surface_config = self.surface_config.get_mut();
        surface_config.width = new_size.width;
        surface_config.height = new_size.height;
        self.context
            .surface
            .configure(&self.context.device, &surface_config);
    }
}
