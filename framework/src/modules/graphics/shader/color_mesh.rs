use crate::modules::graphics::elements::{color::Color, transform::TransformRaw};

use super::{
    super::statics::camera::Camera,
    vertex::{IntoVertexAttributes, VertexAttribute},
    ShaderT,
};

pub struct ColorMeshShader;

impl ShaderT for ColorMeshShader {
    type BindGroupLayouts = Camera;
    type Vertex = Vertex;
    type Instance = TransformRaw;
    type VertexOutput = Color;
}

pub struct Vertex {
    pub pos: [f32; 3],
    pub color: Color,
}

impl IntoVertexAttributes for Vertex {
    fn attributes() -> &'static [super::vertex::VertexAttribute] {
        &[
            VertexAttribute {
                ident: "pos",
                format: wgpu::VertexFormat::Float32x3,
            },
            VertexAttribute {
                ident: "color",
                format: wgpu::VertexFormat::Float32x4,
            },
        ]
    }
}
