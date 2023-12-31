use std::sync::{LazyLock, Mutex};

use glam::{vec3, Vec3};
use wgpu::{BufferUsages, FragmentState, PrimitiveState, ShaderModuleDescriptor, VertexState};

use crate::{
    constants::DEPTH_FORMAT,
    modules::graphics::{
        elements::{buffer::GrowableBuffer, color::Color},
        graphics_context::GraphicsContext,
        statics::{camera::Camera, StaticBindGroup},
        PipelineSettings,
    },
};

use super::{Attribute, RendererT, VertexT, FRAGMENT_ENTRY_POINT, VERTEX_ENTRY_POINT};
// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl Gizmos {
    pub fn draw_line(from: Vec3, to: Vec3, color: Color) {
        let mut queue = LINE_QUEUE.lock().unwrap();
        queue.push(Vertex {
            pos: [from.x, from.y, from.z],
            color,
        });
        queue.push(Vertex {
            pos: [to.x, to.y, to.z],
            color,
        });
    }

    pub fn draw_xyz() {
        let mut queue = LINE_QUEUE.lock().unwrap();
        queue.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::RED,
        });
        queue.push(Vertex {
            pos: [1.0, 0.0, 0.0],
            color: Color::RED,
        });

        queue.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::GREEN,
        });
        queue.push(Vertex {
            pos: [0.0, 1.0, 0.0],
            color: Color::GREEN,
        });

        queue.push(Vertex {
            pos: [0.0, 0.0, 0.0],
            color: Color::BLUE,
        });
        queue.push(Vertex {
            pos: [0.0, 0.0, 1.0],
            color: Color::BLUE,
        });
    }

    pub fn draw_cube(position: Vec3, side_len: f32, color: Color) {
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

        let mut queue = LINE_QUEUE.lock().unwrap();
        for (from, to) in lines {
            queue.push(Vertex {
                pos: [from.x, from.y, from.z],
                color,
            });
            queue.push(Vertex {
                pos: [to.x, to.y, to.z],
                color,
            });
        }
    }
}

/// Line segments
static LINE_QUEUE: LazyLock<Mutex<Vec<Vertex>>> = LazyLock::new(|| Mutex::new(vec![]));

// /////////////////////////////////////////////////////////////////////////////
// Renderer
// /////////////////////////////////////////////////////////////////////////////

pub struct Gizmos {
    pipeline: wgpu::RenderPipeline,
    previous_vertex_queue: Vec<Vertex>,
    vertex_buffer: GrowableBuffer<Vertex>,
}

impl RendererT for Gizmos {
    fn new(context: &GraphicsContext, settings: PipelineSettings) -> Self
    where
        Self: Sized,
    {
        let device = &context.device;
        let label = "Gizmos";

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(&format!("{label} ShaderModule")),
            source: wgpu::ShaderSource::Wgsl(include_str!("gizmos.wgsl").into()),
        });

        let _empty = &mut vec![];
        let vertex_buffers_layout = &[Vertex::vertex_buffer_layout(0, false, _empty)];

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{label} PipelineLayout")),
            bind_group_layouts: &[Camera::bind_group_layout()],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{label} ShaderModule")),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: VERTEX_ENTRY_POINT,
                buffers: vertex_buffers_layout,
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: FRAGMENT_ENTRY_POINT,
                targets: &[Some(Self::color_target_state(settings.format))],
            }),
            primitive: Self::primitive(),
            depth_stencil: Self::depth_stencil(),
            multisample: settings.multisample,
            multiview: None,
        });

        let vertex_buffer = GrowableBuffer::new(device, 256, BufferUsages::VERTEX);

        Self {
            pipeline,
            vertex_buffer,
            previous_vertex_queue: vec![],
        }
    }

    fn depth_stencil() -> Option<wgpu::DepthStencilState>
    where
        Self: Sized,
    {
        Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        })
    }

    fn primitive() -> wgpu::PrimitiveState
    where
        Self: Sized,
    {
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        }
    }

    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder) {
        let mut queue = LINE_QUEUE.lock().unwrap();
        self.previous_vertex_queue.clear();
        std::mem::swap(&mut self.previous_vertex_queue, &mut queue);
        self.vertex_buffer
            .prepare(&self.previous_vertex_queue, &context.queue, &context.device);
    }

    fn render<'pass, 'encoder>(
        &'encoder self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
        graphics_settings: &crate::modules::graphics::settings::GraphicsSettings,
        asset_store: &'encoder crate::modules::assets::asset_store::AssetStore<'encoder>,
    ) {
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
