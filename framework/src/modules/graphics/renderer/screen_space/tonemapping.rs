use wgpu::{RenderPass, ShaderModule};

use crate::{
    constants::SURFACE_COLOR_FORMAT, modules::graphics::graphics_context::GraphicsContext,
};

pub struct ToneMappingPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl ToneMappingPipeline {
    pub fn new(context: &GraphicsContext, screen_vertex_state: wgpu::VertexState) -> Self {
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rect 3d Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("tonemapping.frag.wgsl").into()),
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[context.rgba_bind_group_layout],
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
