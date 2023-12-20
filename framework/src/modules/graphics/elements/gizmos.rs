use std::cell::RefCell;

use glam::{vec3, Vec3};
use wgpu::{BufferUsages, PrimitiveState, RenderPass, ShaderModuleDescriptor};

use crate::{
    constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT, SURFACE_COLOR_FORMAT},
    modules::graphics::{
        graphics_context::GraphicsContext,
        shader::{Attribute, VertexT},
        statics::{camera::Camera, StaticBindGroup},
    },
};

use super::{buffer::GrowableBuffer, color::Color};

pub struct GizmosRenderer {
    context: GraphicsContext,
    pipeline: wgpu::RenderPipeline,
    vertices: Vec<Vertex>,
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
        let _empty = &mut vec![];
        let vertex_and_transform_layout: [wgpu::VertexBufferLayout; 1] =
            [Vertex::vertex_buffer_layout(0, false, _empty)];

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
            vertices: vec![],
        }
    }

    pub fn draw_line(&mut self, from: Vec3, to: Vec3, color: Color) {
        self.vertices.push(Vertex {
            pos: [from.x, from.y, from.z],
            color,
        });
        self.vertices.push(Vertex {
            pos: [to.x, to.y, to.z],
            color,
        });
    }

    pub fn draw_xyz(&mut self) {
        self.vertices.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::RED,
        });
        self.vertices.push(Vertex {
            pos: [1.0, 0.0, 0.0],
            color: Color::RED,
        });

        self.vertices.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::GREEN,
        });
        self.vertices.push(Vertex {
            pos: [0.0, 1.0, 0.0],
            color: Color::GREEN,
        });

        self.vertices.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::BLUE,
        });
        self.vertices.push(Vertex {
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

        for (from, to) in lines {
            self.vertices.push(Vertex {
                pos: [from.x, from.y, from.z],
                color,
            });
            self.vertices.push(Vertex {
                pos: [to.x, to.y, to.z],
                color,
            });
        }
    }

    pub fn prepare(&mut self) {
        // Note: todo!() this is an ugly position. fix later,
        self.vertex_buffer
            .prepare(&self.vertices, &self.context.queue, &self.context.device);
        self.vertices.clear();
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
    const ATTRIBUTES: &'static [crate::modules::graphics::shader::Attribute] = &[
        Attribute::new("pos", wgpu::VertexFormat::Float32x3),
        Attribute::new("color", wgpu::VertexFormat::Float32x4),
    ];
}
