use std::{collections::HashMap, sync::Arc};

use etagere::AtlasAllocator;
use fontdue::{Font, FontSettings};
use glam::{ivec2, IVec2};
use image::{Rgba, RgbaImage};
use wgpu::Queue;

use crate::modules::graphics::{
    elements::texture::{BindableTexture, Texture},
    graphics_context::GraphicsContext,
};

pub const DEFAULT_FONT: &[u8] = include_bytes!("../../../assets/Oswald-Medium.ttf");

pub const TEXT_ATLAS_SIZE: u32 = 2048;

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

    fn offset_and_size(&self) -> OffsetAndSize {
        OffsetAndSize {
            offset: self.offset_in_atlas,
            size: ivec2(self.metrics.width as i32, self.metrics.height as i32),
        }
    }
}

pub struct OffsetAndSize {
    pub offset: IVec2,
    pub size: IVec2,
}

impl TextRasterizer {
    pub fn get_or_rasterize_char(&mut self, glyph_key: &GlyphKey) -> OffsetAndSize {
        if let Some(glyph) = self.glyphs.get(glyph_key) {
            glyph.offset_and_size()
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
            )
            .expect("Failed to rasterize glyph");
            let offset_and_size = glyph.offset_and_size();
            self.glyphs.insert(*glyph_key, glyph);
            offset_and_size
        }
    }

    pub fn atlas_texture(&self) -> &Arc<BindableTexture> {
        &self.atlas_texture
    }

    pub fn default_font(&self) -> FontHandle {
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
        let atlas_texture = Arc::new(BindableTexture::new(
            &context,
            context.rgba_bind_group_layout,
            atlas_texture,
        ));

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
        eprintln!("rasterized glyph for key {:?} is empty!", glyph_key);
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

    // create the glyph
    let glyph = Glyph {
        metrics,
        bitmap,
        offset_in_atlas,
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
