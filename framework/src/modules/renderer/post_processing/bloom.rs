use std::borrow::Cow;

use wgpu::{BlendComponent, BlendFactor, BlendOperation, BlendState};

use crate::{
    elements::texture::rgba_bind_group_layout,
    modules::{
        renderer::{screen_texture::HdrTexture, HDR_COLOR_FORMAT},
        GraphicsContext, MainScreenSize,
    },
};

use super::ScreenVertexShader;

pub struct Bloom {
    bloom_textures: BloomTextures,
    bloom_pipelines: BloomPipelines,
    context: GraphicsContext,
}

struct BloomPipelines {
    downsample_threshold_pipeline: wgpu::RenderPipeline,
    downsample_pipeline: wgpu::RenderPipeline,
    upsample_pipeline: wgpu::RenderPipeline,
    final_upsample_pipeline: wgpu::RenderPipeline,
}

impl BloomPipelines {
    pub fn new(
        shader_wgsl: &str,
        ctx: &GraphicsContext,
        screen_size: &MainScreenSize,
        screen_vertex_shader: &ScreenVertexShader,
    ) -> Self {
        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    screen_size.bind_group_layout(),
                    rgba_bind_group_layout(&ctx.device),
                ],
                push_constant_ranges: &[],
            });

        let fragment_shader = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Bloom Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_wgsl)).into(),
            });

        let create_pipeline = |label: &str,
                               entry_point: &str,
                               blend: Option<wgpu::BlendState>|
         -> wgpu::RenderPipeline {
            ctx.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(&pipeline_layout),
                    vertex: screen_vertex_shader.vertex_state(),
                    fragment: Some(wgpu::FragmentState {
                        module: &fragment_shader,
                        entry_point,
                        targets: &[Some(wgpu::ColorTargetState {
                            format: HDR_COLOR_FORMAT,
                            blend,
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
                })
        };

        let downsample_threshold_pipeline =
            create_pipeline("Downsample Threshold", "threshold_downsample", None);
        let downsample_pipeline = create_pipeline("Downsample", "downsample", None);

        let up_blend_state = Some(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent::OVER,
        });

        let final_up_blend_state = Some(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::Constant,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent::OVER,
        });

        let upsample_pipeline = create_pipeline("Bloom shader", "upsample", up_blend_state);
        // only differs from upsample pipeline in the use of a constant for blending it back into the orginial image (the render target of this pipeline)
        let final_upsample_pipeline =
            create_pipeline("Bloom shader", "upsample", final_up_blend_state);

        Self {
            downsample_threshold_pipeline,
            downsample_pipeline,
            upsample_pipeline,
            final_upsample_pipeline,
        }
    }
}

pub struct BloomTextures {
    b2: HdrTexture,
    b4: HdrTexture,
    b8: HdrTexture,
    b16: HdrTexture,
    b32: HdrTexture,
    b64: HdrTexture,
    b128: HdrTexture,
    b256: HdrTexture,
    b512: HdrTexture,
}

impl BloomTextures {
    pub fn create(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let b2 = HdrTexture::create(device, width / 2, height / 2, 1, "b2");
        let b4 = HdrTexture::create(device, width / 4, height / 4, 1, "b4");
        let b8 = HdrTexture::create(device, width / 8, height / 8, 1, "b8");
        let b16 = HdrTexture::create(device, width / 16, height / 16, 1, "b16");
        let b32 = HdrTexture::create(device, width / 32, height / 32, 1, "b32");
        let b64 = HdrTexture::create(device, width / 64, height / 64, 1, "b64");
        let b128 = HdrTexture::create(device, width / 128, height / 128, 1, "b128");
        let b256 = HdrTexture::create(device, width / 256, height / 256, 1, "b256");
        let b512 = HdrTexture::create(device, width / 512, height / 512, 1, "b512");

        Self {
            b2,
            b4,
            b8,
            b16,
            b32,
            b64,
            b128,
            b256,
            b512,
        }
    }
}
