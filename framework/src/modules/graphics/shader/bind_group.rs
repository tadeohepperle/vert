use std::sync::OnceLock;

use smallvec::{smallvec, SmallVec};
use wgpu::{BindGroup, BindGroupLayout};

use crate::modules::graphics::elements::{buffer::UniformBuffer, camera::CameraValues};

pub trait IntoBindGroupLayouts {
    fn bind_group_layouts() -> SmallVec<[&'static wgpu::BindGroupLayout; 2]>;
}

impl IntoBindGroupLayouts for () {
    fn bind_group_layouts() -> SmallVec<[&'static wgpu::BindGroupLayout; 2]> {
        smallvec![]
    }
}

impl<T: StaticBindGroup> IntoBindGroupLayouts for T {
    fn bind_group_layouts() -> SmallVec<[&'static wgpu::BindGroupLayout; 2]> {
        smallvec![Self::layout()]
    }
}

trait StaticBindGroup {
    fn initialize(&self, device: &wgpu::Device);

    fn layout() -> &'static wgpu::BindGroupLayout;

    fn bind_group() -> &'static wgpu::BindGroup;
}

static _CAMERA_BIND_GROUP_LAYOUT: OnceLock<(BindGroup, BindGroupLayout)> = OnceLock::new();
pub struct Camera {
    pub uniform: UniformBuffer<CameraValues>,
}

impl StaticBindGroup for Camera {
    fn layout() -> &'static wgpu::BindGroupLayout {
        &_CAMERA_BIND_GROUP_LAYOUT
            .get()
            .expect("_CAMERA_BIND_GROUP_LAYOUT not set")
            .1
    }

    fn bind_group() -> &'static wgpu::BindGroup {
        &_CAMERA_BIND_GROUP_LAYOUT
            .get()
            .expect("_CAMERA_BIND_GROUP_LAYOUT not set")
            .0
    }

    fn initialize(&self, device: &wgpu::Device) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("CameraBindGroupLayout"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CameraBindGroup"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.uniform.buffer().as_entire_binding(),
            }],
        });

        _CAMERA_BIND_GROUP_LAYOUT
            .set((bind_group, layout))
            .expect("_CAMERA_BIND_GROUP_LAYOUT cannot be set");
    }
}

// static mut _CAMERA_BIND_GROUP_LAYOUT: wgpu::BindGroupLayout =
//     unsafe { std::mem::MaybeUninit::uninit().assume_init() };

// impl CameraBindGroupLayout {
//     unsafe fn _initialize(device: &wgpu::Device) {
//         let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//             label: Some("CameraBindGroupLayout"),
//             entries: &[wgpu::BindGroupLayoutEntry {
//                 binding: 0,
//                 visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
//                 ty: wgpu::BindingType::Buffer {
//                     ty: wgpu::BufferBindingType::Uniform,
//                     has_dynamic_offset: false,
//                     min_binding_size: None,
//                 },
//                 count: None,
//             }],
//         });
//         unsafe { _CAMERA_BIND_GROUP_LAYOUT = layout }
//     }
// }
