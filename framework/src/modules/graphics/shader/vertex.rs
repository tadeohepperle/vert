use crate::modules::graphics::elements::{color::Color, transform::TransformRaw};

pub struct VertexAttribute {
    pub ident: &'static str,
    pub format: wgpu::VertexFormat,
}

pub trait IntoVertexAttributes {
    fn attributes() -> &'static [VertexAttribute];

    /// Returns none, if the attributes are empty. Blanket implementation for () does this.
    ///
    /// If `instance` is true, this gets wgpu::VertexStepMode::Instance otherwise wgpu::VertexStepMode::Vertex
    /// `shader_location_offset` can be set, to make sure that the properties of vertices and instances
    /// do not overlap in terms of their `shader_location`. E.g. if we want to render color meshes,
    /// where each vertex is a 3d pos (loc = 0) and a color (loc = 1), then we want to set the shader_location_offset
    /// to 2 for the VertexBufferLayout of the instances that are transforms (loc = 2, loc = 3, loc = 4, loc = 5).
    fn vertex_buffer_layout(
        instance: bool,
        shader_location_offset: u32,
    ) -> Option<wgpu::VertexBufferLayout<'static>> {
        todo!()
    }
}

impl IntoVertexAttributes for () {
    fn attributes() -> &'static [VertexAttribute] {
        &[]
    }
}

impl IntoVertexAttributes for Color {
    fn attributes() -> &'static [VertexAttribute] {
        &[VertexAttribute {
            ident: "color",
            format: wgpu::VertexFormat::Float32x4,
        }]
    }
}

impl IntoVertexAttributes for TransformRaw {
    fn attributes() -> &'static [VertexAttribute] {
        &[
            VertexAttribute {
                ident: "col1",
                format: wgpu::VertexFormat::Float32x4,
            },
            VertexAttribute {
                ident: "col2",
                format: wgpu::VertexFormat::Float32x4,
            },
            VertexAttribute {
                ident: "col3",
                format: wgpu::VertexFormat::Float32x4,
            },
            VertexAttribute {
                ident: "translation",
                format: wgpu::VertexFormat::Float32x4,
            },
        ]
    }
}
