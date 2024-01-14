// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

use std::collections::HashMap;

use etagere::AtlasAllocator;
use fontdue::{
    layout::{
        CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign,
        WrapStyle,
    },
    Font,
};
use glam::{ivec2, IVec2, Vec2};
use image::RgbaImage;

use crate::{
    elements::{BindableTexture, Color, Rect, Texture, Transform},
    modules::{
        arenas::{Key, OwnedKey},
        Arenas, GraphicsContext, Prepare, Renderer,
    },
    Dependencies, Handle, Module,
};

use super::{ui_rect::UiRect, UiRectRenderer, WorldRectRenderer};

// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl TextRenderer {
    pub fn draw_ui_text(&mut self, text: DrawText) {
        let layout_result = self
            .rasterizer
            .layout_and_rasterize_text(&text, &self.deps.arenas);

        for (pos, uv) in layout_result.glyph_pos_and_uv {
            self.deps.ui_rects.draw_textured_rect(
                UiRect {
                    pos,
                    uv,
                    color: text.color,
                    border_radius: [0.0, 0.0, 0.0, 0.0], // todo!() dedicated text renderer without border radius and with sdf?
                },
                self.atlas_texture_key.key(),
            );
        }
    }

    pub fn draw_world_text(&mut self, text: DrawText, transform: Transform) {
        let layout_result = self
            .rasterizer
            .layout_and_rasterize_text(&text, &self.deps.arenas);

        // center the text for 3d rendering:

        let layout_size_x = layout_result.total_rect.width * 0.5;
        let layout_size_y = layout_result.total_rect.height * 0.5;
        let center_to_layout = |mut r: Rect| -> Rect {
            r.min_x -= layout_size_x;
            r.min_y -= layout_size_y;
            r
        };

        for (pos, uv) in layout_result.glyph_pos_and_uv {
            self.deps.world_rects.draw_textured_rect(
                UiRect {
                    pos: center_to_layout(pos),
                    uv,
                    color: text.color,
                    border_radius: [0.0, 0.0, 0.0, 0.0], // todo!() dedicated text renderer without border radius and with sdf font support?
                },
                transform,
                self.atlas_texture_key.key(),
            );
        }
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Module
// /////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Dependencies)]
pub struct Deps {
    ui_rects: Handle<UiRectRenderer>,
    world_rects: Handle<WorldRectRenderer>,
    renderer: Handle<Renderer>,
    ctx: Handle<GraphicsContext>,
    arenas: Handle<Arenas>,
}

pub struct TextRenderer {
    /// A big texture containing all the glyphs that have been rasterized
    atlas_texture_key: OwnedKey<BindableTexture>,
    rasterizer: TextRasterizer,
    deps: Deps,
}

impl Module for TextRenderer {
    type Config = ();

    type Dependencies = Deps;

    fn new(_config: Self::Config, mut deps: Self::Dependencies) -> anyhow::Result<Self> {
        let image = RgbaImage::new(TEXT_ATLAS_SIZE, TEXT_ATLAS_SIZE);
        let atlas_texture = Texture::from_image(&deps.ctx.device, &deps.ctx.queue, &image);
        let atlas_texture = BindableTexture::new(&deps.ctx.device, atlas_texture);
        let atlas_texture_key = deps.arenas.insert(atlas_texture);

        let rasterizer = TextRasterizer::new(&deps.arenas);

        Ok(TextRenderer {
            atlas_texture_key,
            rasterizer,
            deps,
        })
    }

    fn intialize(handle: Handle<Self>) -> anyhow::Result<()> {
        let mut renderer = handle.deps.renderer;
        renderer.register_prepare(handle);
        Ok(())
    }
}

impl Prepare for TextRenderer {
    fn prepare(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) {
        // Update the atlas texture if new glyphs have been rasterized this frame:
        if !self.rasterizer.atlas_texture_writes.is_empty() {
            let atlas_texture = &self.deps.arenas[&self.atlas_texture_key];
            let texture_writes = std::mem::take(&mut self.rasterizer.atlas_texture_writes);
            for glyph_key in texture_writes {
                let glyph = self.rasterizer.glyphs.get(&glyph_key).unwrap();
                let rgba_image = glyph_to_rgba_image(glyph);
                // Note: todo!() this is a bit of a waste to have a 4 channel image with 4x duplicated grey scale values.
                // But this way it works with the general rect render pipeline for now (comfy engine does this too). Should be split up in the future.
                update_texture_region(
                    &atlas_texture.texture,
                    &rgba_image,
                    glyph.offset_in_atlas,
                    queue,
                )
            }
        }
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Renderer
// /////////////////////////////////////////////////////////////////////////////

pub struct DrawText {
    pub text: String,
    pub pos: Vec2,
    pub font_texture_size: f32,
    pub font_layout_size: f32,
    pub max_width: Option<f32>,
    pub color: Color,
}

impl DrawText {
    pub fn new(text: impl Into<String>) -> Self {
        DrawText {
            text: text.into(),
            ..Default::default()
        }
    }

    pub fn pos(mut self, pos: Vec2) -> Self {
        self.pos = pos;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn size(mut self, font_size: f32) -> Self {
        self.font_texture_size = font_size;
        self
    }

    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }
}

impl Default for DrawText {
    fn default() -> Self {
        Self {
            text: "Hello".to_string(),
            pos: Default::default(),
            font_texture_size: 32.0,
            max_width: None,
            color: Color::GREEN,
            font_layout_size: 32.0,
        }
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Rasterization (currently rather primitive implementation with a single atlas, that panics if not more space is available)
// /////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fontsize(pub f32);

impl Eq for Fontsize {}

impl std::hash::Hash for Fontsize {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let int: u32 = unsafe { std::mem::transmute(self.0) };
        int.hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    font: Key<fontdue::Font>,
    size: Fontsize,
    char: char,
}

pub struct Glyph {
    pub metrics: fontdue::Metrics,
    pub bitmap: Vec<u8>,
    pub offset_in_atlas: IVec2,
    /// UV coordinates in the text atlas texture
    pub uv: Rect,
}

pub const DEFAULT_FONT: &[u8] = include_bytes!("../../../../assets/Oswald-Medium.ttf");
pub const TEXT_ATLAS_SIZE: u32 = 2048;
pub const TEXT_ATLAS_SIZE_F: f32 = TEXT_ATLAS_SIZE as f32;

struct TextRasterizer {
    atlas_allocator: etagere::AtlasAllocator,
    /// glyphs that got just updated this frame and need a rewrite in the texture. Should be taken by the text renderer.
    /// Should happen in Prepare stage. They have already been properly allocated in the atlas allocator.
    atlas_texture_writes: Vec<GlyphKey>,
    glyphs: HashMap<GlyphKey, Glyph>,
    default_font_key: OwnedKey<Font>,
}

impl TextRasterizer {
    fn new(arenas: &Handle<Arenas>) -> Self {
        let atlas_allocator = AtlasAllocator::new(etagere::size2(
            TEXT_ATLAS_SIZE as i32,
            TEXT_ATLAS_SIZE as i32,
        ));
        let font = Font::from_bytes(DEFAULT_FONT, fontdue::FontSettings::default())
            .expect("font should be valid");

        let arenas = arenas.get_mut();
        let default_font_key = arenas.insert(font);
        TextRasterizer {
            atlas_allocator,
            atlas_texture_writes: vec![],
            glyphs: HashMap::new(),
            default_font_key,
        }
    }

    fn layout_and_rasterize_text(
        &mut self,
        text: &DrawText,
        arenas: &Handle<Arenas>,
    ) -> LayoutTextResult {
        // todo! this needs rework, once we support more than just one default font.

        let default_font = &arenas[&self.default_font_key];
        // calculate a layout for each glyph in the text:
        let mut layout: Layout<()> = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            x: text.pos.x,
            y: text.pos.y,
            max_width: text.max_width,
            max_height: None,
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            line_height: 1.0,
            wrap_style: WrapStyle::Word,
            wrap_hard_breaks: true,
        });
        layout.append(
            &[default_font],
            &TextStyle {
                text: &text.text,
                px: text.font_layout_size,
                font_index: 0,
                user_data: (),
            },
        );

        // create ui rectangles that point to the correct
        let mut glyph_pos_and_uv: Vec<(Rect, Rect)> = vec![];
        let mut max_x: f32 = text.pos.x; // top left corner
        let mut max_y: f32 = text.pos.y; // top left corner

        for glyph_pos in layout.glyphs() {
            let char = glyph_pos.parent;
            let atlas_uv = self.get_or_rasterize_char(
                default_font,
                &GlyphKey {
                    font: self.default_font_key.key(),
                    size: Fontsize(text.font_texture_size),
                    char,
                },
            );

            max_x = max_x.max(glyph_pos.x + glyph_pos.width as f32);
            max_y = max_y.max(glyph_pos.y + glyph_pos.height as f32);

            if let Some(atlas_uv) = atlas_uv {
                let pos = Rect::new(
                    glyph_pos.x,
                    glyph_pos.y,
                    glyph_pos.width as f32,
                    glyph_pos.height as f32,
                );
                glyph_pos_and_uv.push((pos, atlas_uv));
            }
        }

        LayoutTextResult {
            glyph_pos_and_uv,
            total_rect: Rect::new(
                text.pos.x,
                text.pos.y,
                max_x - text.pos.x,
                max_y - text.pos.y,
            ),
        }
    }

    fn get_or_rasterize_char(&mut self, font: &Font, glyph_key: &GlyphKey) -> Option<Rect> {
        if let Some(glyph) = self.glyphs.get(glyph_key) {
            Some(glyph.uv)
        } else {
            let glyph = rasterize(glyph_key, font, &mut self.atlas_allocator)?;
            self.atlas_texture_writes.push(*glyph_key);
            let uv = glyph.uv;
            self.glyphs.insert(*glyph_key, glyph);
            Some(uv)
        }
    }
}

pub struct LayoutTextResult {
    pub glyph_pos_and_uv: Vec<(Rect, Rect)>,
    // total bounding rect of the text. Can be used e.g. for centering all of the glyphs by shifting them by half the size or so.
    pub total_rect: Rect,
}

// returns None for empty characters
fn rasterize(
    glyph_key: &GlyphKey,
    font: &Font,
    atlas_allocator: &mut AtlasAllocator,
) -> Option<Glyph> {
    let (metrics, bitmap) = font.rasterize(glyph_key.char, glyph_key.size.0);
    debug_assert_eq!(bitmap.len(), metrics.width * metrics.height);
    if metrics.height == 0 || metrics.width == 0 {
        // this happens for example for spaces.
        return None;
    }

    // reserve some space for the rasterized glyph in the shelf allocator:
    let pad: i32 = 2;
    let allocation = atlas_allocator
        .allocate(etagere::size2(
            metrics.width as i32 + 2 * pad,
            metrics.height as i32 + 2 * pad,
        ))
        .expect("Allocation in atlas allocator failed!");
    let corner = allocation.rectangle.min;
    let offset_in_atlas = ivec2(corner.x + pad, corner.y + pad);
    let uv = Rect::new(
        offset_in_atlas.x as f32 / TEXT_ATLAS_SIZE_F,
        offset_in_atlas.y as f32 / TEXT_ATLAS_SIZE_F,
        metrics.width as f32 / TEXT_ATLAS_SIZE_F,
        metrics.height as f32 / TEXT_ATLAS_SIZE_F,
    );

    // create the glyph
    let glyph = Glyph {
        metrics,
        bitmap,
        offset_in_atlas,
        uv,
    };

    Some(glyph)
}

fn update_texture_region(texture: &Texture, image: &RgbaImage, offset: IVec2, queue: &wgpu::Queue) {
    let size = wgpu::Extent3d {
        width: image.width(),
        height: image.height(),
        depth_or_array_layers: 1,
    };

    queue.write_texture(
        wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: &texture.texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: offset.x as u32,
                y: offset.y as u32,
                z: 0,
            },
        },
        image,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * image.width()),
            rows_per_image: None,
        },
        size,
    );
}

fn glyph_to_rgba_image(glyph: &Glyph) -> RgbaImage {
    let mut image = RgbaImage::new(glyph.metrics.width as u32, glyph.metrics.height as u32);
    for (pix, v) in image.pixels_mut().zip(glyph.bitmap.iter()) {
        let pixel = image::Rgba([*v, *v, *v, *v]);
        *pix = pixel;
    }
    image
}
