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
        texture::{BindableTexture, Texture},
        ui_rect::{Rect, UiRect, UiRectInstance, UiRectTexture},
    },
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

    fn offset_and_size(&self) -> Rect {
        Rect {
            offset: self.offset_in_atlas.as_vec2(),
            size: vec2(self.metrics.width as f32, self.metrics.height as f32),
        }
    }
}

impl TextRasterizer {
    pub fn get_or_rasterize_char(&mut self, glyph_key: &GlyphKey) -> Rect {
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

    // pub fn get_layout(&self) {
    //     let mut layout = Layout::<f32>::new(CoordinateSystem::PositiveYUp);

    //     layout.glyphs();
    //     // layout.reset(&LayoutSettings{ x: todo!(), y: todo!(), max_width: todo!(), max_height: todo!(), horizontal_align: todo!(), vertical_align: todo!(), line_height: todo!(), wrap_style: todo!(), wrap_hard_breaks: todo!() })
    //     // let font = self.fonts.get(&self.default_font).unwrap();
    //     // layout.append(&[font], &TextStyle::new("Hello ", 35.0, 0));
    // }

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

    pub(crate) fn draw_text(&self, draw_text: &DrawText) -> Vec<UiRect> {
        // rasterize all characters in the text (if not done so for the respective character):

        // calculate a layout for each glyph in the text:
        let mut layout: Layout<()> = Layout::new(CoordinateSystem::PositiveYUp);
        layout.reset(&LayoutSettings {
            x: draw_text.pos.x,
            y: draw_text.pos.y,
            max_width: draw_text.max_width,
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
                text: &draw_text.text,
                px: draw_text.font_size,
                font_index: 0,
                user_data: (),
            },
        );

        todo!()

        // // create ui rectangles that point to the correct
        // let ui_rects = layout
        //     .glyphs()
        //     .iter()
        //     .map(|g| {
        //         let char = g.parent;
        //         let atlas_pos = self.get_or_rasterize_char(&GlyphKey {
        //             font: self.default_font,
        //             size: Fontsize(draw_text.font_size),
        //             char,
        //         });
        //         // self.glyphs.get(char).unwrap();
        //         todo!()
        //         // UiRect {
        //         //     instance: UiRectInstance {
        //         //         posbb: ,
        //         //         uvbb: [atlas_pos.offset.x, atlas_pos.offset.y,],
        //         //         color: todo!(),
        //         //         border_radius: todo!(),
        //         //     },
        //         //     texture: UiRectTexture::Text,
        //         // }
        //     })
        //     .collect();

        // ui_rects
    }

    fn default_font(&self) -> &Font {
        self.fonts.get(&self.default_font).unwrap()
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

pub struct DrawText {
    text: String,
    pos: Vec2,
    font_size: f32,
    max_width: Option<f32>,
    color: Color,
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
}

impl Default for DrawText {
    fn default() -> Self {
        Self {
            text: "Hello".to_string(),
            pos: Default::default(),
            font_size: 24.0,
            max_width: None,
            color: Color::GREEN,
        }
    }
}
