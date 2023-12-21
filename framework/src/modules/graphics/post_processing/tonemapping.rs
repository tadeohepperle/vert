use std::sync::Arc;

use wgpu::{RenderPass, ShaderModule};

use crate::{
    constants::SURFACE_COLOR_FORMAT,
    modules::graphics::{
        graphics_context::GraphicsContext, statics::static_texture::RgbaBindGroupLayout,
        ScreenVertexShader,
    },
    utils::watcher::ShaderFileWatcher,
    wgsl_file,
};

use super::PostProcessingEffectT;

pub struct AcesToneMapping {
    pipeline: wgpu::RenderPipeline,
    watcher: ShaderFileWatcher,
    screen_vertex_shader: Arc<ScreenVertexShader>,
}

impl PostProcessingEffectT for AcesToneMapping {
    fn new(context: &GraphicsContext, screen_vertex_shader: &Arc<ScreenVertexShader>) -> Self
    where
        Self: Sized,
    {
        let pipeline = create_pipeline(
            include_str!("tonemapping.wgsl"),
            context,
            screen_vertex_shader,
        );
        let watcher = ShaderFileWatcher::new(&wgsl_file!());
        Self {
            pipeline,
            watcher,
            screen_vertex_shader: screen_vertex_shader.clone(),
        }
    }

    /// input is expected to be hdr texture, and output is surface of window.
    fn apply<'e>(
        &'e mut self,
        encoder: &'e mut wgpu::CommandEncoder,
        input: &wgpu::BindGroup,
        output: &wgpu::TextureView,
        graphics_settings: &crate::modules::graphics::settings::GraphicsSettings,
    ) {
        let mut tone_mapping_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Hdr::process"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
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
        tone_mapping_pass.set_bind_group(0, input, &[]);
        tone_mapping_pass.draw(0..3, 0..1);
    }
}

fn create_pipeline(
    shader_wgsl: &str,
    context: &GraphicsContext,
    screen_vertex_shader: &ScreenVertexShader,
) -> wgpu::RenderPipeline {
    let shader = context
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Tonemapping Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_wgsl)),
        });
    let pipeline_layout = context
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[RgbaBindGroupLayout.static_layout()],
            push_constant_ranges: &[],
        });

    let pipeline = context
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
