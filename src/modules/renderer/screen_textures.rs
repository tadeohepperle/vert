use super::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT};
use crate::{
    elements::{
        texture::{rgba_bind_group_layout, rgba_bind_group_layout_msaa4},
        BindableTexture, Color, Texture,
    },
    modules::GraphicsContext,
};

pub struct ScreenTextures {
    pub depth_texture: DepthTexture,
    pub hdr_msaa_texture: HdrTexture,
    pub hdr_resolve_target: HdrTexture,
    pub screen_vertex_shader: ScreenVertexShader,
}

impl ScreenTextures {
    pub fn new(ctx: &GraphicsContext) -> Self {
        let depth_texture = DepthTexture::create(&ctx);
        let hdr_msaa_texture = HdrTexture::create_screen_sized(&ctx, 4);
        let hdr_resolve_target = HdrTexture::create_screen_sized(&ctx, 1);
        let screen_vertex_shader = ScreenVertexShader::new(&ctx.device);

        Self {
            depth_texture,
            hdr_msaa_texture,
            hdr_resolve_target,
            screen_vertex_shader,
        }
    }

    pub fn new_hdr_target_render_pass<'e>(
        &'e self,
        encoder: &'e mut wgpu::CommandEncoder,
        color: Color,
    ) -> wgpu::RenderPass<'e> {
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: self.hdr_msaa_texture.view(),
            resolve_target: Some(self.hdr_resolve_target.view()),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color.into()),
                store: wgpu::StoreOp::Store,
            },
        };
        let main_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Hdr Renderpass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.depth_texture.view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        main_render_pass
    }
}

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

/// Shader for a single triangle that covers the entire screen.
#[derive(Debug)]
pub struct ScreenVertexShader(wgpu::ShaderModule);

impl ScreenVertexShader {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Screen Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("screen.vert.wgsl").into()),
        });
        ScreenVertexShader(module)
    }

    pub fn vertex_state(&self) -> wgpu::VertexState<'_> {
        wgpu::VertexState {
            module: &self.0,
            entry_point: "vs_main",
            buffers: &[],
        }
    }
}
