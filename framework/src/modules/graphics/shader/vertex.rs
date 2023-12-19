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
}

impl VertexT for () {
    const ATTRIBUTES: &'static [VertexAttribute] = &[];
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

/// Returns None, if the attributes are empty. Blanket implementation for () does this.
///
/// If `instance` is true, this gets wgpu::VertexStepMode::Instance otherwise wgpu::VertexStepMode::Vertex
/// `shader_location_offset` can be set, to make sure that the properties of vertices and instances
/// do not overlap in terms of their `shader_location`. E.g. if we want to render color meshes,
/// where each vertex is a 3d pos (loc = 0) and a color (loc = 1), then we want to set the shader_location_offset
/// to 2 for the VertexBufferLayout of the instances that are transforms (loc = 2, loc = 3, loc = 4, loc = 5).
///
/// We pass in attributes_with_positions, because Rust does not have super let lifetimes yet... sigh...
pub fn wgpu_vertex_buffer_layout<'a, V: VertexT>(
    instance: bool,
    mut shader_location_offset: u32,
    attributes_with_positions: &'a mut Vec<wgpu::VertexAttribute>,
) -> Option<wgpu::VertexBufferLayout<'a>> {
    assert!(attributes_with_positions.is_empty());
    let attributes = V::ATTRIBUTES;
    if attributes.is_empty() {
        return None;
    }

    let mut offset: u64 = 0;
    for a in attributes {
        attributes_with_positions.push(wgpu::VertexAttribute {
            format: a.format,
            offset,
            shader_location: shader_location_offset,
        });
        shader_location_offset += 1;
        offset += a.format.size();
    }

    let step_mode = if instance {
        wgpu::VertexStepMode::Instance
    } else {
        wgpu::VertexStepMode::Vertex
    };
    let layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<V>() as u64,
        step_mode,
        attributes: attributes_with_positions,
    };
    Some(layout)
}
