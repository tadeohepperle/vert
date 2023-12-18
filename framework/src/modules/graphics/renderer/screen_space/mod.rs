use rand::{thread_rng, Rng};
use wgpu::{RenderPass, ShaderModule, ShaderModuleDescriptor};

use crate::{
    constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT, SURFACE_COLOR_FORMAT},
    modules::graphics::{
        elements::{
            screen_space::ScreenSpaceBindGroup,
            texture::{BindableTexture, Texture},
        },
        graphics_context::GraphicsContext,
        settings::GraphicsSettings,
    },
};

use self::{bloom::BloomPipeline, tonemapping::ToneMappingPipeline};

pub mod bloom;
pub mod tonemapping;

pub struct ScreenSpaceRenderer {
    msaa_depth_texture: DepthTexture,
    /// 4x msaa samples for this texture
    hdr_msaa_texture: HdrTexture,
    /// only 1 sample, the hdr_msaa_texture resolves into the hdr_resolve_texture.
    hdr_resolve_texture: HdrTexture,
    tone_mapping_pipeline: ToneMappingPipeline,
    bloom_pipeline: BloomPipeline,
    screen_vertex_shader: ShaderModule,
}

impl ScreenSpaceRenderer {
    pub fn create(
        context: &GraphicsContext,
        screen_space_bind_group: ScreenSpaceBindGroup,
    ) -> Self {
        // setup textures
        let msaa_depth_texture = DepthTexture::create(&context);
        let msaa_hdr_texture = HdrTexture::create_screen_sized(context, MSAA_SAMPLE_COUNT);
        let hdr_resolve_target_texture = HdrTexture::create_screen_sized(context, 1);

        // setup shader where a single triangle covers the entire screen
        let screen_vertex_shader = context.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Screen Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("screen.vert.wgsl").into()),
        });
        let screen_vertex_state = wgpu::VertexState {
            module: &screen_vertex_shader,
            entry_point: "vs_main",
            buffers: &[],
        };

        // setup pipelines for postprocessing and tonemapping
        let tone_mapping_pipeline = ToneMappingPipeline::new(&context, screen_vertex_state.clone());
        let bloom_pipeline =
            BloomPipeline::new(&context, screen_vertex_state, screen_space_bind_group);
        ScreenSpaceRenderer {
            msaa_depth_texture,
            hdr_msaa_texture: msaa_hdr_texture,
            hdr_resolve_texture: hdr_resolve_target_texture,
            screen_vertex_shader,
            tone_mapping_pipeline,
            bloom_pipeline,
        }
    }

    pub fn new_hdr_4xmsaa_render_pass<'a: 'e, 'e>(
        &'a self,
        encoder: &'e mut wgpu::CommandEncoder,
        graphics_settings: &GraphicsSettings,
    ) -> RenderPass<'e> {
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: self.hdr_msaa_texture.view(),
            resolve_target: Some(self.hdr_resolve_texture.view()),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(graphics_settings.clear_color.into()),
                store: wgpu::StoreOp::Store,
            },
        };
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Renderpass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.msaa_depth_texture.view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        })
    }

    pub fn resize(&mut self, context: &GraphicsContext) {
        self.msaa_depth_texture.recreate(context);
        self.hdr_msaa_texture = HdrTexture::create_screen_sized(context, MSAA_SAMPLE_COUNT);
        self.hdr_resolve_texture = HdrTexture::create_screen_sized(context, 1);
        // recreate bloom textures too
        self.bloom_pipeline.resize(context);
    }

    /// applies post processing to the HDR image and maps from the HDR image to an SRGB image (the surface_view = screen that is presented to user)
    pub fn render_to_surface_view<'a: 'e, 'e>(
        &'a self,
        encoder: &'e mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        graphics_settings: &GraphicsSettings,
    ) {
        if graphics_settings.bloom.activated {
            self.bloom_pipeline.apply_bloom(
                encoder,
                self.hdr_resolve_texture.bind_group(),
                self.hdr_resolve_texture.view(),
                graphics_settings,
            );
        }

        self.tone_mapping_pipeline.apply_tone_mapping(
            encoder,
            self.hdr_resolve_texture.bind_group(),
            surface_view,
        );
    }
}

pub struct DepthTexture(Texture);

impl DepthTexture {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.0.view
    }

    pub fn create(context: &GraphicsContext) -> Self {
        let config = context.surface_config.get();
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
    sample_count: u32,
}

impl HdrTexture {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.texture.texture.view
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.texture.bind_group
    }

    pub fn create_screen_sized(context: &GraphicsContext, sample_count: u32) -> Self {
        let config = context.surface_config.get();
        Self::create(
            context,
            config.width,
            config.height,
            sample_count,
            format!("Screen sized HDR with sample_count: {sample_count}"),
        )
    }

    pub fn create(
        context: &GraphicsContext,
        width: u32,
        height: u32,
        sample_count: u32,
        label: impl Into<String>,
    ) -> Self {
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

        let texture = context.device.create_texture(descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let label: String = label.into();
        let layout = if sample_count == 1 {
            context.rgba_bind_group_layout
        } else {
            context.rgba_bind_group_layout_multisampled
        };
        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
            sample_count,
        }
    }
}
