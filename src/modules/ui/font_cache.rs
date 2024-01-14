use etagere::{AllocId, AtlasAllocator};
use fontdue::{
    layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle},
    Font,
};
use glam::{ivec2, IVec2};
use image::RgbaImage;

use std::collections::HashMap;

use crate::{
    elements::{rect::Aabb, BindableTexture, Rect, Texture},
    modules::{
        arenas::{Key, OwnedKey},
        Arenas, GraphicsContext, Prepare, Renderer,
    },
    Dependencies, Handle, Module,
};

// const PREALLOCATED_CHARACTERS: &str =
//     "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890,./\\?<>{}[]!@#$%^&*()_-=+|~` \n\tÄäÖöÜüß";

#[derive(Debug, Dependencies)]
pub struct Deps {
    ctx: Handle<GraphicsContext>,
    renderer: Handle<Renderer>,
    arenas: Handle<Arenas>,
}

const ATLAS_SIZE: u32 = 4096;

/// Todo! Currently no glyph cleanup, which is quite bad. Glyph cleanup can be pretty hard though.
pub struct FontCache {
    deps: Deps,
    atlas_texture: OwnedKey<BindableTexture>,
    atlas_allocator: AtlasAllocator,
    default_font: OwnedKey<Font>,
    glyphs: HashMap<GlyphKey, Glyph>,
    texture_writes: Vec<GlyphKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    font: Key<fontdue::Font>,
    font_size: FontSize,
    char: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontSize(pub u32);

impl From<f32> for FontSize {
    fn from(value: f32) -> Self {
        if value < 0.0 {
            panic!("Cannot create Fontsize from negative number")
        }
        Self(value as u32)
    }
}

impl From<u32> for FontSize {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<FontSize> for f32 {
    fn from(value: FontSize) -> Self {
        value.0 as f32
    }
}

struct Glyph {
    /// currently not used, but could be used to deallocate the glyph from the shelf atlas.
    _alloc_id: AllocId,
    metrics: fontdue::Metrics,
    bitmap: Vec<u8>,
    /// minx and miny in px in the atlas texture.
    offset_in_atlas: IVec2,
    /// UV coordinates in the text atlas texture (always in range 0.0 to 1.0)
    atlas_uv: Aabb,
}

impl Module for FontCache {
    type Config = ();
    type Dependencies = Deps;

    fn new(_config: Self::Config, mut deps: Self::Dependencies) -> anyhow::Result<Self> {
        const DEFAULT_FONT_BYTES: &[u8] = include_bytes!("../../../assets/Oswald-Medium.ttf");
        let default_font = fontdue::Font::from_bytes(DEFAULT_FONT_BYTES, Default::default())
            .expect("could not load default font");

        let default_font = deps.arenas.insert(default_font);

        let atlas_width: u32 = ATLAS_SIZE;
        let atlas_height: u32 = ATLAS_SIZE;
        let atlas_allocator =
            AtlasAllocator::new(etagere::size2(atlas_width as i32, atlas_height as i32));

        let image = RgbaImage::new(atlas_width, atlas_height);
        let atlas_texture = Texture::from_image(&deps.ctx.device, &deps.ctx.queue, &image);
        let atlas_texture = BindableTexture::new(&deps.ctx.device, atlas_texture);
        let atlas_texture = deps.arenas.insert(atlas_texture);

        Ok(FontCache {
            deps,
            default_font,
            atlas_texture,
            atlas_allocator,
            glyphs: HashMap::new(),
            texture_writes: vec![],
        })
    }

    fn intialize(handle: Handle<Self>) -> anyhow::Result<()> {
        let mut renderer = handle.deps.renderer;
        renderer.register_prepare(handle);

        Ok(())
    }
}

impl Prepare for FontCache {
    fn prepare(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) {
        let atlas_texture = &self.deps.arenas[&self.atlas_texture];
        for key in self.texture_writes.iter() {
            let glyph = self.glyphs.get(key).unwrap();
            let glyph_image = glyph_to_rgba_image(glyph);
            update_texture_region(
                &atlas_texture.texture,
                &glyph_image,
                glyph.offset_in_atlas,
                queue,
            );
        }
        self.texture_writes.clear();
    }
}

impl FontCache {
    pub fn default_font(&self) -> Key<Font> {
        self.default_font.key()
    }

    pub fn atlas_texture(&self) -> Key<BindableTexture> {
        self.atlas_texture.key()
    }

    // pub fn atlas_texture_obj(&self) -> &BindableTexture {
    //     &self.deps.arenas[&self.atlas_texture]
    // }

    /// Returns non if there is no glyph that can be assigned to the char (e.g. for space)
    fn get_glyph_atlas_uv_or_rasterize(&mut self, key: GlyphKey) -> Option<Aabb> {
        if let Some(glyph) = self.glyphs.get_mut(&key) {
            return Some(glyph.atlas_uv);
        }

        let font = &self.deps.arenas[key.font];
        let (metrics, bitmap) = font.rasterize(key.char, key.font_size.into());
        debug_assert_eq!(bitmap.len(), metrics.width * metrics.height);

        if metrics.height == 0 || metrics.width == 0 {
            return None;
        }

        // pixel padding in texture atlas around rasterized fonts
        const PAD_PX: usize = 2;
        // find some space where to put this glyph
        let allocation = self
            .atlas_allocator
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
            offset_in_atlas.x as f32 / ATLAS_SIZE as f32,
            offset_in_atlas.y as f32 / ATLAS_SIZE as f32,
            (offset_in_atlas.x + metrics.width as i32) as f32 / ATLAS_SIZE as f32,
            (offset_in_atlas.y + metrics.height as i32) as f32 / ATLAS_SIZE as f32,
        );

        self.texture_writes.push(key);
        // store glyph:
        let glyph = Glyph {
            metrics,
            bitmap,
            offset_in_atlas,
            atlas_uv: uv,
            _alloc_id: allocation.id,
        };
        self.glyphs.insert(key, glyph);
        Some(uv)
    }

    /// if layout_font_size_px is None, the size at which the font was rasterized font is used for layout
    pub fn perform_text_layout(
        &mut self,
        text: &str,
        font_size: FontSize,
        layout_font_size: f32,
        layout_settings: &LayoutSettings,
        font: Option<Key<Font>>,
    ) -> TextLayoutResult {
        // Note: (layout_settings.x, layout_settings.y) is the top left corner where the text starts.
        let font = font.unwrap_or_else(|| self.default_font.key());
        let font_obj = &self.deps.arenas[font];

        let mut layout: Layout<()> = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(layout_settings);
        // this performs the layout:
        layout.append(
            &[font_obj],
            &TextStyle {
                text,
                px: layout_font_size,
                font_index: 0,
                user_data: (),
            },
        );

        let mut glyph_pos_and_atlas_uv: Vec<(Aabb, Aabb)> = vec![];
        let mut max_x: f32 = layout_settings.x; // top left corner
        let mut max_y: f32 = layout_settings.y; // top left corner

        for glyph_pos in layout.glyphs() {
            let key = GlyphKey {
                font,
                font_size,
                char: glyph_pos.parent,
            };
            let Some(glyph_uv) = self.get_glyph_atlas_uv_or_rasterize(key) else {
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
            glyph_pos_and_atlas_uv.push((pos, glyph_uv));
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
    /// Todo! make pos a rect instead, because it is easier to add to it.
    pub glyph_pos_and_atlas_uv: Vec<(Aabb, Aabb)>,
    // total bounding rect of the text. Can be used e.g. for centering all of the glyphs by shifting them by half the size or so.
    pub total_rect: Rect,
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
