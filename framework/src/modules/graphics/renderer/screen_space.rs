use wgpu::{RenderPass, SurfaceConfiguration};

use crate::{
    constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT, SURFACE_COLOR_FORMAT},
    modules::graphics::graphics_context::{rgba_bind_group_layout, GraphicsContext},
};

pub struct ScreenSpaceRenderer {
    msaa_depth_texture: DepthTexture,
    msaa_hdr_texture: MSAATexture,
    hdr_to_u8_pipeline: HdrToU8Pipeline,
}

impl ScreenSpaceRenderer {
    pub fn create(context: &GraphicsContext) -> Self {
        let msaa_depth_texture = DepthTexture::create(&context);
        let msaa_hdr_texture = MSAATexture::create(&context);
        let hdr_to_u8_pipeline = HdrToU8Pipeline::new(&context);
        ScreenSpaceRenderer {
            msaa_depth_texture,
            msaa_hdr_texture,
            hdr_to_u8_pipeline,
        }
    }

    pub fn new_hdr_4xmsaa_render_pass<'a: 'e, 'e>(
        &'a self,
        encoder: &'e mut wgpu::CommandEncoder,
    ) -> RenderPass<'e> {
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: &self.msaa_hdr_texture.msaa_texture_view,
            resolve_target: Some(&self.msaa_hdr_texture.resolve_target_view),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        };
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Renderpass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.msaa_depth_texture.view,
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
        self.msaa_hdr_texture.recreate(context);
    }

    /// applies post processing to the HDR image and maps from the HDR image to an SRGB image (the surface_view = screen that is presented to user)
    pub fn render_to_surface_view<'a: 'e, 'e>(
        &'a self,
        encoder: &'e mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
    ) {
        let mut hdr_to_u8_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Hdr::process"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.hdr_to_u8_pipeline.process(
            &mut hdr_to_u8_pass,
            &self.msaa_hdr_texture.resolve_target_bind_group,
        );
    }
}

pub struct DepthTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    size: wgpu::Extent3d,
}

impl DepthTexture {
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

        Self {
            texture,
            view,
            sampler,
            size,
        }
    }

    pub fn recreate(&mut self, context: &GraphicsContext) {
        *self = Self::create(context);
    }
}

pub struct MSAATexture {
    msaa_texture: wgpu::Texture,
    msaa_texture_view: wgpu::TextureView,
    resolve_target: wgpu::Texture,
    resolve_target_view: wgpu::TextureView,
    resolve_target_sampler: wgpu::Sampler,
    resolve_target_bind_group: wgpu::BindGroup,
}

impl MSAATexture {
    pub fn create(context: &GraphicsContext) -> Self {
        let config = context.surface_config.get();
        let extent = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let msaa_texture_descriptor = &wgpu::TextureDescriptor {
            size: extent,
            mip_level_count: 1,
            sample_count: MSAA_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };
        let msaa_texture = context.device.create_texture(msaa_texture_descriptor);
        let msaa_texture_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let resolve_target_descriptor = &wgpu::TextureDescriptor {
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_COLOR_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };

        let resolve_target = context.device.create_texture(resolve_target_descriptor);
        let resolve_target_view =
            resolve_target.create_view(&wgpu::TextureViewDescriptor::default());

        let resolve_target_sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let resolve_target_bind_group =
            context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Hdr::bind_group"),
                    layout: rgba_bind_group_layout(&context.device),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&resolve_target_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&resolve_target_sampler),
                        },
                    ],
                });

        MSAATexture {
            msaa_texture,
            msaa_texture_view,
            resolve_target,
            resolve_target_view,
            resolve_target_sampler,
            resolve_target_bind_group,
        }
    }

    pub fn recreate(&mut self, context: &GraphicsContext) {
        *self = Self::create(context);
    }
}

struct HdrToU8Pipeline {
    pipeline: wgpu::RenderPipeline,
}

impl HdrToU8Pipeline {
    pub fn new(context: &GraphicsContext) -> Self {
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rect 3d Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("hdr.wgsl").into()),
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[rgba_bind_group_layout(&context.device)],
                    push_constant_ranges: &[],
                });

        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&format!("{:?}", shader)),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: SURFACE_COLOR_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        Self { pipeline }
    }

    pub fn process<'e, 'p>(
        &'e self,
        hdr_to_u8_pass: &'p mut RenderPass<'e>,
        hdr_resolve_target_bind_group: &'e wgpu::BindGroup,
    ) {
        hdr_to_u8_pass.set_pipeline(&self.pipeline);
        hdr_to_u8_pass.set_bind_group(0, hdr_resolve_target_bind_group, &[]);
        hdr_to_u8_pass.draw(0..3, 0..1);
    }
}
