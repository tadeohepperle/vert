use std::hint;

use wgpu::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, Color, CommandEncoder,
    RenderPassDescriptor, RenderPipeline, TextureView,
};

use crate::{
    constants::HDR_COLOR_FORMAT,
    modules::graphics::{
        elements::texture::{BindableTexture, Texture},
        graphics_context::GraphicsContext,
        settings::GraphicsSettings,
        shader::bind_group::StaticBindGroup,
        statics::{screen_size::ScreenSize, static_texture::RgbaBindGroupLayout},
    },
};

use super::HdrTexture;

/// The input to the BloomPipeline is an HDR texture A that has a bindgroup.
/// We need to be able to use this texture A as a render attachment.
/// The steps this bloom pipeline takes, each bullet point is one render pass:
///
/// B1 has 1/2 the resolution of the original image, B2 has 1/4 the resolution and so on...
///
/// # 1. Downsampling:
///
/// - threshold and downsample the image, store result in B1
/// - downsample B1 store the result in B2
/// - downsample B2 store the result in B3
/// - downsample B3 store the result in B4
///
/// note: we need to be able to use B1..BX as bindgroups of textures, to sample them in fragment shaders.
/// # 2. Upsampling:
///
/// - upsample B4 and add it to B3
/// - upsample B3 and add it to B2
/// - upsample B2 and add it to B1
/// - upsample B1 and add it to the original HDR image A.
///
/// This should result in a bloom.
pub struct BloomPipeline {
    downsample_threshold_pipeline: wgpu::RenderPipeline,
    downsample_pipeline: wgpu::RenderPipeline,
    upsample_pipeline: wgpu::RenderPipeline,
    final_upsample_pipeline: wgpu::RenderPipeline,
    bloom_textures: BloomTextures,
}

impl BloomPipeline {
    pub fn new(context: &GraphicsContext, screen_vertex_state: wgpu::VertexState) -> Self {
        // let fragment_shader =

        //         todo!()
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        ScreenSize::bind_group_layout(),
                        RgbaBindGroupLayout.get(),
                    ],
                    push_constant_ranges: &[],
                });

        let fragment_shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Bloom Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("bloom.frag.wgsl").into()),
            });

        let create_pipeline =
            |label: &str, entry_point: &str, blend: Option<wgpu::BlendState>| -> RenderPipeline {
                context
                    .device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some(label),
                        layout: Some(&pipeline_layout),
                        vertex: screen_vertex_state.clone(),
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

        let config = context.surface_config.get();
        let width = config.width;
        let height = config.height;
        let bloom_textures = BloomTextures::create(context, width, height);

        Self {
            downsample_pipeline,
            upsample_pipeline,
            downsample_threshold_pipeline,
            bloom_textures,
            final_upsample_pipeline,
        }
    }

    pub fn resize(&mut self, context: &GraphicsContext) {
        // recreate textures
        let config = context.surface_config.get();
        let width = config.width;
        let height = config.height;
        self.bloom_textures = BloomTextures::create(context, width, height);
    }

    /// the `texture_view` should be the texture view of the screen. We draw to it.
    /// the `texture_view_bind_group` is the same texture, given as an input. We use it to
    /// draw some bloom to intermediate textures and write that bloom back to it at the end.
    ///
    /// Note: Of course having so many render passes is kinda inefficient, but the result looks pretty nice right now.
    /// Later we can see how hazel does bloom and have a similar thing.
    pub fn apply_bloom<'e>(
        &'e self,
        encoder: &'e mut CommandEncoder,
        texture_bind_group: &wgpu::BindGroup,
        texture_view: &TextureView,
        graphics_settings: &GraphicsSettings,
    ) {
        fn run_screen_render_pass<'e>(
            label: &str,
            encoder: &'e mut CommandEncoder,
            input_texture: &'e wgpu::BindGroup,
            output_texture: &'e TextureView,
            pipeline: &'e wgpu::RenderPipeline,
        ) {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, ScreenSize::bind_group(), &[]);
            pass.set_bind_group(1, input_texture, &[]);
            pass.draw(0..3, 0..1);
        }

        // /////////////////////////////////////////////////////////////////////////////
        // downsample
        // /////////////////////////////////////////////////////////////////////////////

        run_screen_render_pass(
            "1 -> 1/2 downsample and threshold",
            encoder,
            texture_bind_group,
            self.bloom_textures.b2.view(),
            &self.downsample_threshold_pipeline,
        );
        run_screen_render_pass(
            "1/2 -> 1/4 downsample",
            encoder,
            self.bloom_textures.b2.bind_group(),
            self.bloom_textures.b4.view(),
            &self.downsample_pipeline,
        );
        run_screen_render_pass(
            "1/4 -> 1/8 downsample",
            encoder,
            self.bloom_textures.b4.bind_group(),
            self.bloom_textures.b8.view(),
            &self.downsample_pipeline,
        );
        run_screen_render_pass(
            "1/8 -> 1/16 downsample",
            encoder,
            self.bloom_textures.b8.bind_group(),
            self.bloom_textures.b16.view(),
            &self.downsample_pipeline,
        );

        run_screen_render_pass(
            "1/16 -> 1/32 downsample",
            encoder,
            self.bloom_textures.b16.bind_group(),
            self.bloom_textures.b32.view(),
            &self.downsample_pipeline,
        );

        run_screen_render_pass(
            "1/32 -> 1/64 downsample",
            encoder,
            self.bloom_textures.b32.bind_group(),
            self.bloom_textures.b64.view(),
            &self.downsample_pipeline,
        );

        run_screen_render_pass(
            "1/64 -> 1/128 downsample",
            encoder,
            self.bloom_textures.b64.bind_group(),
            self.bloom_textures.b128.view(),
            &self.downsample_pipeline,
        );

        run_screen_render_pass(
            "1/128 -> 1/256 downsample",
            encoder,
            self.bloom_textures.b128.bind_group(),
            self.bloom_textures.b256.view(),
            &self.downsample_pipeline,
        );

        run_screen_render_pass(
            "1/256 -> 1/512 downsample",
            encoder,
            self.bloom_textures.b256.bind_group(),
            self.bloom_textures.b512.view(),
            &self.downsample_pipeline,
        );

        // /////////////////////////////////////////////////////////////////////////////
        // upsample
        // /////////////////////////////////////////////////////////////////////////////

        run_screen_render_pass(
            "1/512 -> 1/256 upsample and add",
            encoder,
            self.bloom_textures.b512.bind_group(),
            self.bloom_textures.b256.view(),
            &self.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/256 -> 1/128 upsample and add",
            encoder,
            self.bloom_textures.b256.bind_group(),
            self.bloom_textures.b128.view(),
            &self.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/128 -> 1/64 upsample and add",
            encoder,
            self.bloom_textures.b128.bind_group(),
            self.bloom_textures.b64.view(),
            &self.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/64 -> 1/32 upsample and add",
            encoder,
            self.bloom_textures.b64.bind_group(),
            self.bloom_textures.b32.view(),
            &self.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/32 -> 1/16 upsample and add",
            encoder,
            self.bloom_textures.b32.bind_group(),
            self.bloom_textures.b16.view(),
            &self.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/16 -> 1/8 upsample and add",
            encoder,
            self.bloom_textures.b16.bind_group(),
            self.bloom_textures.b8.view(),
            &self.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/8 -> 1/4 upsample and add",
            encoder,
            self.bloom_textures.b8.bind_group(),
            self.bloom_textures.b4.view(),
            &self.upsample_pipeline,
        );

        run_screen_render_pass(
            "1/4 -> 1/2 upsample and add",
            encoder,
            self.bloom_textures.b4.bind_group(),
            self.bloom_textures.b2.view(),
            &self.upsample_pipeline,
        );

        // /////////////////////////////////////////////////////////////////////////////
        // Final pass, now with blend factor to add to original image
        // /////////////////////////////////////////////////////////////////////////////

        let blend_factor = graphics_settings.bloom.blend_factor as f64;
        let blend_factor = Color {
            r: blend_factor,
            g: blend_factor,
            b: blend_factor,
            a: blend_factor,
        };

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("1/2 -> 1 upsample and add"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.final_upsample_pipeline);
        pass.set_blend_constant(blend_factor);
        pass.set_bind_group(0, ScreenSize::bind_group(), &[]);
        pass.set_bind_group(1, self.bloom_textures.b2.bind_group(), &[]);
        pass.draw(0..3, 0..1);
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
    pub fn create(context: &GraphicsContext, width: u32, height: u32) -> Self {
        let b2 = HdrTexture::create(context, width / 2, height / 2, 1, "b2");
        let b4 = HdrTexture::create(context, width / 4, height / 4, 1, "b4");
        let b8 = HdrTexture::create(context, width / 8, height / 8, 1, "b8");
        let b16 = HdrTexture::create(context, width / 16, height / 16, 1, "b16");
        let b32 = HdrTexture::create(context, width / 32, height / 32, 1, "b32");
        let b64 = HdrTexture::create(context, width / 64, height / 64, 1, "b64");
        let b128 = HdrTexture::create(context, width / 128, height / 128, 1, "b128");
        let b256 = HdrTexture::create(context, width / 256, height / 256, 1, "b256");
        let b512 = HdrTexture::create(context, width / 512, height / 512, 1, "b512");

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
