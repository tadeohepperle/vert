use std::sync::Arc;

use wgpu::{BindGroup, BindGroupLayout};

use crate::modules::graphics::graphics_context::GraphicsContext;

use super::buffer::{ToRaw, UniformBuffer};

/// similar to a camera but only projects in screen space.
pub struct ScreenSpace {
    uniform: UniformBuffer<ScreenSpaceValues>,
    bind_group: ScreenSpaceBindGroup,
}

impl ScreenSpace {
    pub fn new(context: &GraphicsContext) -> ScreenSpace {
        let size = context.size();
        let values = ScreenSpaceValues {
            width: size.width,
            height: size.height,
            scale_factor: context.scale_factor(),
        };
        let uniform = UniformBuffer::new(values, &context.device);

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("screen space bindgroup layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("screen space bindgroup"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.buffer().as_entire_binding(),
                }],
            });

        ScreenSpace {
            uniform,
            bind_group: ScreenSpaceBindGroup(Arc::new((bind_group, bind_group_layout))),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.uniform.value.width = width;
        self.uniform.value.height = height;
    }

    pub fn bind_group(&self) -> ScreenSpaceBindGroup {
        self.bind_group.clone()
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue) {
        self.uniform.update_raw_and_buffer(queue);
    }
}

#[derive(Debug, Clone)]
pub struct ScreenSpaceBindGroup(Arc<(BindGroup, BindGroupLayout)>);
impl ScreenSpaceBindGroup {
    pub fn layout(&self) -> &BindGroupLayout {
        &self.0 .1
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.0 .0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenSpaceValues {
    width: u32,
    height: u32,
    scale_factor: f64,
}

impl ScreenSpaceValues {
    /// width / height
    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

/// the stuff that gets sent to the shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct ScreenSpaceValuesRaw {
    width: f32,
    height: f32,
    aspect: f32,
}

impl ToRaw for ScreenSpaceValues {
    type Raw = ScreenSpaceValuesRaw;

    fn to_raw(&self) -> Self::Raw {
        ScreenSpaceValuesRaw {
            width: self.width as f32,
            height: self.height as f32,
            aspect: self.aspect(),
        }
    }
}
