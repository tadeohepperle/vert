use vert_core::prelude::*;
use wgpu::{PrimitiveState, RenderPass, ShaderModuleDescriptor};

use crate::{
    constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT, SURFACE_COLOR_FORMAT},
    modules::graphics::{graphics_context::GraphicsContext, VertexT},
};

use super::{
    buffer::IndexBuffer,
    camera::CameraBindGroup,
    color::Color,
    rect::{PeparedRects, Rect, RectT, RectTexture},
    texture::{BindableTexture, Texture},
    transform::TransformRaw,
    ui_rect::UiRect,
};

impl RectT for Rect3D {}

/// Should be drawn in one big instance buffer for all rectangles.
/// No vertex buffer needed.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rect3D {
    pub ui_rect: UiRect,
    // the transform of this Rect3D instance in 3d space.
    // the pos should be seen as an offset from this position.
    // If the transform is not rotated, the
    pub transform: TransformRaw,
}

impl VertexT for Rect3D {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Rect3D>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // pos   (this pos in kinda unnecessary since in the shader it is combined with the transform mat4 below)
                // so we could also combine it upfront. But lets keep it, to have an easier time calculating letter layouts.
                // (in one word we probably will just have the same transform Mat4 for all letters, but they differ in their pos.)
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
                // leave one free slot here for later...
                // ...
                // transform (this is quite a lot of date for each transform, hmmmmm....)
                // maybe not the best idea to have this on instances? Going back to separate vertex and instance buffers?
                // imagine we have a text with 100 letters, then we send 100 times the same Mat4, same border radius and same color to the gpu for each instance.
                // This is clearly not optimal. But maybe fine at the moment.
                // On the other hand having the transforms for each letter means it is super simple to make letters rain down in like a spell casting game or so.
                //
                // but also letters never need border radius, so this is also a waste here... probably split up pipelines for font rendering and rect rendering in the future.
                // definitely when we want rects that not only have border radius but also real borders, box shadows and other sdf stuff.
                //
                // We could also use 2 fields for the color to indicate gradients, etc. Useful for text? Idk. But for UI rects this would be nice.
                // again this probably more applies for ui rects and not the 3d rects.
                // or should we set the transform by setting a bindgroup and combining it all? So many options...
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 20]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 24]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 28]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct Rect3DRenderPipeline {
    pipeline: wgpu::RenderPipeline,
    camera_bind_group: CameraBindGroup,
    index_buffer: IndexBuffer,
    /// used for setting the texture bindgroups for rects where no texture is defined.
    white_px: BindableTexture,
}

impl Rect3DRenderPipeline {
    pub fn new(context: &GraphicsContext, camera_bind_group: CameraBindGroup) -> Self {
        let device = &context.device;

        let white_px = BindableTexture::new(
            context,
            context.rgba_bind_group_layout,
            Texture::create_white_px_texture(device, &context.queue),
        );

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Rect 3d Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("rect_3d.wgsl").into()),
        });

        // No vertices, just instances
        let vertex_and_transform_layout: [wgpu::VertexBufferLayout; 1] = [Rect3D::desc()];

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Rect 3d Pipelinelayout"),
                bind_group_layouts: &[camera_bind_group.layout(), context.rgba_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rect 3d Pipeline"),
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
                cull_mode: Some(wgpu::Face::Back), // setting none here, allows for two sided sprites.
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

        Rect3DRenderPipeline {
            pipeline,
            camera_bind_group,
            index_buffer,
            white_px,
        }
    }

    ///
    pub fn render_3d_rects<'s: 'e, 'p, 'e>(
        &'s self,
        render_pass: &'p mut RenderPass<'e>,
        prepared_rects: &'e PeparedRects<Rect3D>,
        text_atlas_texture: &'e BindableTexture,
    ) {
        if prepared_rects.texture_groups.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);

        // screen space info and index buffer are fixed, because all rects have just 4 verts / 2 triangles.
        render_pass.set_bind_group(0, &self.camera_bind_group.bind_group(), &[]);
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
