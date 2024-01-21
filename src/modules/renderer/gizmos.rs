use glam::vec3;
use glam::Vec3;
use wgpu::BufferUsages;
use wgpu::FragmentState;
use wgpu::PrimitiveState;
use wgpu::ShaderModuleDescriptor;
use wgpu::VertexState;

use crate::elements::camera3d::Camera3dGR;
use crate::elements::Color;
use crate::elements::GrowableBuffer;
use crate::modules::renderer::Attribute;
use crate::modules::renderer::VertexT;
use crate::modules::renderer::DEPTH_FORMAT;
use crate::modules::renderer::HDR_COLOR_FORMAT;
use crate::modules::renderer::MSAA_SAMPLE_COUNT;
use crate::modules::GraphicsContext;

use crate::Prepare;

// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl Gizmos {
    pub fn draw_line(&mut self, from: Vec3, to: Vec3, color: Color) {
        self.vertex_queue.push(Vertex {
            pos: [from.x, from.y, from.z],
            color,
        });
        self.vertex_queue.push(Vertex {
            pos: [to.x, to.y, to.z],
            color,
        });
    }

    pub fn draw_xyz(&mut self) {
        self.vertex_queue.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::RED,
        });
        self.vertex_queue.push(Vertex {
            pos: [1.0, 0.0, 0.0],
            color: Color::RED,
        });

        self.vertex_queue.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::GREEN,
        });
        self.vertex_queue.push(Vertex {
            pos: [0.0, 1.0, 0.0],
            color: Color::GREEN,
        });

        self.vertex_queue.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::BLUE,
        });
        self.vertex_queue.push(Vertex {
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
            self.vertex_queue.push(Vertex {
                pos: [from.x, from.y, from.z],
                color,
            });
            self.vertex_queue.push(Vertex {
                pos: [to.x, to.y, to.z],
                color,
            });
        }
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Module
// /////////////////////////////////////////////////////////////////////////////

pub struct Gizmos {
    /// immediate vertices, written to vertex_buffer every frame.
    vertex_queue: Vec<Vertex>,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: GrowableBuffer<Vertex>,
}
impl Gizmos {
    pub fn new(ctx: &GraphicsContext, camera: &Camera3dGR) -> Self {
        let vertex_buffer = GrowableBuffer::new(&ctx.device, 256, BufferUsages::VERTEX);
        let pipeline = create_pipeline(&ctx.device, camera);
        Gizmos {
            pipeline,
            vertex_queue: vec![],
            vertex_buffer,
        }
    }

    pub fn render<'encoder>(
        &'encoder self,
        render_pass: &mut wgpu::RenderPass<'encoder>,
        camera: &'encoder Camera3dGR,
    ) {
        if self.vertex_buffer.len() == 0 {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.buffer().slice(..));
        render_pass.draw(0..self.vertex_buffer.len() as u32, 0..1);
    }
}

impl Prepare for Gizmos {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) {
        self.vertex_buffer
            .prepare(&self.vertex_queue, device, queue);
        self.vertex_queue.clear();
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Renderer
// /////////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: Color,
}

impl VertexT for Vertex {
    const ATTRIBUTES: &'static [Attribute] = &[
        Attribute::new("pos", wgpu::VertexFormat::Float32x3),
        Attribute::new("color", wgpu::VertexFormat::Float32x4),
    ];
}

fn create_pipeline(device: &wgpu::Device, camera: &Camera3dGR) -> wgpu::RenderPipeline {
    let label = "Gizmos";

    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some(&format!("{label} ShaderModule")),
        source: wgpu::ShaderSource::Wgsl(include_str!("gizmos.wgsl").into()),
    });

    let _empty = &mut vec![];
    let vertex_buffers_layout = &[Vertex::vertex_buffer_layout(0, false, _empty)];

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} PipelineLayout")),
        bind_group_layouts: &[camera.bind_group_layout()],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("{label} ShaderModule")),
        layout: Some(&layout),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_buffers_layout,
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: HDR_COLOR_FORMAT,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
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
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        },
        multiview: None,
    })
}
