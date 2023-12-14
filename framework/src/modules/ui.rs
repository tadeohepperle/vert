use std::{cmp::Ordering, ops::Range, sync::Arc};

use bytemuck::Zeroable;
use wgpu::util::{BufferInitDescriptor, DeviceExt};

use super::graphics::{
    elements::{
        texture::BindableTexture,
        ui_rect::{UiRect, UiRectInstance, UiRectRenderPipeline},
    },
    graphics_context::GraphicsContext,
    Prepare, Render,
};

const RECT_BUFFER_MIN_SIZE: usize = 256;

/// Immediate mode ui drawing. Collects rects that are then drawn by the renderer.
/// This is my own take on a Immediate mode UI lib like egui.
///
/// at the start of every frame rect_queue is cleared. We can submit new rects to rectqueue.
/// Before rendering (prepare stage) all the rects in the rect_queue are sorted after their textures and written into one
/// big instance buffer.
pub struct ImmediateUi {
    context: GraphicsContext,
    /// cleared every frame
    rect_queue: Vec<UiRect>,
    /// written to and recreated if too small
    prepared_rects: PeparedRects,
}

impl ImmediateUi {
    pub fn new(context: GraphicsContext) -> Self {
        let draw_rects = PeparedRects::new(&context.device);

        ImmediateUi {
            context,
            rect_queue: vec![],
            prepared_rects: draw_rects,
        }
    }

    pub fn draw_rect(&mut self, ui_rect: UiRect) {
        self.rect_queue.push(ui_rect);
    }

    pub(crate) fn prepared_rects(&self) -> &PeparedRects {
        &self.prepared_rects
    }
}

impl Prepare for ImmediateUi {
    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder) {
        let rects = std::mem::take(&mut self.rect_queue);
        self.prepared_rects.prepare(rects, context);
    }
}

#[derive(Debug)]
pub struct RectInstanceBuffer {
    len: usize,
    cap: usize,
    buffer: wgpu::Buffer,
}

impl RectInstanceBuffer {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

#[derive(Debug)]
/// We can sort all rects into the texture groups they have. This way we have only N-TextureGroups draw calls.
pub struct PeparedRects {
    /// Buffer with instances (sorted)
    pub instance_buffer: RectInstanceBuffer,
    /// texture_regions, refer to regions of the sorted buffer.
    pub texture_groups: Vec<(Range<u32>, Option<Arc<BindableTexture>>)>,
}

impl PeparedRects {
    /// create an new DrawRects backed by a gpu buffer with RECT_BUFFER_MIN_SIZE elements in it.
    pub fn new(device: &wgpu::Device) -> Self {
        let n_bytes = std::mem::size_of::<UiRectInstance>() * RECT_BUFFER_MIN_SIZE;
        let zeros = vec![0u8; n_bytes];
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&zeros),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            label: None,
        });
        let instances = RectInstanceBuffer {
            len: 0,
            cap: RECT_BUFFER_MIN_SIZE,
            buffer,
        };

        PeparedRects {
            instance_buffer: instances,
            texture_groups: vec![],
        }
    }

    /// sorts the rects after their textures and updates the GPU buffer. If GPU buffer too small, create a new one with 2x the last capacity.
    pub fn prepare(&mut self, rects: Vec<UiRect>, context: &GraphicsContext) {
        let (mut instances, texture_groups) = create_sorted_rect_instances(rects);
        self.texture_groups = texture_groups;
        self.instance_buffer.len = instances.len();
        if self.instance_buffer.cap <= instances.len() {
            // the space in the buffer is enough, just write all rects to the buffer.
            context.queue.write_buffer(
                &self.instance_buffer.buffer,
                0,
                bytemuck::cast_slice(&instances),
            )
        } else {
            // space is not enough, we need to create a new buffer:
            let mut new_cap = RECT_BUFFER_MIN_SIZE;
            while instances.len() > new_cap {
                new_cap *= 2;
            }
            // fill up with zeroed elements:
            for _ in 0..(new_cap - instances.len()) {
                instances.push(UiRectInstance::zeroed());
            }
            // create a new buffer, now with probably 2x the size:
            self.instance_buffer.cap = new_cap;
            self.instance_buffer.buffer =
                context.device.create_buffer_init(&BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    label: None,
                });
        }
    }
}

fn create_sorted_rect_instances(
    mut rects: Vec<UiRect>,
) -> (
    Vec<UiRectInstance>,
    Vec<(Range<u32>, Option<Arc<BindableTexture>>)>,
) {
    if rects.is_empty() {
        return (vec![], vec![]);
    }

    rects.sort_by(|a, b| match (&a.texture, &b.texture) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => a.texture.id.cmp(&b.texture.id),
    });

    // cache this to use it after the loop

    let mut instances: Vec<UiRectInstance> = vec![];
    let mut texture_groups: Vec<(Range<u32>, Option<Arc<BindableTexture>>)> = vec![];

    let mut last_start_idx: usize = 0;
    let mut last_texture_id: Option<u128> = None;
    let mut last_group_texture: Option<Arc<BindableTexture>> =
        rects.first().unwrap().texture.clone();

    for (i, rect) in rects.into_iter().enumerate() {
        instances.push(rect.instance);
        let texture_id = rect.texture.as_ref().map(|e| e.texture.id);
        if texture_id != last_texture_id {
            let range = (last_start_idx as u32)..(i as u32);
            texture_groups.push((range, last_group_texture));
            last_start_idx = i;
            last_texture_id = texture_id;
            last_group_texture = rect.texture;
        }
    }

    if last_start_idx < instances.len() {
        let range = (last_start_idx as u32)..(instances.len() as u32);
        texture_groups.push((range, last_group_texture));
    }
    (instances, texture_groups)
}
