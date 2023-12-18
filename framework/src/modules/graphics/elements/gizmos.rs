use std::cell::RefCell;

use glam::{vec3, Vec3};
use wgpu::{BufferUsages, PrimitiveState, RenderPass, ShaderModuleDescriptor};

use crate::{
    constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT, SURFACE_COLOR_FORMAT},
    modules::graphics::{
        graphics_context::GraphicsContext, shader::bind_group::StaticBindGroup,
        statics::camera::Camera, VertexT,
    },
};

use super::{buffer::GrowableBuffer, color::Color};

pub struct GizmosRenderer {
    context: GraphicsContext,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: GrowableBuffer<Vertex>,
}

impl GizmosRenderer {
    pub fn new(context: &GraphicsContext) -> Self {
        let device = &context.device;

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Gizmos Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gizmos.wgsl").into()),
        });

        // No vertices, just instances
        let vertex_and_transform_layout: [wgpu::VertexBufferLayout; 1] = [Vertex::desc()];

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Gizmos Pipelinelayout"),
                bind_group_layouts: &[Camera::bind_group_layout()],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gizmos Pipeline"),
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
                    format: HDR_COLOR_FORMAT,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::OVER,
                        color: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLE_COUNT,
                alpha_to_coverage_enabled: true,
                ..Default::default()
            },
            multiview: None,
        });

        let vertex_buffer = GrowableBuffer::new(device, 256, BufferUsages::VERTEX);

        Self {
            context: context.clone(),
            pipeline,
            vertex_buffer,
        }
    }

    pub fn draw_line(&mut self, from: Vec3, to: Vec3, color: Color) {
        let data = self.vertex_buffer.data();
        data.push(Vertex {
            pos: [from.x, from.y, from.z],
            color,
        });
        data.push(Vertex {
            pos: [to.x, to.y, to.z],
            color,
        });
    }

    pub fn draw_xyz(&mut self) {
        let data = self.vertex_buffer.data();
        data.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::RED,
        });
        data.push(Vertex {
            pos: [1.0, 0.0, 0.0],
            color: Color::RED,
        });

        data.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::GREEN,
        });
        data.push(Vertex {
            pos: [0.0, 1.0, 0.0],
            color: Color::GREEN,
        });

        data.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::BLUE,
        });
        data.push(Vertex {
            pos: [0.0, 0.0, 1.0],
            color: Color::BLUE,
        });
    }

    pub fn draw_cube(&mut self, position: Vec3, side_len: f32, color: Color) {
        let l = side_len / 2.0;

        let v1 = position + vec3(-l, -l, -l);
        let v2 = position + vec3(l, -l, -l);
        let v3 = position + vec3(l, -l, l);
        let v4 = position + vec3(-l, -l, l);
        let v5 = position + vec3(-l, l, -l);
        let v6 = position + vec3(l, l, -l);
        let v7 = position + vec3(l, l, l);
        let v8 = position + vec3(-l, l, l);
        let lines = [
            (v1, v2),
            (v2, v3),
            (v3, v4),
            (v4, v1),
            (v5, v6),
            (v6, v7),
            (v7, v8),
            (v8, v5),
            (v1, v5),
            (v2, v6),
            (v3, v7),
            (v4, v8),
        ];
        let data = self.vertex_buffer.data();
        for (from, to) in lines {
            data.push(Vertex {
                pos: [from.x, from.y, from.z],
                color,
            });
            data.push(Vertex {
                pos: [to.x, to.y, to.z],
                color,
            });
        }
    }

    pub fn prepare(&mut self) {
        // Note: todo!() this is an ugly position. fix later,
        self.vertex_buffer
            .prepare(&self.context.queue, &self.context.device);
        self.vertex_buffer.data().clear();
    }

    pub fn render<'s: 'e, 'p, 'e>(&'s self, render_pass: &'p mut RenderPass<'e>) {
        if self.vertex_buffer.buffer_len() == 0 {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, Camera::bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.buffer().slice(..));
        render_pass.draw(0..(self.vertex_buffer.buffer_len() as u32), 0..1);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: Color,
}

impl VertexT for Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // 3d pos
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
