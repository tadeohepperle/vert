// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl TextRenderer {
    pub fn draw_ui_text(text: DrawText) {
        let mut rasterizer = TEXT_RASTERIZER.get().unwrap().lock().unwrap();
        let layout_result = rasterizer.layout_and_rasterize_text(
            &text.text,
            text.pos,
            text.font_texture_size,
            text.font_layout_size,
            text.max_width,
        );

        let text_texture = TEXT_ATLAS_TEXTURE_KEY.get().unwrap();
        let mut queue = UI_RECT_QUEUE.lock().unwrap();
        for (pos, uv) in layout_result.glyph_pos_and_uv {
            queue.add(
                UiRect {
                    pos,
                    uv,
                    color: text.color,
                    border_radius: [0.0, 0.0, 0.0, 0.0], // todo!() dedicated text renderer without border radius and with sdf?
                },
                *text_texture,
            );
        }
    }

    pub fn draw_3d_text(text: DrawText, transform: Transform) {
        let mut rasterizer = TEXT_RASTERIZER.get().unwrap().lock().unwrap();
        let layout_result = rasterizer.layout_and_rasterize_text(
            &text.text,
            text.pos,
            text.font_texture_size,
            text.font_layout_size,
            text.max_width,
        );

        let text_texture = TEXT_ATLAS_TEXTURE_KEY.get().unwrap();

        let layout_size_x = layout_result.total_rect.size[0] * 0.5;
        let layout_size_y = layout_result.total_rect.size[1] * 0.5;
        let center_to_layout = |mut r: Rect| -> Rect {
            r.offset = [r.offset[0] - layout_size_x, r.offset[1] - layout_size_y];
            r
        };

        let mut queue = WORLD_RECT_QUEUE.lock().unwrap();
        for (pos, uv) in layout_result.glyph_pos_and_uv {
            queue.add(
                WorldRect {
                    ui_rect: UiRect {
                        pos: center_to_layout(pos),
                        uv,
                        color: text.color,
                        border_radius: [0.0, 0.0, 0.0, 0.0], // todo!() dedicated text renderer without border radius
                    },
                    transform: transform.to_raw(),
                },
                *text_texture,
            );
        }
    }
}

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
// Renderer
// /////////////////////////////////////////////////////////////////////////////

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, OnceLock},
};

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

use crate::modules::{
    assets::asset_store::{AssetStore, Key},
    graphics::{
        elements::{
            buffer::ToRaw,
            color::Color,
            texture::{BindableTexture, Texture},
            transform::Transform,
        },
        graphics_context::GraphicsContext,
        PipelineSettings,
    },
};

use super::{
    ui_rect::{Rect, UiRect, UiRectRenderer, UI_RECT_QUEUE},
    world_rect::{WorldRect, WORLD_RECT_QUEUE},
    RendererT,
};

pub const DEFAULT_FONT: &[u8] = include_bytes!("../../../../assets/Oswald-Medium.ttf");

static TEXT_ATLAS_TEXTURE_KEY: OnceLock<Key<BindableTexture>> = OnceLock::new();
static DEFAULT_FONT_KEY: OnceLock<Key<fontdue::Font>> = OnceLock::new();
static TEXT_RASTERIZER: OnceLock<Mutex<TextRasterizer>> = OnceLock::new();

fn initialize_singletons(context: &GraphicsContext) {
    if TEXT_ATLAS_TEXTURE_KEY.get().is_none() {
        let image = RgbaImage::new(TEXT_ATLAS_SIZE, TEXT_ATLAS_SIZE);
        let atlas_texture = Texture::from_image(&context.device, &context.queue, &image);
        let atlas_texture = BindableTexture::new(&context, atlas_texture);
        let key = AssetStore::lock().textures_mut().insert(atlas_texture);
        TEXT_ATLAS_TEXTURE_KEY.set(key).unwrap();
    }

    if DEFAULT_FONT_KEY.get().is_none() {
        let font = Font::from_bytes(DEFAULT_FONT, fontdue::FontSettings::default())
            .expect("font should be valid");
        let key = AssetStore::lock().fonts_mut().insert(font);
        DEFAULT_FONT_KEY.set(key).unwrap();
    }

    if TEXT_RASTERIZER.get().is_none() {
        let atlas_allocator = AtlasAllocator::new(etagere::size2(
            TEXT_ATLAS_SIZE as i32,
            TEXT_ATLAS_SIZE as i32,
        ));
        let rasterizer = TextRasterizer {
            atlas_allocator,
            atlas_texture_writes: vec![],
            glyphs: HashMap::new(),
        };
        _ = TEXT_RASTERIZER.set(Mutex::new(rasterizer));
    }
}

pub struct TextRenderer {}

impl RendererT for TextRenderer {
    fn new(context: &GraphicsContext, settings: PipelineSettings) -> Self
    where
        Self: Sized,
    {
        initialize_singletons(context);
        TextRenderer {}
    }

    /// Only responsible for writes to the Atlas Texture
    fn prepare(
        &mut self,
        context: &crate::modules::graphics::graphics_context::GraphicsContext,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut rasterizer = TEXT_RASTERIZER.get().unwrap().lock().unwrap();
        // Update the atlas texture if new glyphs have been rasterized this frame:
        if !rasterizer.atlas_texture_writes.is_empty() {
            let assets = AssetStore::lock();
            let atlas_texture = assets
                .textures()
                .get(*TEXT_ATLAS_TEXTURE_KEY.get().unwrap())
                .unwrap();
            let texture_writes = std::mem::take(&mut rasterizer.atlas_texture_writes);
            for glyph_key in texture_writes {
                let glyph = rasterizer.glyphs.get(&glyph_key).unwrap();
                let rgba_image = glyph_to_rgba_image(glyph);
                // Note: todo!() this is a bit of a waste to have a 4 channel image with 4x duplicated grey scale values.
                // But this way it works with the general rect render pipeline for now (comfy engine does this too). Should be split up in the future.
                update_texture_region(
                    &atlas_texture.texture,
                    &rgba_image,
                    glyph.offset_in_atlas,
                    &context.queue,
                )
            }
        }
    }

    fn render<'pass, 'encoder>(
        &'encoder self,
        _render_pass: &'pass mut wgpu::RenderPass<'encoder>,
        _graphics_settings: &crate::modules::graphics::settings::GraphicsSettings,
        _asset_store: &'encoder crate::modules::assets::asset_store::AssetStore<'encoder>,
    ) {
        // delegate all the work to the text renderer at the moment
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Rasterization (currently rather primitive implementation with a single atlas, that panics if not more space is available)
// /////////////////////////////////////////////////////////////////////////////

pub const TEXT_ATLAS_SIZE: u32 = 2048;
pub const TEXT_ATLAS_SIZE_F: f32 = TEXT_ATLAS_SIZE as f32;

struct TextRasterizer {
    atlas_allocator: etagere::AtlasAllocator,
    /// glyphs that got just updated this frame and need a rewrite in the texture. Should be taken by the text renderer.
    /// Should happen in Prepare stage. They have already been properly allocated in the atlas allocator.
    atlas_texture_writes: Vec<GlyphKey>,
    glyphs: HashMap<GlyphKey, Glyph>,
}

impl TextRasterizer {
    fn new() -> Self {
        let atlas_allocator = AtlasAllocator::new(etagere::size2(
            TEXT_ATLAS_SIZE as i32,
            TEXT_ATLAS_SIZE as i32,
        ));
        TextRasterizer {
            atlas_allocator,
            atlas_texture_writes: vec![],
            glyphs: HashMap::new(),
        }
    }

    fn layout_and_rasterize_text(
        &mut self,
        text: &String,
        pos: Vec2,
        font_texture_size: f32,
        font_layout_size: f32,
        max_width: Option<f32>,
    ) -> LayoutTextResult {
        // todo! this needs rework, once we support more than just one default font.
        let assets = AssetStore::lock();
        let default_font = assets
            .fonts()
            .get(*DEFAULT_FONT_KEY.get().unwrap())
            .unwrap();
        // calculate a layout for each glyph in the text:
        let mut layout: Layout<()> = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            x: pos.x,
            y: pos.y,
            max_width,
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
                text,
                px: font_layout_size,
                font_index: 0,
                user_data: (),
            },
        );

        // create ui rectangles that point to the correct
        let mut glyph_pos_and_uv: Vec<(Rect, Rect)> = vec![];
        let mut max_x: f32 = pos.x; // top left corner
        let mut max_y: f32 = pos.y; // top left corner

        for glyph_pos in layout.glyphs() {
            let char = glyph_pos.parent;
            let atlas_uv = self.get_or_rasterize_char(
                default_font,
                &GlyphKey {
                    font: *DEFAULT_FONT_KEY.get().unwrap(),
                    size: Fontsize(font_texture_size),
                    char,
                },
            );

            max_x = max_x.max(glyph_pos.x + glyph_pos.width as f32);
            max_y = max_y.max(glyph_pos.y + glyph_pos.height as f32);

            if let Some(atlas_uv) = atlas_uv {
                let pos = Rect {
                    offset: [glyph_pos.x, glyph_pos.y],
                    size: [glyph_pos.width as f32, glyph_pos.height as f32],
                };
                glyph_pos_and_uv.push((pos, atlas_uv));
            }
        }

        LayoutTextResult {
            glyph_pos_and_uv,
            total_rect: Rect {
                offset: [pos.x, pos.y],
                size: [max_x - pos.x, max_y - pos.y],
            },
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

// /////////////////////////////////////////////////////////////////////////////
// Data
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

// /////////////////////////////////////////////////////////////////////////////
// Rasterization
// /////////////////////////////////////////////////////////////////////////////

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
    let uv = Rect {
        offset: [
            offset_in_atlas.x as f32 / TEXT_ATLAS_SIZE_F,
            offset_in_atlas.y as f32 / TEXT_ATLAS_SIZE_F,
        ],
        size: [
            metrics.width as f32 / TEXT_ATLAS_SIZE_F,
            metrics.height as f32 / TEXT_ATLAS_SIZE_F,
        ],
    };

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
        &image,
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
