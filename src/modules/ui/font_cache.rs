use anyhow::anyhow;
use etagere::AtlasAllocator;
use fontdue::Font;
use glam::{ivec2, IVec2};
use image::{GenericImage, RgbaImage};
use std::collections::HashMap;

use crate::{
    elements::{rect::Aabb, BindableTexture, Texture},
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
    cached_fonts: Arena<CachedFont>,
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
            cached_fonts,
        })
    }
}

impl FontCache {
    pub fn add_font(&mut self, font: fontdue::Font) -> Key<Font> {
        self.fonts.insert(font)
    }

    pub fn rasterize_default_font(&mut self, font_size_px: f32) -> anyhow::Result<Key<CachedFont>> {
        self.rasterize_font(self.default_font_key, font_size_px)
    }

    pub fn rasterize_font(
        &mut self,
        font_key: Key<Font>,
        font_size_px: f32,
    ) -> anyhow::Result<Key<CachedFont>> {
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
                uv,
            };
            rasterized_glyphs.insert(ch, glyph);
        }

        // create a texture that contains the atlas image.
        let texture = Texture::from_image(&self.ctx.device, &self.ctx.queue, &atlas_image);
        let texture = BindableTexture::new(&self.ctx.device, texture);

        let cached_font = CachedFont {
            font: font_key,
            font_size_px,
            atlas_allocator,
            rasterized_glyphs,
            texture,
        };
        let key = self.cached_fonts.insert(cached_font);
        Ok(key)
    }
}

pub struct CachedFont {
    pub font: Key<Font>,
    pub font_size_px: f32,
    pub atlas_allocator: etagere::AtlasAllocator,
    pub rasterized_glyphs: HashMap<char, Glyph>,
    pub texture: BindableTexture,
}

pub struct Glyph {
    pub metrics: fontdue::Metrics,
    pub bitmap: Vec<u8>,
    /// minx and miny in px in the atlas texture.
    pub offset_in_atlas: IVec2,
    /// UV coordinates in the text atlas texture (always in range 0.0 to 1.0)
    pub uv: Aabb,
}
