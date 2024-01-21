use crate::{
    elements::{buffer::ToRaw, UniformBuffer},
    modules::GraphicsContext,
    Resize, Resized,
};

/// Very similar to MainCamera3D
pub struct ScreenGR {
    uniform: UniformBuffer<ScreenRaw>,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ScreenGR {
    pub fn new(ctx: &GraphicsContext, screen: &Screen) -> Self {
        let _width = ctx.size.width;
        let _height = ctx.size.height;

        let uniform = UniformBuffer::new(screen.to_raw(), &ctx.device);

        let layout_descriptor = wgpu::BindGroupLayoutDescriptor {
            label: Some("ScreenSize BindGroupLayout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        };
        let bind_group_layout = ctx.device.create_bind_group_layout(&layout_descriptor);
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ScreenSize BindGroup"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.buffer().as_entire_binding(),
            }],
        });

        Self {
            uniform,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, screen: &Screen) {
        self.uniform.update_and_prepare(screen.to_raw(), queue);
    }
}

impl ScreenGR {
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

impl Resize for Screen {
    fn resize(&mut self, resized: Resized) {
        self.height = resized.new_size.height;
        self.width = resized.new_size.width;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

impl Screen {
    pub fn from_window(window: &winit::window::Window) -> Self {
        Self {
            width: window.inner_size().width,
            height: window.inner_size().height,
            scale_factor: window.scale_factor(),
        }
    }

    /// width / height
    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

/// the stuff that gets sent to the shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct ScreenRaw {
    width: f32,
    height: f32,
    aspect: f32,
    scale_factor: f32,
}

impl ToRaw for Screen {
    type Raw = ScreenRaw;

    fn to_raw(&self) -> Self::Raw {
        ScreenRaw {
            width: self.width as f32,
            height: self.height as f32,
            aspect: self.aspect(),
            scale_factor: self.scale_factor as f32,
        }
    }
}
