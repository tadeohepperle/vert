use std::{ops::DivAssign, sync::Arc};

use glam::{UVec2, Vec2};
use image::RgbaImage;
use vert_core::{arenas::Arenas, prelude::*};
use wgpu::{PrimitiveState, RenderPass, ShaderModuleDescriptor};

use crate::{
    constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT, SURFACE_COLOR_FORMAT},
    modules::graphics::{
        elements::rect::RectTexture, graphics_context::GraphicsContext,
        shader::bind_group::StaticBindGroup, statics::screen_size::ScreenSize, VertexT,
    },
};

use super::{
    buffer::IndexBuffer,
    color::Color,
    rect::{PeparedRects, Rect, RectT},
    texture::{BindableTexture, Texture},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiRect {
    // min x, min y, size x, size y
    pub pos: Rect,
    // min x, min y, size x, size y
    pub uv: Rect,
    pub color: Color,
    pub border_radius: [f32; 4],
}

impl RectT for UiRect {}

impl VertexT for UiRect {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<UiRect>() as wgpu::BufferAddress,
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

pub struct UiRectRenderPipeline {
    pipeline: wgpu::RenderPipeline,
    index_buffer: IndexBuffer,
    /// used for setting the texture bindgroups for rects where no texture is defined.
    white_px: BindableTexture,
}

impl UiRectRenderPipeline {
    pub fn new(context: &GraphicsContext) -> Self {
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
        let vertex_and_transform_layout: [wgpu::VertexBufferLayout; 1] = [UiRect::desc()];

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ui Rect Pipelinelayout"),
                bind_group_layouts: &[
                    ScreenSize::bind_group_layout(),
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
                    format: HDR_COLOR_FORMAT,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::OVER,
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

        // just a simple rect:
        let index_buffer = IndexBuffer::new(vec![0, 1, 2, 0, 2, 3], &context.device);

        UiRectRenderPipeline {
            pipeline,
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
        prepared_rects: &'e PeparedRects<UiRect>,
        text_atlas_texture: &'e BindableTexture,
    ) {
        if prepared_rects.texture_groups.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);

        // screen space info and index buffer are fixed, because all rects have just 4 verts / 2 triangles.
        render_pass.set_bind_group(0, ScreenSize::bind_group(), &[]);
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
            let texture_bind_group: &wgpu::BindGroup = match texture {
                RectTexture::White => &self.white_px.bind_group,
                RectTexture::Text => &text_atlas_texture.bind_group,
                RectTexture::Custom(tex) => &tex.bind_group,
            };
            render_pass.set_bind_group(1, texture_bind_group, &[]);
            render_pass.draw_indexed(0..index_count, 0, range.start..range.end);
        }
    }
}
