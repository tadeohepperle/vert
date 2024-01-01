use wgpu::BindGroupDescriptor;

use crate::{
    elements::{buffer::ToRaw, Camera3D, UniformBuffer},
    utils::Timing,
    Dependencies, Handle, Module, WinitMain,
};

use super::{input::ResizeEvent, GraphicsContext, Input, Prepare, Renderer};

/// Very similar to MainCamera3D
pub struct MainScreenSize {
    uniform: UniformBuffer<ScreenSize>,
    deps: Deps,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
}

#[derive(Debug, Dependencies)]
pub struct Deps {
    input: Handle<Input>,
    renderer: Handle<Renderer>,
    ctx: Handle<GraphicsContext>,
}

impl Module for MainScreenSize {
    type Config = ();
    type Dependencies = Deps;

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let width = deps.ctx.size.width;
        let height = deps.ctx.size.height;
        let scale_factor = deps.ctx.scale_factor;

        let screen_size = ScreenSize {
            width,
            height,
            scale_factor,
        };
        let uniform = UniformBuffer::new(screen_size, &deps.ctx.device);

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
        let bind_group_layout = deps.ctx.device.create_bind_group_layout(&layout_descriptor);
        let bind_group = deps
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ScreenSize BindGroup"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.buffer().as_entire_binding(),
                }],
            });

        Ok(Self {
            uniform,
            deps,
            bind_group_layout,
            bind_group,
        })
    }

    fn intialize(handle: Handle<Self>) -> anyhow::Result<()> {
        let mut input = handle.deps.input;
        input.register_resize_listener(handle, Self::resize, Timing::MIDDLE);

        let mut renderer = handle.deps.renderer;
        renderer.register_prepare(handle);

        Ok(())
    }
}

impl Prepare for MainScreenSize {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        self.uniform.update_raw_and_buffer(queue);
    }
}

impl MainScreenSize {
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    fn resize(&mut self, event: ResizeEvent) {
        // todo! scale_factor needs to be part of event.
        self.uniform.value = ScreenSize {
            width: event.new_size.width,
            height: event.new_size.height,
            scale_factor: self.uniform.value.scale_factor,
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

impl ScreenSize {
    /// width / height
    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

/// the stuff that gets sent to the shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct ScreenSizeRaw {
    width: f32,
    height: f32,
    aspect: f32,
    scale_factor: f32,
}

impl ToRaw for ScreenSize {
    type Raw = ScreenSizeRaw;

    fn to_raw(&self) -> Self::Raw {
        ScreenSizeRaw {
            width: self.width as f32,
            height: self.height as f32,
            aspect: self.aspect(),
            scale_factor: self.scale_factor as f32,
        }
    }
}
