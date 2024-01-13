use glam::Vec3;

use crate::elements::{Color, IndexBuffer, ToRaw, VertexBuffer};

pub struct PbrRenderer {
    sun: DirectionalLight,
}

pub struct PbrMesh {
    vertex_buffer: VertexBuffer<Vertex>,
    index_buffer: IndexBuffer,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pos: [f32; 3],
    diffuse_uv: [f32; 2],
}

pub struct DirectionalLight {
    direction: Vec3,
    color: Color,
    intensity: f32,
}
