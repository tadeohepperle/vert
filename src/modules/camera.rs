


use crate::{
    elements::{Camera3D, UniformBuffer},
    utils::Timing,
    Dependencies, Handle, Module,
};

use super::{input::ResizeEvent, GraphicsContext, Input, Prepare, Renderer};

/// Very similar to MainScreenSize
pub struct MainCamera3D {
    uniform: UniformBuffer<Camera3D>,
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

impl Module for MainCamera3D {
    type Config = ();
    type Dependencies = Deps;

    fn new(_config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let camera_3d = Camera3D::new(deps.ctx.size.width, deps.ctx.size.height);
        let uniform = UniformBuffer::new(camera_3d, &deps.ctx.device);

        let layout_descriptor = wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera BindGroupLayout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None, // ??? is this right?
                },
                count: None,
            }],
        };
        let bind_group_layout = deps.ctx.device.create_bind_group_layout(&layout_descriptor);
        let bind_group = deps
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Camera BindGroup"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.buffer().as_entire_binding(),
                }],
            });

        Ok(Self {
            uniform,
            deps,
            bind_group,
            bind_group_layout,
        })
    }

    fn intialize(handle: Handle<Self>) -> anyhow::Result<()> {
        let mut input = handle.deps.input;
        input.register_resize_listener(handle, Self::resize, Timing::DEFAULT);

        let mut renderer = handle.deps.renderer;
        renderer.register_prepare(handle);
        Ok(())
    }
}

impl Prepare for MainCamera3D {
    fn prepare(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) {
        self.uniform.update_raw_and_buffer(queue);
    }
}

impl MainCamera3D {
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    fn resize(&mut self, event: ResizeEvent) {
        self.uniform
            .value
            .projection
            .resize(event.new_size.width, event.new_size.height);
    }

    pub fn camera_mut(&mut self) -> &mut Camera3D {
        &mut self.uniform.value
    }
}
