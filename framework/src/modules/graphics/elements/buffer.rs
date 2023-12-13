use std::{borrow::Cow, mem::size_of};

use wgpu::util::{BufferInitDescriptor, DeviceExt};

use crate::modules::graphics::VertexT;

pub trait ToRaw {
    type Raw: Copy + bytemuck::Pod + bytemuck::Zeroable + PartialEq;
    fn to_raw(&self) -> Self::Raw;
}

pub trait BufferT {}

pub struct UniformBuffer<U: ToRaw> {
    pub value: U,
    raw: U::Raw,
    buffer: wgpu::Buffer,
    pub name: Option<Cow<'static, str>>,
}

impl<U: ToRaw> UniformBuffer<U> {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn update_raw_and_buffer(&mut self, queue: &wgpu::Queue) {
        let raw = self.value.to_raw();
        if self.raw != raw {
            self.raw = raw;
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.raw]));
        }
    }

    pub fn new(value: U, device: &wgpu::Device) -> Self {
        let raw = value.to_raw();
        let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[raw]),
            usage,
            label: None,
        });
        UniformBuffer {
            value,
            raw,
            buffer,
            name: None,
        }
    }

    pub fn named(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }
}

pub struct InstanceBuffer<U: ToRaw> {
    values: Vec<U>,
    raw_values: Vec<U::Raw>,
    buffer: wgpu::Buffer,
    pub name: Option<Cow<'static, str>>,
    changed: bool,
}
impl<U: ToRaw> InstanceBuffer<U> {
    pub fn new(values: Vec<U>, device: &wgpu::Device) -> Self {
        let raw_values: Vec<U::Raw> = values.iter().map(|u| u.to_raw()).collect();
        // The InstanceBuffer is basically also a vertex buffer, only at pos 1 instead at pos 0.
        let usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&raw_values),
            usage,
            label: None,
        });
        InstanceBuffer {
            values,
            raw_values,
            buffer,
            name: None,
            changed: false,
        }
    }

    pub fn values(&self) -> &Vec<U> {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut Vec<U> {
        self.changed = true;
        &mut self.values
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn update_raw_and_buffer(&mut self, queue: &wgpu::Queue) {
        if self.changed {
            self.raw_values = self.values.iter().map(|u| u.to_raw()).collect();
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.raw_values));
        }
    }

    pub fn named(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn len(&self) -> u32 {
        self.values.len() as u32
    }
}

/// VertexBuffer cannot be updated.
pub struct VertexBuffer<V: VertexT> {
    data: Vec<V>,
    buffer: wgpu::Buffer,
}

impl<V: VertexT> VertexBuffer<V> {
    pub fn new(data: Vec<V>, device: &wgpu::Device) -> Self {
        let usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&data),
            usage,
            label: None,
        });
        VertexBuffer { data, buffer }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn len(&self) -> u32 {
        self.data.len() as u32
    }
}

pub struct IndexBuffer {
    /// vertex indices
    pub data: Vec<u32>,
    pub buffer: wgpu::Buffer,
}

impl IndexBuffer {
    pub fn new(data: Vec<u32>, device: &wgpu::Device) -> Self {
        let usage = wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST;
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&data),
            usage,
            label: None,
        });
        IndexBuffer { data, buffer }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn len(&self) -> u32 {
        self.data.len() as u32
    }
}

// pub struct RawBuffer<R>
// where
//     R: Copy + bytemuck::Pod + bytemuck::Zeroable,
// {
//     pub buffer: wgpu::Buffer,
//     pub data: Vec<R>,
// }

// impl<R: Copy + bytemuck::Pod + bytemuck::Zeroable> RawBuffer<R> {
//     pub fn new(device: &wgpu::Device, data: Vec<R>, usage: wgpu::BufferUsages) -> Self {
//         let buffer = device.create_buffer_init(&BufferInitDescriptor {
//             contents: bytemuck::cast_slice(&data),
//             usage,
//             label: None,
//         });
//         Self { buffer, data }
//     }

//     pub fn from_slice<T: ToRaw<Raw = R>>(
//         device: &wgpu::Device,
//         data: &[T],
//         usage: wgpu::BufferUsages,
//     ) -> Self {
//         let raw_data = data.iter().map(ToRaw::to_raw).collect::<Vec<R>>();
//         Self::new(device, raw_data, usage)
//     }

//     pub fn buffer_size(&self) -> wgpu::BufferAddress {
//         (self.data.len() * size_of::<R>()) as wgpu::BufferAddress
//     }
// }

// pub struct Buffer<U, R>
// where
//     U: ToRaw<Raw = R>,
//     R: Copy + bytemuck::Pod + bytemuck::Zeroable,
// {
//     pub name: Option<Cow<'static, str>>,
//     pub data: Vec<U>,
//     pub data: Vec<R>,
//     pub buffer: wgpu::Buffer,

//     pub raw_buffer: RawBuffer<R>,
//     pub usage: wgpu::BufferUsages,
// }

// impl<U, R> Buffer<U, R>
// where
//     U: ToRaw<Raw = R>,
//     R: Copy + bytemuck::Pod + bytemuck::Zeroable,
// {
//     pub fn uniform(device: &wgpu::Device, object: U) -> Self {
//         let data = vec![object];
//         let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
//         Self::new(device, data, usage)
//     }

//     pub fn storage(device: &wgpu::Device, data: Vec<U>) -> Self {
//         let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
//         Self::new(device, data, usage)
//     }

//     pub fn named(mut self, name: impl Into<Cow<'static, str>>) -> Self {
//         self.name = Some(name.into());
//         self
//     }

//     // todo!() functions for updating a buffer using a staging buffer or writing to the queue directly.

//     pub fn new(device: &wgpu::Device, data: Vec<U>, usage: wgpu::BufferUsages) -> Self {
//         let raw_buffer = RawBuffer::from_slice(device, &data, usage);
//         Self::from_parts(data, raw_buffer, usage)
//     }

//     pub fn from_parts(data: Vec<U>, raw_buffer: RawBuffer<R>, usage: wgpu::BufferUsages) -> Self {
//         Self {
//             data,
//             raw_buffer,
//             usage,
//             name: None,
//         }
//     }
// }