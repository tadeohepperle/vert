use std::sync::Arc;

use self::graphics_context::GraphicsContext;

pub mod elements;
pub mod graphics_context;
pub mod renderer;

pub trait VertexT: Copy + bytemuck::Pod + bytemuck::Zeroable {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

pub trait Prepare {
    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder);
}

pub trait Render {
    fn render<'s: 'encoder, 'pass, 'encoder>(
        &'s mut self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
    );
}
