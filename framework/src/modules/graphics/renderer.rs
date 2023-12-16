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
        camera::{Camera, CameraBindGroup},
        color_mesh::ColorMeshRenderPipeline,
        gizmos::GizmosRenderer,
        rect_3d::Rect3DRenderPipeline,
        screen_space::ScreenSpaceBindGroup,
        texture::Texture,
        ui_rect::UiRectRenderPipeline,
    },
    graphics_context::GraphicsContext,
    Render,
};

pub mod bloom;
pub mod screen_space;

pub struct Renderer {
    context: GraphicsContext,

    color_mesh_render_pipeline: ColorMeshRenderPipeline,
    ui_rect_render_pipeline: UiRectRenderPipeline,
    rect_3d_render_pipeline: Rect3DRenderPipeline,
    pub(crate) gizmos_renderer: GizmosRenderer,

    screen_space_renderer: ScreenSpaceRenderer,
}

impl Renderer {
    pub fn initialize(
        context: GraphicsContext,
        camera_bind_group: CameraBindGroup,
        screen_space_bind_group: ScreenSpaceBindGroup,
    ) -> anyhow::Result<Self> {
        let screen_space_renderer = ScreenSpaceRenderer::create(&context);

        let color_mesh_render_pipeline =
            ColorMeshRenderPipeline::new(&context, camera_bind_group.clone());
        let ui_rect_render_pipeline = UiRectRenderPipeline::new(&context, screen_space_bind_group);
        let rect_3d_render_pipeline =
            Rect3DRenderPipeline::new(&context, camera_bind_group.clone());
        let gizmos_renderer = GizmosRenderer::new(&context, camera_bind_group);

        Ok(Self {
            context,
            screen_space_renderer,
            color_mesh_render_pipeline,
            ui_rect_render_pipeline,
            rect_3d_render_pipeline,
            gizmos_renderer,
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
        let mut render_pass = self
            .screen_space_renderer
            .new_hdr_4xmsaa_render_pass(encoder);

        self.gizmos_renderer.render(&mut render_pass);

        // render color meshes:
        self.color_mesh_render_pipeline
            .render_color_meshes(&mut render_pass, arenas);

        // render ui rectangles:
        self.ui_rect_render_pipeline.render_ui_rects(
            &mut render_pass,
            ui.prepared_ui_rects(),
            ui.text_atlas_texture(),
        );

        // render 3d triangles:
        self.rect_3d_render_pipeline.render_3d_rects(
            &mut render_pass,
            ui.prepared_3d_rects(),
            ui.text_atlas_texture(),
        );

        drop(render_pass);

        // /////////////////////////////////////////////////////////////////////////////
        // Post processing, HDR -> SRGB u8 tonemapping
        // /////////////////////////////////////////////////////////////////////////////

        self.screen_space_renderer
            .render_to_surface_view(encoder, surface_view);
    }
}
