use std::sync::Arc;

use wgpu::RenderPassColorAttachment;
use winit::dpi::PhysicalSize;

use crate::{
    constants::DEPTH_FORMAT,
    modules::{egui::EguiState, ui::ImmediateUi},
};

use self::screen_space::ScreenSpaceRenderer;

use super::{
    elements::{
        color_mesh::ColorMeshRenderPipeline, gizmos::GizmosRenderer, rect_3d::Rect3DRenderPipeline,
        texture::Texture, ui_rect::UiRectRenderPipeline,
    },
    graphics_context::GraphicsContext,
    settings::GraphicsSettings,
    Render,
};

pub mod screen_space;

pub struct Renderer {
    context: GraphicsContext,

    color_mesh_render_pipeline: ColorMeshRenderPipeline,
    ui_rect_render_pipeline: UiRectRenderPipeline,
    rect_3d_render_pipeline: Rect3DRenderPipeline,
    pub(crate) gizmos_renderer: GizmosRenderer,

    screen_space_renderer: ScreenSpaceRenderer,
    graphics_settings: GraphicsSettings,
}

impl Renderer {
    pub fn initialize(
        context: GraphicsContext,
        graphics_settings: GraphicsSettings,
    ) -> anyhow::Result<Self> {
        let screen_space_renderer = ScreenSpaceRenderer::create(&context);

        let color_mesh_render_pipeline = ColorMeshRenderPipeline::new(&context);
        let ui_rect_render_pipeline = UiRectRenderPipeline::new(&context);
        let rect_3d_render_pipeline = Rect3DRenderPipeline::new(&context);
        let gizmos_renderer = GizmosRenderer::new(&context);

        Ok(Self {
            context,
            screen_space_renderer,
            color_mesh_render_pipeline,
            ui_rect_render_pipeline,
            rect_3d_render_pipeline,
            gizmos_renderer,
            graphics_settings,
        })
    }

    pub fn resize(&mut self) {
        // recreate the depth and msaa texture
        self.screen_space_renderer.resize(&self.context);
    }

    /// grabs the stuff he needs from the arenas and renders it.
    ///
    /// surface_view is expected to be in srbg u8 format
    pub fn render(
        &self,
        surface_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        arenas: &vert_core::arenas::Arenas,
        ui: &ImmediateUi,
    ) {
        // /////////////////////////////////////////////////////////////////////////////
        // MSAA HDR render pass
        // /////////////////////////////////////////////////////////////////////////////

        // create a new HDR MSSAx4 renderpass:
        let mut main_render_pass = self
            .screen_space_renderer
            .new_hdr_4xmsaa_render_pass(encoder, &self.graphics_settings);

        self.gizmos_renderer.render(&mut main_render_pass);

        // render color meshes:
        self.color_mesh_render_pipeline
            .render_color_meshes(&mut main_render_pass, arenas);

        // render ui rectangles:
        self.ui_rect_render_pipeline.render_ui_rects(
            &mut main_render_pass,
            ui.prepared_ui_rects(),
            ui.text_atlas_texture(),
        );

        // render 3d triangles:
        self.rect_3d_render_pipeline.render_3d_rects(
            &mut main_render_pass,
            ui.prepared_3d_rects(),
            ui.text_atlas_texture(),
        );

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
