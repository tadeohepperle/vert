use std::hint;

use wgpu::{
    BlendComponent, BlendState, Color, CommandEncoder, RenderPassDescriptor, RenderPipeline,
    TextureView,
};

use crate::{
    constants::HDR_COLOR_FORMAT,
    modules::graphics::{
        elements::{
            screen_space::ScreenSpaceBindGroup,
            texture::{BindableTexture, Texture},
        },
        graphics_context::GraphicsContext,
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
    downsample_pipeline: wgpu::RenderPipeline,
    upsample_pipeline: wgpu::RenderPipeline,
    downsample_threshold_pipeline: wgpu::RenderPipeline,
    bloom_textures: BloomTextures,
    screen_space_bind_group: ScreenSpaceBindGroup,
}

impl BloomPipeline {
    pub fn new(
        context: &GraphicsContext,
        screen_vertex_state: wgpu::VertexState,
        screen_space_bind_group: ScreenSpaceBindGroup,
    ) -> Self {
        // let fragment_shader =

        //         todo!()
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        screen_space_bind_group.layout(),
                        context.rgba_bind_group_layout,
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

        let down_blend_state: Option<BlendState> = None;

        let up_blend_state = Some(BlendState {
            color: BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        });

        let downsample_threshold_pipeline = create_pipeline(
            "Downsample Threshold",
            "threshold_downsample",
            down_blend_state,
        );
        let downsample_pipeline = create_pipeline("Downsample", "downsample", down_blend_state);
        let upsample_pipeline = create_pipeline("Bloom shader", "upsample", up_blend_state);

        let config = context.surface_config.get();
        let width = config.width;
        let height = config.height;
        let bloom_textures = BloomTextures::create(context, width, height);

        Self {
            downsample_pipeline,
            upsample_pipeline,
            downsample_threshold_pipeline,
            bloom_textures,
            screen_space_bind_group,
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
    pub fn apply_bloom<'e>(
        &'e self,
        encoder: &'e mut CommandEncoder,
        texture_bind_group: &wgpu::BindGroup,
        texture_view: &TextureView,
    ) {
        fn run_screen_render_pass<'e>(
            label: &str,
            encoder: &'e mut CommandEncoder,
            input_texture: &'e wgpu::BindGroup,
            output_texture: &'e TextureView,
            pipeline: &'e wgpu::RenderPipeline,
            screen_space_bindgroup: &ScreenSpaceBindGroup,
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
            pass.set_bind_group(0, screen_space_bindgroup.bind_group(), &[]);
            pass.set_bind_group(1, input_texture, &[]);
            pass.set_blend_constant(Color {
                r: 0.45,
                g: 0.45,
                b: 0.45,
                a: 0.45,
            });
            pass.draw(0..3, 0..1);
        }

        run_screen_render_pass(
            "1 -> 1/2 downsample and threshold",
            encoder,
            texture_bind_group,
            self.bloom_textures.b2.view(),
            &self.downsample_threshold_pipeline,
            &self.screen_space_bind_group,
        );
        run_screen_render_pass(
            "1/2 -> 1/4 downsample",
            encoder,
            self.bloom_textures.b2.bind_group(),
            self.bloom_textures.b4.view(),
            &self.downsample_pipeline,
            &self.screen_space_bind_group,
        );
        run_screen_render_pass(
            "1/4 -> 1/8 downsample",
            encoder,
            self.bloom_textures.b4.bind_group(),
            self.bloom_textures.b8.view(),
            &self.downsample_pipeline,
            &self.screen_space_bind_group,
        );
        run_screen_render_pass(
            "1/8 -> 1/16 downsample",
            encoder,
            self.bloom_textures.b8.bind_group(),
            self.bloom_textures.b16.view(),
            &self.downsample_pipeline,
            &self.screen_space_bind_group,
        );

        run_screen_render_pass(
            "1/16 -> 1/8 upsample and add",
            encoder,
            self.bloom_textures.b16.bind_group(),
            self.bloom_textures.b8.view(),
            &self.upsample_pipeline,
            &self.screen_space_bind_group,
        );

        run_screen_render_pass(
            "1/8 -> 1/4 upsample and add",
            encoder,
            self.bloom_textures.b8.bind_group(),
            self.bloom_textures.b4.view(),
            &self.upsample_pipeline,
            &self.screen_space_bind_group,
        );

        run_screen_render_pass(
            "1/4 -> 1/2 upsample and add",
            encoder,
            self.bloom_textures.b4.bind_group(),
            self.bloom_textures.b2.view(),
            &self.upsample_pipeline,
            &self.screen_space_bind_group,
        );

        run_screen_render_pass(
            "1/2 -> 1 upsample and add",
            encoder,
            self.bloom_textures.b2.bind_group(),
            texture_view,
            &self.upsample_pipeline,
            &self.screen_space_bind_group,
        );
    }
}

pub struct BloomTextures {
    b2: HdrTexture,
    b4: HdrTexture,
    b8: HdrTexture,
    b16: HdrTexture,
}

impl BloomTextures {
    pub fn create(context: &GraphicsContext, width: u32, height: u32) -> Self {
        let b2 = HdrTexture::create(context, width / 2, height / 2, 1, "b2");
        let b4 = HdrTexture::create(context, width / 4, height / 4, 1, "b4");
        let b8 = HdrTexture::create(context, width / 16, height / 16, 1, "b8");
        let b16 = HdrTexture::create(context, width / 32, height / 32, 1, "b16");
        Self { b2, b4, b8, b16 }
    }
}
