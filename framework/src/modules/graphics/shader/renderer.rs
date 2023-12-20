use std::{borrow::Cow, path::PathBuf};

use crate::modules::graphics::{
    graphics_context::GraphicsContext, renderer::PipelineSettings, settings::GraphicsSettings,
};

const VERTEX_ENTRY_POINT: &str = "vs_main";
const FRAGMENT_ENTRY_POINT: &str = "fs_main";

pub trait RendererT: 'static {
    fn new(graphics_context: &GraphicsContext, pipeline_config: PipelineSettings) -> Self
    where
        Self: Sized;

    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder);

    fn render<'s: 'encoder, 'pass, 'encoder>(
        &'s self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
        graphics_settings: &GraphicsSettings,
    );
}
