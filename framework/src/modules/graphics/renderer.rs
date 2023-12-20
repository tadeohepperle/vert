use std::{path::PathBuf, sync::Arc};

use wgpu::RenderPassColorAttachment;
use winit::dpi::PhysicalSize;

use crate::{
    constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT},
    modules::{egui::EguiState, watcher::FileWatcher},
};

use self::screen_space::ScreenSpaceRenderer;

use super::{
    elements::{gizmos::GizmosRenderer, texture::Texture},
    graphics_context::GraphicsContext,
    settings::GraphicsSettings,
    shader::{ShaderCodeSource, ShaderPipelineConfig, ShaderRendererT, ShaderT},
    Render,
};

pub mod screen_space;

pub struct Renderer {
    context: GraphicsContext,
    pub(crate) gizmos_renderer: GizmosRenderer,

    screen_space_renderer: ScreenSpaceRenderer,
    graphics_settings: GraphicsSettings,
    shader_renderers: Vec<DynShaderRenderer>,
}

pub struct DynShaderRenderer {
    watch_path: Option<PathBuf>,
    renderer: Box<dyn ShaderRendererT>,
}

impl Renderer {
    pub fn initialize(
        context: GraphicsContext,
        graphics_settings: GraphicsSettings,
    ) -> anyhow::Result<Self> {
        let screen_space_renderer = ScreenSpaceRenderer::create(&context);
        let gizmos_renderer: GizmosRenderer = GizmosRenderer::new(&context);

        Ok(Self {
            context,
            screen_space_renderer,
            gizmos_renderer,
            graphics_settings,
            shader_renderers: vec![],
        })
    }

    pub fn update(&mut self, file_watcher: &FileWatcher) {
        // check if any shader source file has been changed. If so, rebuild the renderer for that shader (inlcudes the wgpu::Pipeline)

        for r in self.shader_renderers.iter_mut() {
            if let Some(path) = &r.watch_path {
                if file_watcher.file_modified(path) {
                    r.renderer.rebuild(&self.context, pipeline_config());
                }
            }
        }
    }

    /// Creates a new renderer for this shader
    pub fn register_shader<T: ShaderT>(&mut self, watcher: &FileWatcher) {
        let source = &<T as ShaderT>::CODE_SOURCE;
        let watch_path = source.as_path_to_watch();
        if let Some(path) = &watch_path {
            watcher.watch(path);
        }

        let renderer = <T as ShaderT>::Renderer::new(&self.context, pipeline_config());
        let dyn_renderer = DynShaderRenderer {
            watch_path,
            renderer: Box::new(renderer),
        };
        self.shader_renderers.push(dyn_renderer);
    }

    pub fn resize(&mut self) {
        // recreate the depth and msaa texture
        self.screen_space_renderer.resize(&self.context);
    }

    pub fn prepare(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.gizmos_renderer.prepare();

        for r in self.shader_renderers.iter_mut() {
            r.renderer.prepare(&self.context, encoder);
        }
    }

    /// grabs the stuff he needs from the arenas and renders it.
    ///
    /// surface_view is expected to be in srbg u8 format
    pub fn render(
        &self,
        surface_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        arenas: &vert_core::arenas::Arenas,
    ) {
        // /////////////////////////////////////////////////////////////////////////////
        // MSAA HDR render pass
        // /////////////////////////////////////////////////////////////////////////////

        // create a new HDR MSSAx4 renderpass:
        let mut main_render_pass = self
            .screen_space_renderer
            .new_hdr_4xmsaa_render_pass(encoder, &self.graphics_settings);

        self.gizmos_renderer.render(&mut main_render_pass);
        for r in self.shader_renderers.iter() {
            r.renderer
                .render(&mut main_render_pass, &self.graphics_settings);
        }

        // // render color meshes:
        // self.color_mesh_render_pipeline
        //     .render_color_meshes(&mut main_render_pass, arenas);

        // // render ui rectangles:
        // self.ui_rect_render_pipeline.render_ui_rects(
        //     &mut main_render_pass,
        //     ui.prepared_ui_rects(),
        //     ui.text_atlas_texture(),
        // );

        // // render 3d triangles:
        // self.rect_3d_render_pipeline.render_3d_rects(
        //     &mut main_render_pass,
        //     ui.prepared_3d_rects(),
        //     ui.text_atlas_texture(),
        // );

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
fn pipeline_config() -> ShaderPipelineConfig {
    ShaderPipelineConfig {
        multisample: wgpu::MultisampleState {
            count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        },
        target: wgpu::ColorTargetState {
            format: HDR_COLOR_FORMAT,
            blend: Some(wgpu::BlendState {
                alpha: wgpu::BlendComponent::REPLACE,
                color: wgpu::BlendComponent::REPLACE,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        },
    }
}
