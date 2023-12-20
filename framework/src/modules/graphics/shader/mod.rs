use std::{borrow::Cow, path::PathBuf};

use crate::constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT};
use indoc::indoc;
use smallvec::{smallvec, SmallVec};
use wgpu::{BindGroupLayout, PrimitiveState};

use super::{
    elements::{color::Color, transform::TransformRaw},
    graphics_context::GraphicsContext,
    renderer::PipelineSettings,
    settings::GraphicsSettings,
};

pub mod color_mesh;
pub mod ui_rect;

const VERTEX_ENTRY_POINT: &str = "vs_main";
const FRAGMENT_ENTRY_POINT: &str = "fs_main";

pub trait RendererT: 'static {
    fn new(graphics_context: &GraphicsContext, settings: PipelineSettings) -> Self
    where
        Self: Sized;

    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder);

    fn render<'s: 'encoder, 'pass, 'encoder>(
        &'s self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
        graphics_settings: &GraphicsSettings,
    );

    /// defaults, can be overriden
    fn primitive() -> wgpu::PrimitiveState
    where
        Self: Sized,
    {
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        }
    }

    /// defaults, can be overriden
    fn depth_stencil() -> Option<wgpu::DepthStencilState>
    where
        Self: Sized,
    {
        Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        })
    }
}

pub struct Attribute {
    pub ident: &'static str,
    pub format: wgpu::VertexFormat,
}
impl Attribute {
    pub const fn new(ident: &'static str, format: wgpu::VertexFormat) -> Self {
        Self { ident, format }
    }
}

pub trait VertexT: 'static + Sized {
    const ATTRIBUTES: &'static [Attribute];

    /// We pass in `empty_vec`, because Rust does not have super let lifetimes yet... sigh...
    fn vertex_buffer_layout<'a>(
        shader_location_offset: usize,
        is_instance: bool,
        empty_vec: &'a mut Vec<wgpu::VertexAttribute>,
    ) -> wgpu::VertexBufferLayout<'a> {
        let mut shader_location_offset: u32 = shader_location_offset as u32;
        if is_instance {
            assert_ne!(shader_location_offset, 0)
        }
        assert!(empty_vec.is_empty());
        let attributes = Self::ATTRIBUTES;

        let mut offset: u64 = 0;
        for a in attributes {
            empty_vec.push(wgpu::VertexAttribute {
                format: a.format,
                offset,
                shader_location: shader_location_offset,
            });
            shader_location_offset += 1;
            offset += a.format.size();
        }

        let step_mode = if is_instance {
            wgpu::VertexStepMode::Instance
        } else {
            wgpu::VertexStepMode::Vertex
        };
        let layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode,
            attributes: empty_vec,
        };
        layout
    }
}

impl VertexT for Color {
    const ATTRIBUTES: &'static [Attribute] =
        &[Attribute::new("color", wgpu::VertexFormat::Float32x4)];
}
