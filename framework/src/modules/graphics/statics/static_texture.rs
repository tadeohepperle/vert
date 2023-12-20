use std::sync::OnceLock;

use image::RgbaImage;
use wgpu::{BindGroup, BindGroupLayout};

use crate::modules::graphics::{
    elements::texture::{BindableTexture, Texture},
    graphics_context::GraphicsContext,
};

use super::StaticBindGroup;

pub struct RgbaBindGroupLayout;

impl RgbaBindGroupLayout {
    /// Must be initialized before
    pub fn get(&self) -> &'static BindGroupLayout {
        _RGBA_BIND_GROUP_LAYOUT
            .get()
            .expect("RgbaBindGroupLayout not initialized")
    }

    pub fn get_multisampled(&self) -> &'static BindGroupLayout {
        _RGBA_BIND_GROUP_LAYOUT_MSAA4
            .get()
            .expect("RgbaBindGroupLayout MSSA4 not initialized")
    }
}

static _RGBA_BIND_GROUP_LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
static _RGBA_BIND_GROUP_LAYOUT_MSAA4: OnceLock<BindGroupLayout> = OnceLock::new();

/// # CALL ONLY ONCE!
///
/// todo!() this is all a bit crazy, fix this later.
pub fn initialize_static_textures(context: &GraphicsContext) {
    let layout = context
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
    _RGBA_BIND_GROUP_LAYOUT
        .set(layout)
        .expect("_RGBA_BIND_GROUP_LAYOUT not initializable");

    let layout_mssa4 = context
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: false, // filterable needs to be false for multisampled textures.
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
    _RGBA_BIND_GROUP_LAYOUT_MSAA4
        .set(layout_mssa4)
        .expect("_RGBA_BIND_GROUP_LAYOUT MSAA4 not initializable");

    WhitePxTexture::initialize(context)
}

pub fn rgba_bind_group_layout(
    device: &wgpu::Device,
    multisampled: bool,
) -> &'static BindGroupLayout {
    static RGBA_BIND_GROUP_LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
    static RGBA_BIND_GROUP_LAYOUT_MULTISAMPLED: OnceLock<BindGroupLayout> = OnceLock::new();

    let layout = if multisampled {
        &RGBA_BIND_GROUP_LAYOUT_MULTISAMPLED
    } else {
        &RGBA_BIND_GROUP_LAYOUT
    };

    layout.get_or_init(|| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: !multisampled, // filterable needs to be false for multisampled textures.
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    })
}

pub struct WhitePxTexture;
static _WHITE_PX_TEXTURE: OnceLock<BindableTexture> = OnceLock::new();

impl WhitePxTexture {
    pub fn initialize(context: &GraphicsContext) {
        let mut white_px = RgbaImage::new(1, 1);
        white_px.get_pixel_mut(0, 0).0 = [255, 255, 255, 255];
        let texture = Texture::from_image(&context.device, &context.queue, &white_px);
        let bindable_texture = BindableTexture::new(context, texture);
        _WHITE_PX_TEXTURE.set(bindable_texture);
    }
}

impl StaticBindGroup for WhitePxTexture {
    fn bind_group_layout() -> &'static wgpu::BindGroupLayout {
        RgbaBindGroupLayout.get()
    }

    fn bind_group() -> &'static wgpu::BindGroup {
        &_WHITE_PX_TEXTURE
            .get()
            .expect("_WHITE_PX_TEXTURE_BIND_GROUP not set")
            .bind_group
    }
}
