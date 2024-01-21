use wgpu::{PushConstantRange, ShaderStages};

use crate::{
    elements::texture::rgba_bind_group_layout,
    modules::{renderer::SURFACE_COLOR_FORMAT, GraphicsContext},
};

use super::ScreenVertexShader;

pub struct AcesToneMapping {
    enabled: bool,
    pipeline: wgpu::RenderPipeline,
}

impl AcesToneMapping {
    pub fn new(ctx: &GraphicsContext, screen_vertex_shader: &ScreenVertexShader) -> Self {
        let pipeline = create_pipeline(
            include_str!("tonemapping.wgsl"),
            &ctx.device,
            &screen_vertex_shader,
        );
        Self {
            enabled: true,
            pipeline,
        }
    }

    pub fn enabled_mut(&mut self) -> &mut bool {
        &mut self.enabled
    }

    /// Note: input texture should be hdr, output sdr
    pub fn apply<'e>(
        &'e mut self,
        encoder: &'e mut wgpu::CommandEncoder,
        input_texture: &wgpu::BindGroup,
        output_texture: &wgpu::TextureView,
    ) {
        let mut tone_mapping_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("AcesToneMapping"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_texture,
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
        tone_mapping_pass.set_bind_group(0, input_texture, &[]);
        tone_mapping_pass.draw(0..3, 0..1);
    }
}

fn create_pipeline(
    shader_wgsl: &str,
    device: &wgpu::Device,
    screen_vertex_shader: &ScreenVertexShader,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Tonemapping Shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_wgsl)),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[rgba_bind_group_layout(device)],
        push_constant_ranges: &[PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            range: 0..16,
        }],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("{:?}", shader)),
        layout: Some(&pipeline_layout),
        vertex: screen_vertex_shader.vertex_state(),
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

    pipeline
}
