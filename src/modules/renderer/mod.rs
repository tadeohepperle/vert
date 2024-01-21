use crate::elements::Color;

pub mod screen_textures;
pub use screen_textures::{DepthTexture, HdrTexture, ScreenTextures, ScreenVertexShader};

pub mod bloom;
pub use bloom::{Bloom, BloomSettings};

pub mod tone_mapping;
pub use tone_mapping::AcesToneMapping;

pub mod gizmos;
pub use gizmos::Gizmos;

pub mod color_mesh;
pub use color_mesh::ColorMeshRenderer;

pub mod ui_rect;
pub use ui_rect::UiRectRenderer;

pub mod world_rect;
pub use world_rect::WorldRectRenderer;

pub mod text_renderer;
pub use text_renderer::TextRenderer;

pub const SURFACE_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const HDR_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const MSAA_SAMPLE_COUNT: u32 = 4;

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
    fn vertex_buffer_layout(
        shader_location_offset: usize,
        is_instance: bool,
        empty_vec: &mut Vec<wgpu::VertexAttribute>,
    ) -> wgpu::VertexBufferLayout<'_> {
        let mut shader_location_offset: u32 = shader_location_offset as u32;
        if !is_instance {
            assert_eq!(shader_location_offset, 0)
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

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode,
            attributes: empty_vec,
        }
    }
}

impl VertexT for Color {
    const ATTRIBUTES: &'static [Attribute] =
        &[Attribute::new("color", wgpu::VertexFormat::Float32x4)];
}
