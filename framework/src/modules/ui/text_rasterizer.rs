use std::{borrow::Cow, collections::HashMap, sync::Arc};

use egui::Widget;
use etagere::AtlasAllocator;
use fontdue::{
    layout::{
        CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign,
        WrapStyle,
    },
    Font, FontSettings,
};
use glam::{ivec2, vec2, IVec2, Vec2};
use image::{Rgba, RgbaImage};
use wgpu::Queue;

use crate::modules::graphics::{
    elements::{
        color::Color,
        rect::{Rect, RectTexture, RectWithTexture},
        texture::{BindableTexture, Texture},
        ui_rect::UiRect,
    },
    graphics_context::GraphicsContext,
    statics::static_texture::RgbaBindGroupLayout,
};

pub const DEFAULT_FONT: &[u8] = include_bytes!("../../../assets/Oswald-Medium.ttf");

pub const TEXT_ATLAS_SIZE: u32 = 2048;
pub const TEXT_ATLAS_SIZE_F: f32 = TEXT_ATLAS_SIZE as f32;

pub struct TextRasterizer {
    context: GraphicsContext,

    atlas_allocator: etagere::AtlasAllocator,
    atlas_texture: Arc<BindableTexture>,
    /// here the u32 is the pixel size:
    glyphs: HashMap<GlyphKey, Glyph>,
    default_font: FontHandle,
    next_font_handle: FontHandle,
    fonts: HashMap<FontHandle, fontdue::Font>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontHandle(u32);

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
    font: FontHandle,
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

impl Glyph {
    fn create_rgba_image(&self) -> RgbaImage {
        let mut image = RgbaImage::new(self.metrics.width as u32, self.metrics.height as u32);
        for (pix, v) in image.pixels_mut().zip(self.bitmap.iter()) {
            let pixel = image::Rgba([*v, *v, *v, *v]);
            *pix = pixel;
        }
        image
    }
}

impl TextRasterizer {
    /// returns None for e.g. empty characters.
    pub fn get_or_rasterize_char(&mut self, glyph_key: &GlyphKey) -> Option<Rect> {
        if let Some(glyph) = self.glyphs.get(glyph_key) {
            Some(glyph.uv)
        } else {
            let font = self
                .fonts
                .get(&glyph_key.font)
                .expect("Cannot rasterize font with invalid font handle!");
            let glyph = rasterize(
                glyph_key,
                font,
                &mut self.atlas_allocator,
                &self.atlas_texture.texture,
                &self.context.queue,
            )?;
            let uv = glyph.uv;
            self.glyphs.insert(*glyph_key, glyph);
            Some(uv)
        }
    }

    pub fn atlas_texture(&self) -> &Arc<BindableTexture> {
        &self.atlas_texture
    }

    pub fn default_font_handle(&self) -> FontHandle {
        self.default_font
    }

    pub fn add_font(&mut self, font: Font) -> FontHandle {
        let handle = self.next_font_handle;
        self.fonts.insert(handle, font);
        self.next_font_handle = FontHandle(handle.0 + 1);
        return handle;
    }

    pub(super) fn new(context: GraphicsContext) -> TextRasterizer {
        let atlas_allocator = AtlasAllocator::new(etagere::size2(
            TEXT_ATLAS_SIZE as i32,
            TEXT_ATLAS_SIZE as i32,
        ));

        // create a fully transparent texture.
        let image = RgbaImage::new(TEXT_ATLAS_SIZE, TEXT_ATLAS_SIZE);
        let atlas_texture = Texture::from_image(&context.device, &context.queue, &image);
        let atlas_texture = Arc::new(BindableTexture::new(&context, atlas_texture));

        let font = Font::from_bytes(DEFAULT_FONT, fontdue::FontSettings::default())
            .expect("font should be valid");
        let default_font = FontHandle(0);
        let fonts = [(default_font, font)].into();

        let rasterizer = TextRasterizer {
            context,
            atlas_allocator,
            atlas_texture,
            glyphs: HashMap::new(),
            default_font,
            next_font_handle: FontHandle(1),
            fonts,
        };

        rasterizer
    }

    /// lays out the text giving back the positions and uv positions in the texture atlas of all glyphs.
    ///
    /// If encountering glyphs not yet rasterized to the texture, it writes them into the text atlas texture.
    pub fn layout_and_rasterize_text(
        &mut self,
        text: &String,
        pos: Vec2,
        font_texture_size: f32,
        font_layout_size: f32,
        max_width: Option<f32>,
    ) -> LayoutTextResult {
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
        let default_font = self.default_font();
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
            let atlas_uv = self.get_or_rasterize_char(&GlyphKey {
                font: self.default_font,
                size: Fontsize(font_texture_size),
                char,
            });

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

    fn default_font(&self) -> &Font {
        self.fonts.get(&self.default_font).unwrap()
    }
}

pub struct LayoutTextResult {
    pub glyph_pos_and_uv: Vec<(Rect, Rect)>,
    // total bounding rect of the text. Can be used e.g. for centering all of the glyphs by shifting them by half the size or so.
    pub total_rect: Rect,
}

fn rasterize(
    glyph_key: &GlyphKey,
    font: &Font,
    atlas_allocator: &mut AtlasAllocator,
    atlas_texture: &Texture,
    queue: &wgpu::Queue,
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

    // write the glyph to the big texture:
    // Note: todo! this is a bit of a waste to have a 4 channel image with 4x duplicated grey scale values.
    // But this way it works with the general rect render pipeline for now (comfy engine does this too). Should be split up in the future.
    let image = glyph.create_rgba_image();
    update_texture_region(&atlas_texture, &image, offset_in_atlas, &queue);

    Some(glyph)
}

fn update_texture_region(texture: &Texture, image: &RgbaImage, offset: IVec2, queue: &Queue) {
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
