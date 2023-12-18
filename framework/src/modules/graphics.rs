use std::sync::Arc;

use self::graphics_context::GraphicsContext;
use vert_core::prelude::*;

pub mod elements;
pub mod graphics_context;
pub mod renderer;
pub mod settings;
pub mod shader;

pub trait VertexT: Copy + bytemuck::Pod + bytemuck::Zeroable {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

reflect!(Prepare);
pub trait Prepare {
    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder);
}

pub trait Render {
    fn render<'s: 'encoder, 'pass, 'encoder>(
        &'s self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
    );
}
