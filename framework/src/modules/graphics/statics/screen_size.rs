use std::sync::{Arc, OnceLock};

use smallvec::{smallvec, SmallVec};
use wgpu::{
    naga::{ScalarKind, TypeInner, VectorSize},
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, ShaderStages,
};

use crate::modules::graphics::{
    elements::buffer::{ToRaw, UniformBuffer},
    graphics_context::GraphicsContext,
};

use super::{StaticBindGroup, ToBindGroup};

/// similar to a camera but only projects in screen space.
pub struct ScreenSize {
    uniform: UniformBuffer<ScreenSizeValues>,
}

impl ToBindGroup for ScreenSize {
    fn layout_descriptor() -> wgpu::BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            label: Some("ScreenSize BindGroup Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None, // ???
                },
                count: None,
            }],
        }
    }

    fn to_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ScreenSize BindGroup"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.uniform.buffer().as_entire_binding(),
            }],
        })
    }
}

static _SCREEN_SIZE_BIND_GROUP: OnceLock<(BindGroup, BindGroupLayout)> = OnceLock::new();
impl StaticBindGroup for ScreenSize {
    fn bind_group_layout() -> &'static wgpu::BindGroupLayout {
        &_SCREEN_SIZE_BIND_GROUP
            .get()
            .expect("_CAMERA_BIND_GROUP not set")
            .1
    }

    fn bind_group() -> &'static wgpu::BindGroup {
        &_SCREEN_SIZE_BIND_GROUP
            .get()
            .expect("_CAMERA_BIND_GROUP not set")
            .0
    }
}

impl ScreenSize {
    /// # Warning
    ///
    /// Should only be called once!
    pub fn new(context: &GraphicsContext) -> ScreenSize {
        let size = context.size();
        let values = ScreenSizeValues {
            width: size.width,
            height: size.height,
            scale_factor: context.scale_factor(),
        };
        let uniform = UniformBuffer::new(values, &context.device);
        let screen_size = ScreenSize { uniform };

        // Initialize the static bind group for the screen size
        let layout = context
            .device
            .create_bind_group_layout(&ScreenSize::layout_descriptor());
        let bind_group = screen_size.to_bind_group(&context.device, &layout);
        _SCREEN_SIZE_BIND_GROUP
            .set((bind_group, layout))
            .expect("_SCREEN_SIZE_BIND_GROUP cannot be set");

        screen_size
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
