use super::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT};
use crate::{
    elements::{
        texture::{rgba_bind_group_layout, rgba_bind_group_layout_msaa4},
        BindableTexture, Texture,
    },
    modules::GraphicsContext,
};
use log::warn;
use rand::{thread_rng, Rng};
pub struct DepthTexture(Texture);

impl DepthTexture {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.0.view
    }

    pub fn create(context: &GraphicsContext) -> Self {
        let config = &context.surface_config;
        let format = DEPTH_FORMAT;
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            size,
            mip_level_count: 1,
            sample_count: MSAA_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format],
        };
        let texture = context.device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self(Texture {
            label: Some("Depth Texture".into()),
            id: 333,
            texture,
            view,
            sampler,
            size,
        })
    }

    pub fn recreate(&mut self, context: &GraphicsContext) {
        *self = Self::create(context);
    }
}

#[derive(Debug)]
pub struct HdrTexture {
    texture: BindableTexture,
    /// for MSAA
    _unused_sample_count: u32,
}

impl HdrTexture {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.texture.texture.view
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.texture.bind_group
    }

    pub fn create_screen_sized(ctx: &GraphicsContext, sample_count: u32) -> Self {
        Self::create(
            &ctx.device,
            ctx.surface_config.width,
            ctx.surface_config.height,
            sample_count,
            format!("Screen sized HDR with sample_count: {sample_count}"),
        )
    }

    pub fn create(
        device: &wgpu::Device,
        mut width: u32,
        mut height: u32,
        sample_count: u32,
        label: impl Into<String>,
    ) -> Self {
        let label: String = label.into();

        if width == 0 {
            warn!(
                "Attempted to create Hdr HdrTexture with size {width}x{height} with label {label}",
            );
            width = 1;
        }

        if height == 0 {
            warn!(
                "Attempted to create Hdr HdrTexture with size {width}x{height} with label {label}",
            );
            height = 1;
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let descriptor = &wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
            view_formats: &[],
        };

        let texture = device.create_texture(descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let layout = match sample_count {
            1 => rgba_bind_group_layout(device),
            4 => rgba_bind_group_layout_msaa4(device),
            _ => panic!("Sample count {sample_count} not supported"),
        };

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let texture = Texture {
            label: Some(label.into()),
            id: thread_rng().gen(),
            texture,
            view,
            sampler,
            size,
        };

        HdrTexture {
            texture: BindableTexture {
                texture,
                bind_group,
            },
            _unused_sample_count: sample_count,
        }
    }
}
