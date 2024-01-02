use std::sync::Arc;

use wgpu::{CommandEncoder, TextureView};

pub mod bloom;
pub mod tonemapping;

use super::{graphics_context::GraphicsContext, settings::GraphicsSettings, ScreenVertexShader};

pub trait PostProcessingEffectT {
    fn new(context: &GraphicsContext, screen_vertex_shader: &Arc<ScreenVertexShader>) -> Self
    where
        Self: Sized;

    fn resize(&mut self, _context: &GraphicsContext) {}

    fn apply<'e>(
        &'e mut self,
        encoder: &'e mut CommandEncoder,
        input: &wgpu::BindGroup,
        output: &TextureView,
        graphics_settings: &GraphicsSettings,
    );
}

pub fn create_post_processing_pipeline() {}
