use atomic_refcell::AtomicRefCell;
use winit::{dpi::PhysicalSize, window::Window};

const DESIRED_SURFACE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

pub struct GraphicsContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub surface_format: wgpu::TextureFormat,
    pub surface_config: AtomicRefCell<wgpu::SurfaceConfiguration>,
}

impl GraphicsContext {
    pub async fn initialize(window: &Window) -> anyhow::Result<Self> {
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
            .find(|f| *f == DESIRED_SURFACE_FORMAT)
            .expect("SURFACE_FORMAT not found in surface caps ");

        let physical_size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: physical_size.width,
            height: physical_size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let surface_config = AtomicRefCell::new(surface_config);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            surface_format,
            surface_config,
        })
    }

    pub fn resize(&self, new_size: PhysicalSize<u32>) {
        let mut surface_config = self.surface_config.borrow_mut();
        surface_config.width = new_size.width;
        surface_config.height = new_size.height;
        self.surface.configure(&self.device, &surface_config);
    }
}
