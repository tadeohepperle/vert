use std::{marker::PhantomData, ops::Range, sync::Arc};

use wgpu::util::DeviceExt;

use crate::modules::graphics::graphics_context::GraphicsContext;

use super::{buffer::GrowableBuffer, texture::BindableTexture};

const RECT_BUFFER_MIN_SIZE: usize = 256;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rect {
    /// min x, min y (top left corner)
    pub offset: [f32; 2],
    /// size x, size y
    pub size: [f32; 2],
}

impl Rect {
    pub const fn new(offset: [f32; 2], size: [f32; 2]) -> Self {
        Self { offset, size }
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            size: [1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone)]
pub struct RectWithTexture<T: RectT> {
    pub rect: T,
    pub texture: RectTexture,
}

#[derive(Debug, Clone)]
pub enum RectTexture {
    White,
    Text,
    Custom(Arc<BindableTexture>),
}

impl RectTexture {
    #[inline]
    pub fn id(&self) -> u128 {
        match self {
            RectTexture::White => 0,
            RectTexture::Text => 1,
            RectTexture::Custom(tex) => tex.texture.id,
        }
    }
}

impl PartialEq for RectTexture {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custom(l0), Self::Custom(r0)) => l0.texture.id == r0.texture.id,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Eq for RectTexture {}

impl PartialOrd for RectTexture {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(&self, &other))
    }
}

impl Ord for RectTexture {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id().cmp(&other.id())
    }
}

pub trait RectT: bytemuck::Zeroable + bytemuck::Pod {}

#[derive(Debug)]
/// We can sort all rects into the texture groups they have. This way we have only N-TextureGroups draw calls.
pub struct PeparedRects<T: RectT> {
    /// Buffer with instances (sorted)
    pub instance_buffer: GrowableBuffer<T>,
    /// texture_regions, refer to regions of the sorted buffer.
    pub texture_groups: Vec<(Range<u32>, RectTexture)>,
}

impl<T: RectT> PeparedRects<T> {
    /// create an new DrawRects backed by a gpu buffer with RECT_BUFFER_MIN_SIZE elements in it.
    pub fn new(device: &wgpu::Device) -> Self {
        PeparedRects {
            instance_buffer: GrowableBuffer::new(
                device,
                RECT_BUFFER_MIN_SIZE,
                wgpu::BufferUsages::VERTEX,
            ),
            texture_groups: vec![],
        }
    }

    /// sorts the rects after their textures and updates the GPU buffer. If GPU buffer too small, create a new one with 2x the last capacity.
    /// todo: maybe we can replace this by a general growable buffer:
    pub fn prepare(&mut self, rects: Vec<RectWithTexture<T>>, context: &GraphicsContext) {
        // Note: even if the instance buffer contains some crap entries or some trailing zeros, we do not care, because we
        // only render instances in the ranges that are returned in the texture_groups.
        let (instances, texture_groups) = create_sorted_rect_instances(rects);
        self.texture_groups = texture_groups;
        *self.instance_buffer.data() = instances;
        self.instance_buffer
            .prepare(&context.queue, &context.device);
    }
}

fn create_sorted_rect_instances<T: RectT>(
    mut rects: Vec<RectWithTexture<T>>,
) -> (Vec<T>, Vec<(Range<u32>, RectTexture)>) {
    if rects.is_empty() {
        return (vec![], vec![]);
    }

    rects.sort_by(|a, b| a.texture.cmp(&b.texture));

    let mut instances: Vec<T> = vec![];
    let mut texture_groups: Vec<(Range<u32>, RectTexture)> = vec![];

    let mut last_start_idx: usize = 0;
    let mut last_texture: RectTexture = rects.first().unwrap().texture.clone();
    let mut last_texture_id: u128 = last_texture.id();

    for (i, rect) in rects.into_iter().enumerate() {
        instances.push(rect.rect);
        let texture_id = rect.texture.id();
        if texture_id != last_texture_id {
            let range = (last_start_idx as u32)..(i as u32);
            texture_groups.push((range, last_texture));
            last_start_idx = i;
            last_texture = rect.texture;
            last_texture_id = texture_id;
        }
    }

    if last_start_idx < instances.len() {
        let range = (last_start_idx as u32)..(instances.len() as u32);
        texture_groups.push((range, last_texture));
    }
    (instances, texture_groups)
}
