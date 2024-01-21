use std::{borrow::Cow, marker::PhantomData};

use wgpu::util::{BufferInitDescriptor, DeviceExt};

use crate::utils::next_pow2_number;

pub trait ToRaw {
    type Raw: Copy + bytemuck::Pod + bytemuck::Zeroable + PartialEq;
    fn to_raw(&self) -> Self::Raw;
}

pub trait BufferT {}

pub struct UniformBuffer<U: Copy + bytemuck::Pod + bytemuck::Zeroable + PartialEq> {
    pub value: U,
    buffer: wgpu::Buffer,
    pub name: Option<Cow<'static, str>>,
}

impl<U: Copy + bytemuck::Pod + bytemuck::Zeroable + PartialEq> UniformBuffer<U> {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn update_and_prepare(&mut self, value: U, queue: &wgpu::Queue) {
        if value != self.value {
            self.value = value;
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.value]));
        }
    }

    pub fn new(value: U, device: &wgpu::Device) -> Self {
        let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&[value]),
            usage,
            label: None,
        });
        UniformBuffer {
            value,
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
pub struct VertexBuffer<V: bytemuck::Pod> {
    data: Vec<V>,
    buffer: wgpu::Buffer,
}

impl<V: bytemuck::Pod> VertexBuffer<V> {
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

#[derive(Debug)]
pub struct GrowableBuffer<T: bytemuck::Pod + bytemuck::Zeroable> {
    /// This is tracked in addition to having the len in the data, to have the possibility of clearing data at the end of frame without losing len information.
    /// See Gizmos and other immediate geometry.
    buffer_len: usize,
    buffer_cap: usize,
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
    phantom: PhantomData<T>,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> GrowableBuffer<T> {
    pub fn new_from_data(device: &wgpu::Device, usage: wgpu::BufferUsages, data: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(data),
            usage: usage | wgpu::BufferUsages::COPY_DST,
            label: None,
        });

        GrowableBuffer {
            buffer_len: data.len(),
            buffer_cap: data.len(),
            buffer,
            usage,
            phantom: PhantomData,
        }
    }

    pub fn new(device: &wgpu::Device, min_cap: usize, usage: wgpu::BufferUsages) -> Self {
        let n_bytes = std::mem::size_of::<T>() * min_cap;
        let zeros = vec![0u8; n_bytes];
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&zeros),
            usage: usage | wgpu::BufferUsages::COPY_DST,
            label: None,
        });

        GrowableBuffer {
            buffer_len: 0,
            buffer_cap: min_cap,
            buffer,
            usage,
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.buffer_len
    }

    /// updates the gpu buffer, growing it, when not having enough space for data.
    ///
    /// Todo! do not write, if empty!!
    pub fn prepare(&mut self, data: &[T], device: &wgpu::Device, queue: &wgpu::Queue) {
        self.buffer_len = data.len();
        if self.buffer_len <= self.buffer_cap {
            // println!(
            //     "Write buffer: {} {}   {} ",
            //     self.buffer_cap,
            //     self.buffer_len,
            //     std::any::type_name::<T>()
            // );
            // the space in the buffer is enough, just write all rects to the buffer.
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data))
        } else {
            // println!(
            //     "Create new Growable Buffer in Grow: {} {}   {} ",
            //     self.buffer_cap,
            //     self.buffer_len,
            //     std::any::type_name::<T>()
            // );
            // space is not enough, we need to create a new buffer:

            let new_cap = next_pow2_number(self.buffer_len);

            // not ideal here, but we can optimize later, should not happen too often that a buffer doubles hopefully.
            let mut cloned_data_with_zeros = data.to_vec();
            for _ in 0..(new_cap - self.buffer_len) {
                cloned_data_with_zeros.push(T::zeroed());
            }

            // create a new buffer with new doubled capacity
            self.buffer_cap = new_cap;
            self.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: bytemuck::cast_slice(&cloned_data_with_zeros),
                usage: self.buffer.usage(),
                label: None,
            });
        }
    }

    // /// may destroy buffer.
    // ///
    // /// You can use `allocate_enough_space` + `buffer_write` as an alternative to `prepare` to write data that is not in one continous memory region into the buffer.
    // pub fn allocate_enough_space(&mut self, len: usize, device: &wgpu::Device) {
    //     self.buffer_len = 0;
    //     if len > self.buffer_cap {
    //         let new_cap = next_pow2_number(len);
    //         self.buffer_cap= new_cap;
    //         let buffer_size = ( new_cap * size_of::<T>()) as u64;
    //         device.create_buffer(&BufferDescriptor { label: None, size:buffer_size, usage: self.buffer.usage(), mapped_at_creation: false });
    //     }
    // }

    // pub fn buffer_write(&mut self, index:  )

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}
