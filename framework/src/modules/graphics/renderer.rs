use std::{path::PathBuf, sync::Arc};

use wgpu::RenderPassColorAttachment;
use winit::dpi::PhysicalSize;

use crate::{
    constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT},
    modules::{assets::asset_store::AssetStore, egui::EguiState},
};

use self::screen_space::ScreenSpaceRenderer;

use super::{
    elements::texture::Texture, graphics_context::GraphicsContext, settings::GraphicsSettings,
    shader::RendererT,
};

pub mod screen_space;

pub struct Renderer {
    context: GraphicsContext,
    screen_space_renderer: ScreenSpaceRenderer,
    graphics_settings: GraphicsSettings,
    shader_renderers: Vec<Box<dyn RendererT>>,
}

impl Renderer {
    pub fn initialize(
        context: GraphicsContext,
        graphics_settings: GraphicsSettings,
    ) -> anyhow::Result<Self> {
        let screen_space_renderer = ScreenSpaceRenderer::create(&context);

        Ok(Self {
            context,
            screen_space_renderer,
            graphics_settings,
            shader_renderers: vec![],
        })
    }

    /// Creates a new renderer for this shader
    pub fn register_renderer<T: RendererT>(&mut self) {
        let renderer = <T as RendererT>::new(&self.context, pipeline_settings());
        self.shader_renderers.push(Box::new(renderer));
    }

    pub fn resize(&mut self) {
        // recreate the depth and msaa texture
        self.screen_space_renderer.resize(&self.context);
    }

    pub fn prepare(&mut self, encoder: &mut wgpu::CommandEncoder) {
        for r in self.shader_renderers.iter_mut() {
            r.prepare(&self.context, encoder);
        }
    }

    /// grabs the stuff he needs from the arenas and renders it.
    ///
    /// surface_view is expected to be in srbg u8 format
    pub fn render<'e>(
        &self,
        surface_view: &wgpu::TextureView,
        encoder: &'e mut wgpu::CommandEncoder,
        asset_store: &'e AssetStore<'e>,
    ) {
        // /////////////////////////////////////////////////////////////////////////////
        // MSAA HDR render pass
        // /////////////////////////////////////////////////////////////////////////////

        // create a new HDR MSSAx4 renderpass:
        let mut main_render_pass = self
            .screen_space_renderer
            .new_hdr_4xmsaa_render_pass(encoder, &self.graphics_settings);

        for r in self.shader_renderers.iter() {
            r.render(&mut main_render_pass, &self.graphics_settings, asset_store);
        }

        drop(main_render_pass);

        // /////////////////////////////////////////////////////////////////////////////
        // Post processing, HDR -> SRGB u8 tonemapping
        // /////////////////////////////////////////////////////////////////////////////

        self.screen_space_renderer.render_to_surface_view(
            encoder,
            surface_view,
            &self.graphics_settings,
        );
    }

    pub fn settings(&self) -> &GraphicsSettings {
        &self.graphics_settings
    }

    pub fn settings_mut(&mut self) -> &mut GraphicsSettings {
        &mut self.graphics_settings
    }
}

/// todo! integrate and update this with graphics settings.
fn pipeline_settings() -> PipelineSettings {
    PipelineSettings {
        multisample: wgpu::MultisampleState {
            count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        },
        format: HDR_COLOR_FORMAT,
    }
}

#[derive(Debug, Clone)]
pub struct PipelineSettings {
    pub multisample: wgpu::MultisampleState,
    pub format: wgpu::TextureFormat,
}
