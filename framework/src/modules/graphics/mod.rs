use self::{
    graphics_context::GraphicsContext,
    post_processing::{bloom::Bloom, tonemapping::AcesToneMapping, PostProcessingEffectT},
    screen_textures::{DepthTexture, HdrTexture},
    settings::GraphicsSettings,
    shader::{
        color_mesh::ColorMeshRenderer, gizmos::Gizmos, text::TextRenderer, ui_rect::UiRectRenderer,
        world_rect::WorldRectRenderer, RendererT,
    },
};
pub mod elements;
pub mod graphics_context;
pub mod post_processing;
pub mod settings;
pub mod shader;
pub mod statics;

use std::{path::PathBuf, sync::Arc};

use log::warn;
use wgpu::{RenderPassColorAttachment, ShaderModule, ShaderModuleDescriptor};
use winit::dpi::PhysicalSize;

use crate::{
    constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT},
    modules::{assets::asset_store::AssetStore, egui::EguiState},
};

pub mod screen_textures;

pub struct Renderer {
    context: GraphicsContext,
    settings: GraphicsSettings,
    shader_renderers: Vec<Box<dyn RendererT>>,
    post_processing_effects: Vec<Box<dyn PostProcessingEffectT>>,
    /// This is for tonemapping:
    tonemapping_effect: Option<Box<dyn PostProcessingEffectT>>,
    screen_vertex_shader: Arc<ScreenVertexShader>,
    depth_texture: DepthTexture,
    hdr_msaa_texture: HdrTexture,
    hdr_resolve_target: HdrTexture,
}

impl Renderer {
    pub fn new_with_defaults(
        context: GraphicsContext,
        settings: GraphicsSettings,
    ) -> anyhow::Result<Self> {
        let mut renderer = Self::new(context, settings)?;
        renderer.register_renderer::<ColorMeshRenderer>();
        renderer.register_renderer::<UiRectRenderer>();
        renderer.register_renderer::<WorldRectRenderer>();
        renderer.register_renderer::<TextRenderer>();
        renderer.register_renderer::<Gizmos>();

        renderer.add_post_processing_effect::<Bloom>();
        renderer.set_tonemapping_effect::<AcesToneMapping>();

        Ok(renderer)
    }

    pub fn new(context: GraphicsContext, settings: GraphicsSettings) -> anyhow::Result<Self> {
        let screen_vertex_shader = Arc::new(ScreenVertexShader::new(&context.device));

        let msaa_depth_texture = DepthTexture::create(&context);
        let msaa_hdr_texture = HdrTexture::create_screen_sized(&context, MSAA_SAMPLE_COUNT);
        let hdr_resolve_target_texture = HdrTexture::create_screen_sized(&context, 1);

        Ok(Self {
            context,
            settings,
            shader_renderers: vec![],
            screen_vertex_shader,
            depth_texture: msaa_depth_texture,
            hdr_msaa_texture: msaa_hdr_texture,
            hdr_resolve_target: hdr_resolve_target_texture,
            post_processing_effects: vec![],
            tonemapping_effect: None,
        })
    }

    /// Creates a new renderer for this shader
    pub fn register_renderer<T: RendererT>(&mut self) {
        let renderer = <T as RendererT>::new(&self.context, pipeline_settings());
        self.shader_renderers.push(Box::new(renderer));
    }

    pub fn add_post_processing_effect<S: PostProcessingEffectT + 'static>(&mut self) {
        self.post_processing_effects
            .push(Box::new(S::new(&self.context, &self.screen_vertex_shader)))
    }

    pub fn set_tonemapping_effect<S: PostProcessingEffectT + 'static>(&mut self) {
        self.tonemapping_effect = Some(Box::new(S::new(&self.context, &self.screen_vertex_shader)));
    }

    pub fn resize(&mut self) {
        // recreate the depth and msaa texture
        self.depth_texture.recreate(&self.context);
        self.hdr_msaa_texture = HdrTexture::create_screen_sized(&self.context, MSAA_SAMPLE_COUNT);
        self.hdr_resolve_target = HdrTexture::create_screen_sized(&self.context, 1);

        // recreate effect textures too (e.g. for bloom)
        for effect in self.post_processing_effects.iter_mut() {
            effect.resize(&self.context);
        }

        if let Some(effect) = &mut self.tonemapping_effect {
            effect.resize(&self.context);
        }
    }

    pub fn prepare(&mut self, encoder: &mut wgpu::CommandEncoder) {
        for r in self.shader_renderers.iter_mut() {
            r.prepare(&self.context, encoder);
        }
    }

    fn new_hdr_target_render_pass<'e>(
        &'e self,
        encoder: &'e mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'e> {
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: self.hdr_msaa_texture.view(),
            resolve_target: Some(self.hdr_resolve_target.view()),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(self.settings.clear_color.into()),
                store: wgpu::StoreOp::Store,
            },
        };
        let main_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Renderpass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.depth_texture.view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        main_render_pass
    }

    /// grabs the stuff he needs from the arenas and renders it.
    ///
    /// surface_view is expected to be in srbg u8 format
    pub fn render<'e>(
        &mut self,
        surface_view: &wgpu::TextureView,
        encoder: &'e mut wgpu::CommandEncoder,
        asset_store: &'e AssetStore<'e>,
    ) {
        // /////////////////////////////////////////////////////////////////////////////
        // MSAA HDR render pass
        // /////////////////////////////////////////////////////////////////////////////

        // create a new HDR MSSAx4 renderpass, render objects for frame.
        {
            let mut render_pass = self.new_hdr_target_render_pass(encoder);
            for r in self.shader_renderers.iter() {
                r.render(&mut render_pass, &self.settings, asset_store);
            }
        }
        // bloom, vignette, etc.
        for effect in self.post_processing_effects.iter_mut() {
            effect.apply(
                encoder,
                self.hdr_resolve_target.bind_group(),
                self.hdr_resolve_target.view(),
                &self.settings,
            );
        }

        // tonemapping
        if let Some(effect) = &mut self.tonemapping_effect {
            effect.apply(
                encoder,
                self.hdr_resolve_target.bind_group(),
                surface_view,
                &self.settings,
            );
        } else {
            warn!("No mapping from HDR to SDR specified!")
        }
    }

    pub fn settings(&self) -> &GraphicsSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut GraphicsSettings {
        &mut self.settings
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

// /////////////////////////////////////////////////////////////////////////////
// Data
// /////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct PipelineSettings {
    pub multisample: wgpu::MultisampleState,
    pub format: wgpu::TextureFormat,
}

/// Shader for a single triangle that covers the entire screen.
pub struct ScreenVertexShader(wgpu::ShaderModule);
impl ScreenVertexShader {}

impl ScreenVertexShader {
    fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Screen Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("screen.vert.wgsl").into()),
        });
        ScreenVertexShader(module)
    }

    fn vertex_state(&self) -> wgpu::VertexState<'_> {
        wgpu::VertexState {
            module: &self.0,
            entry_point: "vs_main",
            buffers: &[],
        }
    }
}
