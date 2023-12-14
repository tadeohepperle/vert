use std::sync::Arc;

use glam::{UVec2, Vec2};
use vert_core::{arenas::Arenas, prelude::*};
use wgpu::{PrimitiveState, RenderPass, ShaderModuleDescriptor};

use crate::{
    constants::{COLOR_FORMAT, DEPTH_FORMAT},
    modules::graphics::{graphics_context::GraphicsContext, VertexT},
};

use super::{
    buffer::{IndexBuffer, InstanceBuffer, VertexBuffer},
    color::Color,
    screen_space::ScreenSpaceBindGroup,
    texture::BindableTexture,
};

reflect!(UiRect:);
impl Component for UiRect {}
#[derive(Debug, Clone)]
pub struct UiRect {
    pub instance: UiRectInstance,
    pub texture: Option<Arc<BindableTexture>>,
}

pub struct UiRectRenderPipeline {
    pipeline: wgpu::RenderPipeline,
    screen_space_bind_group: ScreenSpaceBindGroup,
    index_buffer: IndexBuffer,
    instances: VertexBuffer<UiRectInstance>,
}

impl UiRectRenderPipeline {
    pub fn new(context: &GraphicsContext, screen_space_bind_group: ScreenSpaceBindGroup) -> Self {
        let device = &context.device;
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Ui Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui_rect.wgsl").into()),
        });

        // No vertices, just instances
        let vertex_and_transform_layout: [wgpu::VertexBufferLayout; 1] = [UiRectInstance::desc()];

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ColoredMesh Pipelinelayout"),
                bind_group_layouts: &[screen_space_bind_group.layout()],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ColoredMesh Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &vertex_and_transform_layout,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::REPLACE,
                        color: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // just a simple rect:
        let index_buffer = IndexBuffer::new(vec![0, 1, 2, 0, 2, 3], &context.device);

        let instances: VertexBuffer<UiRectInstance> = VertexBuffer::new(
            vec![
                UiRectInstance {
                    posbb: [200.0, 200.0, 600.0, 600.0],
                    uvbb: [0.0, 0.0, 1.0, 1.0],
                    color: Color::RED,
                },
                UiRectInstance {
                    posbb: [10.0, 40.0, 100.0, 100.0],
                    uvbb: [0.0, 0.0, 1.0, 1.0],
                    color: Color::BLUE,
                },
            ],
            &context.device,
        );

        UiRectRenderPipeline {
            pipeline,
            screen_space_bind_group,
            index_buffer,
            instances,
        }
    }

    pub fn render_ui_rects<'s: 'e, 'p, 'e>(
        &'s self,
        render_pass: &'p mut RenderPass<'e>,
        arenas: &'e Arenas,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.screen_space_bind_group.bind_group(), &[]);

        render_pass.set_vertex_buffer(0, self.instances.buffer().slice(..));
        // render_pass.set_vertex_buffer(1, obj.transform.buffer().slice(..));
        render_pass.set_index_buffer(
            self.index_buffer.buffer().slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..self.index_buffer.len(), 0, 0..self.instances.len());
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiRectInstance {
    // min x, min y, max x, max y
    pub posbb: [f32; 4],
    // min x, min y, max x, max y
    pub uvbb: [f32; 4],
    pub color: Color,
}
impl VertexT for UiRectInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<UiRectInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // pos
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // color
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
