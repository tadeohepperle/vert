use std::sync::Arc;

use glam::{UVec2, Vec2};
use image::RgbaImage;
use vert_core::{arenas::Arenas, prelude::*};
use wgpu::{PrimitiveState, RenderPass, ShaderModuleDescriptor};

use crate::{
    constants::{COLOR_FORMAT, DEPTH_FORMAT},
    modules::{
        graphics::{graphics_context::GraphicsContext, VertexT},
        ui::{PeparedRects, RectInstanceBuffer},
    },
};

use super::{
    buffer::{IndexBuffer, InstanceBuffer, VertexBuffer},
    color::Color,
    screen_space::ScreenSpaceBindGroup,
    texture::{BindableTexture, Texture},
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
    /// used for setting the texture bindgroups for rects where no texture is defined.
    white_px: BindableTexture,
}

impl UiRectRenderPipeline {
    pub fn new(context: &GraphicsContext, screen_space_bind_group: ScreenSpaceBindGroup) -> Self {
        let device = &context.device;

        let white_px = BindableTexture::new(
            context,
            context.rgba_bind_group_layout,
            Texture::create_white_px_texture(device, &context.queue),
        );

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Ui Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui_rect.wgsl").into()),
        });

        // No vertices, just instances
        let vertex_and_transform_layout: [wgpu::VertexBufferLayout; 1] = [UiRectInstance::desc()];

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ui Rect Pipelinelayout"),
                bind_group_layouts: &[
                    screen_space_bind_group.layout(),
                    context.rgba_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ui Rect Pipeline"),
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
                        color: wgpu::BlendComponent::OVER,
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

        UiRectRenderPipeline {
            pipeline,
            screen_space_bind_group,
            index_buffer,
            white_px,
        }
    }

    /// Tadeo Hepperle, 2023-12-14, Interesting note: We don't need a vertex buffer to draw the rects.
    /// It is totally enough to have just one index buffer, that goes 0,1,3,0,2,3 to create a rect.
    /// And We have one instance buffer, where each rect has some data about it.
    /// Based on the index we can determine vertex position and color and uv for all of the 4 vertices in the vertex shader.
    /// That saves a lot of bandwidth, because for example for vertex positions, we just need 4 floats as a bounding box for the rect,
    /// instead of 4x2 floats if we would specify 4 vertices.
    ///
    /// I first thought we cannot draw without a vertex buffer and just an instance buffer in place of it, but it works well.
    pub fn render_ui_rects<'s: 'e, 'p, 'e>(
        &'s self,
        render_pass: &'p mut RenderPass<'e>,
        prepared_rects: &'e PeparedRects,
    ) {
        render_pass.set_pipeline(&self.pipeline);

        // screen space info and index buffer are fixed, because all rects have just 4 verts / 2 triangles.
        render_pass.set_bind_group(0, &self.screen_space_bind_group.bind_group(), &[]);
        render_pass.set_index_buffer(
            self.index_buffer.buffer().slice(..),
            wgpu::IndexFormat::Uint32,
        );

        // set the instance buffer: (no vertex buffer is used, instead just one big instance buffer that contains the sorted texture group ranges.)
        render_pass.set_vertex_buffer(0, prepared_rects.instance_buffer.buffer().slice(..));

        // draw instanced ranges of the instance buffer for each texture region:
        let index_count = self.index_buffer.len();
        assert_eq!(index_count, 6);
        for (range, texture) in prepared_rects.texture_groups.iter() {
            let texture: &BindableTexture = match texture {
                Some(texture) => texture,
                None => &self.white_px,
            };
            render_pass.set_bind_group(1, &texture.bind_group, &[]);
            render_pass.draw_indexed(0..index_count, 0, range.start..range.end);
        }
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
    pub border_radius: [f32; 4],
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
                // border radius
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
