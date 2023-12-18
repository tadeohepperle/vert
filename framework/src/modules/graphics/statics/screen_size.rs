use std::sync::{Arc, OnceLock};

use wgpu::{BindGroup, BindGroupLayout};

use crate::modules::graphics::{
    elements::buffer::{ToRaw, UniformBuffer},
    graphics_context::GraphicsContext,
    shader::bind_group::StaticBindGroup,
};

/// similar to a camera but only projects in screen space.
pub struct ScreenSize {
    uniform: UniformBuffer<ScreenSizeValues>,
}

impl ScreenSize {
    pub fn new(context: &GraphicsContext) -> ScreenSize {
        let size = context.size();
        let values = ScreenSizeValues {
            width: size.width,
            height: size.height,
            scale_factor: context.scale_factor(),
        };
        let uniform = UniformBuffer::new(values, &context.device);

        // Initialize the static bind group for the screen size
        let layout = context
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
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.buffer().as_entire_binding(),
                }],
            });

        _SCREEN_SIZE_BIND_GROUP
            .set((bind_group, layout))
            .expect("_SCREEN_SIZE_BIND_GROUP cannot be set");

        ScreenSize { uniform }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.uniform.value.width = width;
        self.uniform.value.height = height;
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
pub struct ScreenSizeValues {
    width: u32,
    height: u32,
    scale_factor: f64,
}

impl ScreenSizeValues {
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
}

impl ToRaw for ScreenSizeValues {
    type Raw = ScreenSizeRaw;

    fn to_raw(&self) -> Self::Raw {
        ScreenSizeRaw {
            width: self.width as f32,
            height: self.height as f32,
            aspect: self.aspect(),
        }
    }
}

static _SCREEN_SIZE_BIND_GROUP: OnceLock<(BindGroup, BindGroupLayout)> = OnceLock::new();
impl StaticBindGroup for ScreenSize {
    fn bind_group_layout() -> &'static wgpu::BindGroupLayout {
        &_SCREEN_SIZE_BIND_GROUP
            .get()
            .expect("_SCREEN_SIZE_BIND_GROUP not set")
            .1
    }

    fn bind_group() -> &'static wgpu::BindGroup {
        &_SCREEN_SIZE_BIND_GROUP
            .get()
            .expect("_SCREEN_SIZE_BIND_GROUP not set")
            .0
    }
}
