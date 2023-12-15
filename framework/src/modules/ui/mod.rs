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
        rect::{PeparedRects, RectTexture, RectWithTexture},
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
    pub fn draw_text(&mut self, text: &DrawText) {
        let rects = self.text_rasterizer.draw_ui_text(text);
        self.ui_rect_queue.extend(rects);
    }

    /// like drawing text in ui space, but transformed by the transform. one pixel here should be 0.01 units in 3d space.
    pub fn draw_3d_text(&mut self, text: &DrawText, transform: &Transform) {
        let transform_raw = transform.to_raw();
        let rects = self.text_rasterizer.draw_ui_text(text);
        let rects_3d = rects.into_iter().map(|e| RectWithTexture {
            instance: Rect3D {
                ui_rect: e.instance,
                transform: transform_raw,
            },
            texture: e.texture,
        });

        self.rect_3d_queue.extend(rects_3d);
    }

    pub fn draw_ui_rect(&mut self, ui_rect: RectWithTexture<UiRect>) {
        self.ui_rect_queue.push(ui_rect);
    }

    pub fn draw_3d_rect(&mut self, rect_3d: RectWithTexture<Rect3D>) {
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
