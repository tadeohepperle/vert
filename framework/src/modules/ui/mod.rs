use std::{borrow::Cow, cmp::Ordering, marker::PhantomData, ops::Range, sync::Arc};

use bytemuck::Zeroable;
use egui::{ahash::HashMap, Vec2};
use glam::IVec2;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Color,
};

use self::text_rasterizer::{DrawText, FontHandle, TextRasterizer};

use super::graphics::{
    elements::{
        buffer::ToRaw,
        rect::{PeparedRects, Rect, RectTexture, RectWithTexture},
        rect_3d::Rect3D,
        texture::{BindableTexture, Texture},
        transform::Transform,
        ui_rect::{UiRect, UiRectRenderPipeline},
    },
    graphics_context::GraphicsContext,
    Prepare, Render,
};

pub mod text_rasterizer;

/// Immediate mode ui drawing. Collects rects that are then drawn by the renderer.
/// This is my own take on a Immediate mode UI lib like egui.
///
/// at the start of every frame rect_queue is cleared. We can submit new rects to rectqueue.
/// Before rendering (prepare stage) all the rects in the rect_queue are sorted after their textures and written into one
/// big instance buffer.
pub struct ImmediateUi {
    /// cleared every frame
    ui_rect_queue: Vec<RectWithTexture<UiRect>>,
    rect_3d_queue: Vec<RectWithTexture<Rect3D>>,
    /// written to and recreated if too small
    prepared_ui_rects: PeparedRects<UiRect>,
    prepared_3d_rects: PeparedRects<Rect3D>,
    // /////////////////////////////////////////////////////////////////////////////
    // Text related things:
    // /////////////////////////////////////////////////////////////////////////////
    text_rasterizer: TextRasterizer,
}

impl ImmediateUi {
    pub fn new(context: GraphicsContext) -> Self {
        let prepared_ui_rects = PeparedRects::new(&context.device);
        let prepared_3d_rects = PeparedRects::new(&context.device);
        let text_rasterizer = TextRasterizer::new(context);
        ImmediateUi {
            text_rasterizer,
            ui_rect_queue: vec![],
            rect_3d_queue: vec![],
            prepared_ui_rects,
            prepared_3d_rects,
        }
    }

    /// draw text in ui space
    pub fn draw_text(&mut self, draw_text: &DrawText) {
        let layout_result = self.text_rasterizer.layout_and_rasterize_text(
            &draw_text.text,
            draw_text.pos,
            draw_text.font_texture_size,
            draw_text.font_layout_size,
            draw_text.max_width,
        );

        let iter = layout_result
            .glyph_pos_and_uv
            .into_iter()
            .map(|(pos, uv)| RectWithTexture {
                rect: UiRect {
                    pos,
                    uv,
                    color: draw_text.color,
                    border_radius: [0.0, 0.0, 0.0, 0.0],
                },
                texture: RectTexture::Text,
            });

        self.ui_rect_queue.extend(iter);
    }

    /// like drawing text in ui space, but transformed by the transform. one pixel here should be 0.01 units in 3d space.
    pub fn draw_3d_text(&mut self, draw_text: &DrawText, transform: &Transform) {
        let transform_raw = transform.to_raw();

        let layout_result = self.text_rasterizer.layout_and_rasterize_text(
            &draw_text.text,
            draw_text.pos,
            draw_text.font_texture_size,
            draw_text.font_layout_size,
            draw_text.max_width,
        );

        // size of the total region the layout covers
        // (we do this such that text with offset 0,0 and transform 0,0,0 is centered at the 0,0,0 pos instead of having only its top left corner at 0,0,0)
        let layout_size_x = layout_result.total_rect.size[0] * 0.5;
        let layout_size_y = layout_result.total_rect.size[1] * 0.5;
        let center_to_layout = |mut r: Rect| -> Rect {
            r.offset = [r.offset[0] - layout_size_x, r.offset[1] - layout_size_y];
            r
        };

        let iter = layout_result
            .glyph_pos_and_uv
            .into_iter()
            .map(|(pos, uv)| RectWithTexture {
                rect: Rect3D {
                    ui_rect: UiRect {
                        pos: center_to_layout(pos),
                        uv,
                        color: draw_text.color,
                        border_radius: [0.0, 0.0, 0.0, 0.0],
                    },
                    transform: transform_raw,
                },
                texture: RectTexture::Text,
            });

        self.rect_3d_queue.extend(iter);
    }

    pub fn draw_ui_rect(&mut self, ui_rect: RectWithTexture<UiRect>) {
        self.ui_rect_queue.push(ui_rect);
    }

    pub fn draw_3d_rect(&mut self, mut rect_3d: RectWithTexture<Rect3D>) {
        // lets artificially center the rect. Such that if the user provides, transform 0,0,0 and offset 0,
        // the sprite is centered at 0,0,0 and not having only its top left corner at 0,0,0.
        let rect = &mut rect_3d.rect.ui_rect.pos;
        rect.offset[0] -= rect.size[0] * 0.5;
        rect.offset[1] -= rect.size[1] * 0.5;

        self.rect_3d_queue.push(rect_3d);
    }

    pub(crate) fn prepared_ui_rects(&self) -> &PeparedRects<UiRect> {
        &self.prepared_ui_rects
    }

    pub(crate) fn prepared_3d_rects(&self) -> &PeparedRects<Rect3D> {
        &self.prepared_3d_rects
    }

    pub(crate) fn text_atlas_texture(&self) -> &BindableTexture {
        &self.text_rasterizer.atlas_texture()
    }
}

impl Prepare for ImmediateUi {
    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder) {
        let rects = std::mem::take(&mut self.ui_rect_queue);
        self.prepared_ui_rects.prepare(rects, context);

        let rects = std::mem::take(&mut self.rect_3d_queue);
        self.prepared_3d_rects.prepare(rects, context);
    }
}

// pub struct FontDescriptor {
//     size: f32,
//     name: Cow<'static, str>,
// }

// pub struct FontAtlas {
//     font: fontdue::Font,
//     atlas: etagere::AtlasAllocator,
//     atlas_texture: Texture,
// }
