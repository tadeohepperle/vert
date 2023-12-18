use wgpu::{RenderPass, ShaderModule};

use crate::{
    constants::SURFACE_COLOR_FORMAT,
    modules::graphics::{
        graphics_context::GraphicsContext, statics::static_texture::RgbaBindGroupLayout,
    },
};

pub struct ToneMappingPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl ToneMappingPipeline {
    pub fn new(context: &GraphicsContext, screen_vertex_state: wgpu::VertexState) -> Self {
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Tonemapping Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("tonemapping.frag.wgsl").into()),
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[RgbaBindGroupLayout.get()],
                    push_constant_ranges: &[],
                });

        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&format!("{:?}", shader)),
                layout: Some(&pipeline_layout),
                vertex: screen_vertex_state,
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

    /// The input_hdr_texture should be 16 bit float per channel, while the output_surface_texture should be 8 bit 2RGB.
    pub fn apply_tone_mapping<'e, 'p>(
        &'e self,
        encoder: &'e mut wgpu::CommandEncoder,
        input_hdr_texture: &'e wgpu::BindGroup,
        output_surface_texture: &'e wgpu::TextureView,
    ) {
        let mut tone_mapping_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Hdr::process"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_surface_texture,
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

        tone_mapping_pass.set_pipeline(&self.pipeline);
        tone_mapping_pass.set_bind_group(0, input_hdr_texture, &[]);
        tone_mapping_pass.draw(0..3, 0..1);
    }
}
