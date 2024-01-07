use anyhow::anyhow;
use etagere::AtlasAllocator;
use fontdue::{
    layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle},
    Font,
};
use glam::{ivec2, IVec2};
use image::{GenericImage, RgbaImage};
use std::collections::HashMap;

use crate::{
    elements::{rect::Aabb, BindableTexture, Rect, Texture},
    modules::{
        arenas::{Arena, Key},
        GraphicsContext,
    },
    utils::next_pow2_number,
    Handle, Module,
};

// supporting 64 different characters at the moment, hard coded, change later :)
const N_X_CHARACTERS: usize = 16;
const N_Y_CHARACTERS: usize = 8;
// pixel padding in texture atlas around rasterized fonts
const PAD_PX: usize = 2;

const PREALLOCATED_CHARACTERS: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890,./\\?<>{}[]!@#$%^&*()_-=+|~` \n\tÄäÖöÜüß";

pub struct FontCache {
    ctx: Handle<GraphicsContext>,
    fonts: Arena<Font>,
    default_font_key: Key<Font>,
    rasterized_fonts: Arena<RasterizedFont>,
}

impl Module for FontCache {
    type Config = ();
    type Dependencies = Handle<GraphicsContext>;

    fn new(config: Self::Config, ctx: Self::Dependencies) -> anyhow::Result<Self> {
        const DEFAULT_FONT_BYTES: &[u8] = include_bytes!("../../../assets/Oswald-Medium.ttf");
        let default_font = fontdue::Font::from_bytes(DEFAULT_FONT_BYTES, Default::default())
            .expect("could not load default font");

        let mut fonts = Arena::new();
        let default_font_key = fonts.insert(default_font);
        let cached_fonts = Arena::new();
        Ok(FontCache {
            ctx,
            fonts,
            default_font_key,
            rasterized_fonts: cached_fonts,
        })
    }
}

impl FontCache {
    pub fn get_font(&self, key: Key<Font>) -> Option<&Font> {
        self.fonts.get(key)
    }

    pub fn get_rasterized_font(&self, key: Key<RasterizedFont>) -> Option<&RasterizedFont> {
        self.rasterized_fonts.get(key)
    }

    pub fn default_font_key(&self) -> Key<Font> {
        self.default_font_key
    }

    pub fn add_font(&mut self, font: fontdue::Font) -> Key<Font> {
        self.fonts.insert(font)
    }

    pub fn rasterize_default_font(
        &mut self,
        font_size_px: f32,
    ) -> anyhow::Result<Key<RasterizedFont>> {
        self.rasterize_font(self.default_font_key, font_size_px)
    }

    pub fn rasterize_font(
        &mut self,
        font_key: Key<Font>,
        font_size_px: f32,
    ) -> anyhow::Result<Key<RasterizedFont>> {
        let Some(font) = self.fonts.get(font_key) else {
            return Err(anyhow!("Font key {font_key} is invalid"));
        };

        // The max number of characters that will fit into the atlas allocator.
        let mut max_char_width: usize = 0;
        let mut max_char_height: usize = 0;

        // 256 to get all ascii characters.
        for i in 0..=255u8 {
            let c = i as char;
            let m = font.metrics(c, font_size_px);

            if m.width > max_char_width {
                max_char_width = m.width;
            }
            if m.height > max_char_height {
                max_char_height = m.height;
            }
        }

        let atlas_width: usize = next_pow2_number((max_char_width + PAD_PX * 2) * N_X_CHARACTERS);
        let atlas_height: usize = next_pow2_number((max_char_height + PAD_PX * 2) * N_Y_CHARACTERS);
        let mut atlas_allocator =
            AtlasAllocator::new(etagere::size2(atlas_width as i32, atlas_height as i32));
        let mut atlas_image = RgbaImage::new(atlas_width as u32, atlas_height as u32);
        let mut rasterized_glyphs: HashMap<char, Glyph> = HashMap::new();

        // lets just rasterize all glyphs upfront, no rasterization will ever take place after that.
        // We can implement a fancier approach later (if e.g. chinese should be supported, we need something better anyway.)
        for ch in PREALLOCATED_CHARACTERS.chars() {
            let (metrics, bitmap) = font.rasterize(ch, font_size_px);
            debug_assert_eq!(bitmap.len(), metrics.width * metrics.height);

            if metrics.height == 0 || metrics.width == 0 {
                continue;
            }

            let mut glyph_image = RgbaImage::new(metrics.width as u32, metrics.height as u32);
            for (pix, v) in glyph_image.pixels_mut().zip(bitmap.iter()) {
                *pix = image::Rgba([255, 255, 255, *v]);
            }

            // find some space where to put this glyph
            let allocation = atlas_allocator
                .allocate(etagere::size2(
                    (metrics.width + 2 * PAD_PX) as i32,
                    (metrics.height + 2 * PAD_PX) as i32,
                ))
                .expect("Allocation in atlas allocator failed!");
            let offset_in_atlas = ivec2(
                allocation.rectangle.min.x + PAD_PX as i32,
                allocation.rectangle.min.y + PAD_PX as i32,
            );
            let uv = Aabb::new(
                offset_in_atlas.x as f32 / atlas_width as f32,
                offset_in_atlas.y as f32 / atlas_height as f32,
                (allocation.rectangle.max.x + PAD_PX as i32) as f32 / atlas_width as f32,
                (allocation.rectangle.max.y + PAD_PX as i32) as f32 / atlas_height as f32,
            );

            // copy glyph image to the right region of the texture.
            atlas_image
                .copy_from(
                    &glyph_image,
                    offset_in_atlas.x as u32,
                    offset_in_atlas.y as u32,
                )
                .expect("image copy should work");

            // store glyph:
            let glyph = Glyph {
                metrics,
                bitmap,
                offset_in_atlas,
                atlas_uv: uv,
            };
            rasterized_glyphs.insert(ch, glyph);
        }

        // create a texture that contains the atlas image.
        let texture = Texture::from_image(&self.ctx.device, &self.ctx.queue, &atlas_image);
        let texture = BindableTexture::new(&self.ctx.device, texture);

        let cached_font = RasterizedFont {
            font: font_key,
            font_size_px,
            atlas_allocator,
            glyphs: rasterized_glyphs,
            texture,
        };
        let key = self.rasterized_fonts.insert(cached_font);
        Ok(key)
    }

    /// if layout_font_size_px is None, the size at which the font was rasterized font is used for layout
    pub fn perform_text_layout(
        &self,
        text: &str,
        layout_font_size_px: Option<f32>,
        layout_settings: &LayoutSettings,
        font: Key<RasterizedFont>,
    ) -> TextLayoutResult {
        // Note: (layout_settings.x, layout_settings.y) is the top left corner where the text starts.
        let rasterized_font = self
            .rasterized_fonts
            .get(font)
            .expect("Rasterized Font not found");
        let font = self
            .fonts
            .get(rasterized_font.font)
            .expect("Rasterized Font not found");

        let mut layout: Layout<()> = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(layout_settings);
        // this performs the layout:
        layout.append(
            &[font],
            &TextStyle {
                text,
                px: layout_font_size_px.unwrap_or_else(|| rasterized_font.font_size_px),
                font_index: 0,
                user_data: (),
            },
        );

        let mut glyph_pos_and_atlas_uv: Vec<(Aabb, Aabb)> = vec![];
        let mut max_x: f32 = layout_settings.x; // top left corner
        let mut max_y: f32 = layout_settings.y; // top left corner

        for glyph_pos in layout.glyphs() {
            let char = glyph_pos.parent;
            let Some(glyph) = rasterized_font.glyphs.get(&char) else {
                // empty character, or unknown character, just skip// todo!(warn user if non empty cahr skipped)
                continue;
            };

            max_x = max_x.max(glyph_pos.x + glyph_pos.width as f32);
            max_y = max_y.max(glyph_pos.y + glyph_pos.height as f32);

            let pos = Aabb::new(
                glyph_pos.x,
                glyph_pos.y,
                glyph_pos.x + glyph_pos.width as f32,
                glyph_pos.y + glyph_pos.height as f32,
            );
            glyph_pos_and_atlas_uv.push((pos, glyph.atlas_uv));
        }

        TextLayoutResult {
            glyph_pos_and_atlas_uv,
            total_rect: Rect::new(
                layout_settings.x,
                layout_settings.y,
                max_x - layout_settings.x,
                max_y - layout_settings.y,
            ),
        }
    }
}

#[derive(Debug)]
pub struct TextLayoutResult {
    /// glyph position and their uv position in the texture atlas
    pub glyph_pos_and_atlas_uv: Vec<(Aabb, Aabb)>,
    // total bounding rect of the text. Can be used e.g. for centering all of the glyphs by shifting them by half the size or so.
    pub total_rect: Rect,
}

pub struct RasterizedFont {
    pub font: Key<Font>,
    pub font_size_px: f32,
    pub atlas_allocator: etagere::AtlasAllocator,
    pub glyphs: HashMap<char, Glyph>,
    pub texture: BindableTexture,
}

pub struct Glyph {
    pub metrics: fontdue::Metrics,
    pub bitmap: Vec<u8>,
    /// minx and miny in px in the atlas texture.
    pub offset_in_atlas: IVec2,
    /// UV coordinates in the text atlas texture (always in range 0.0 to 1.0)
    pub atlas_uv: Aabb,
}
