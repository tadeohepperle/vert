use crate::modules::graphics::elements::{color::Color, transform::TransformRaw};

pub struct VertexAttribute {
    pub ident: &'static str,
    pub format: wgpu::VertexFormat,
}
impl VertexAttribute {
    pub const fn new(ident: &'static str, format: wgpu::VertexFormat) -> Self {
        Self { ident, format }
    }
}

pub trait VertexT: 'static + Sized {
    const ATTRIBUTES: &'static [VertexAttribute];

    /// We pass in `empty_vec`, because Rust does not have super let lifetimes yet... sigh...
    fn vertex_buffer_layout<'a>(
        mut shader_location_offset: u32,
        is_instance: bool,
        empty_vec: &'a mut Vec<wgpu::VertexAttribute>,
    ) -> wgpu::VertexBufferLayout<'a> {
        if !is_instance {
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
    const ATTRIBUTES: &'static [VertexAttribute] =
        &[VertexAttribute::new("color", wgpu::VertexFormat::Float32x4)];
}

impl VertexT for TransformRaw {
    const ATTRIBUTES: &'static [VertexAttribute] = &[
        VertexAttribute::new("col1", wgpu::VertexFormat::Float32x4),
        VertexAttribute::new("col2", wgpu::VertexFormat::Float32x4),
        VertexAttribute::new("col3", wgpu::VertexFormat::Float32x4),
        VertexAttribute::new("translation", wgpu::VertexFormat::Float32x4),
    ];
}
