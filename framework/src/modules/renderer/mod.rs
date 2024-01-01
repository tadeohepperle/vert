use std::sync::Arc;

use crate::{
    app::{self, ModuleId, UntypedHandle},
    elements::Color,
    utils::{Timing, TimingQueue},
    Dependencies, Handle, Module, Plugin,
};

use self::{
    main_pass_renderer::{MainPassRenderer, MainPassRendererHandle},
    post_processing::{PostProcessingEffect, PostProcessingEffectHandle, ScreenVertexShader},
    screen_texture::{DepthTexture, HdrTexture},
};

use super::{input::ResizeEvent, GraphicsContext, Input, Schedule, Scheduler};
use log::error;
use wgpu::{CommandEncoder, RenderPass};

pub mod main_pass_renderer;
pub mod post_processing;
pub mod screen_texture;

pub use post_processing::{AcesToneMapping, ToneMappingSettings};

#[derive(Dependencies)]
pub struct RendererDependencies {
    scheduler: Handle<Scheduler>,
    ctx: Handle<GraphicsContext>,
    input: Handle<Input>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RendererSettings {
    clear_color: Color,
}

impl Default for RendererSettings {
    fn default() -> Self {
        Self {
            clear_color: Color::new(0.5, 0.3, 0.8),
        }
    }
}

pub const SURFACE_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const HDR_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const MSAA_SAMPLE_COUNT: u32 = 4;

pub struct Renderer {
    settings: RendererSettings,
    deps: RendererDependencies,

    depth_texture: DepthTexture,
    hdr_msaa_texture: HdrTexture,
    hdr_resolve_target: HdrTexture,

    main_pass_renderers: TimingQueue<MainPassRendererHandle>,
    post_processing_effects: TimingQueue<PostProcessingEffectHandle>,
    tone_mapping: Option<PostProcessingEffectHandle>,

    screen_vertex_shader: ScreenVertexShader,
}

impl Module for Renderer {
    type Config = RendererSettings;
    type Dependencies = RendererDependencies;

    fn new(settings: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let depth_texture = DepthTexture::create(&deps.ctx);
        let hdr_msaa_texture = HdrTexture::create_screen_sized(&deps.ctx, 4);
        let hdr_resolve_target = HdrTexture::create_screen_sized(&deps.ctx, 1);

        let screen_vertex_shader = ScreenVertexShader::new(&deps.ctx.device);

        let renderer = Renderer {
            settings,
            deps,
            depth_texture,
            hdr_msaa_texture,
            hdr_resolve_target,
            main_pass_renderers: TimingQueue::new(),
            post_processing_effects: TimingQueue::new(),
            tone_mapping: None,
            screen_vertex_shader,
        };

        Ok(renderer)
    }

    fn intialize(handle: crate::Handle<Self>) -> anyhow::Result<()> {
        // register resize handler in input
        let mut input = handle.deps.input;
        // Note: Should be registered after the resize event listener of the graphics context, such that the graphics context is already configured to the new size.
        input.register_resize_event_listener(handle, Self::resize, Timing::MIDDLE);

        let mut scheduler = handle.deps.scheduler;
        scheduler.register(
            handle,
            Schedule::Update,
            Timing::RENDER,
            Self::prepare_and_render,
        );
        Ok(())
    }
}

impl Renderer {
    pub fn screen_vertex_shader(&self) -> &ScreenVertexShader {
        &self.screen_vertex_shader
    }

    pub fn register_main_pass_renderer<R: Module + MainPassRenderer>(
        &mut self,
        handle: Handle<R>,
        timing: Timing,
    ) {
        let handle = MainPassRendererHandle::new(handle);
        self.main_pass_renderers.insert(handle, timing); // todo! maybe return key, to deregister later.
    }

    pub fn register_post_processing_effect<R: Module + PostProcessingEffect>(
        &mut self,
        handle: Handle<R>,
        timing: Timing,
    ) {
        let handle = PostProcessingEffectHandle::new(handle);
        self.post_processing_effects.insert(handle, timing);
    }

    pub fn register_tonemapping_effect<R: Module + PostProcessingEffect>(
        &mut self,
        handle: Handle<R>,
    ) {
        let handle = PostProcessingEffectHandle::new(handle);
        if self.tone_mapping.is_some() {
            error!(
                "Setting a tonemapping effect, while another is already set. Other effect is discarded"
            );
        }
        self.tone_mapping = Some(handle);
    }

    fn resize(&mut self, new_size: ResizeEvent) {
        // new_size not used because it is taken from the graphics context, which gets the new screen size before.
        println!("Renderer Resized");
        self.depth_texture.recreate(&self.deps.ctx);
        self.hdr_msaa_texture = HdrTexture::create_screen_sized(&self.deps.ctx, MSAA_SAMPLE_COUNT);
        self.hdr_resolve_target = HdrTexture::create_screen_sized(&self.deps.ctx, 1);
    }

    fn prepare_and_render(&mut self) {
        let ctx = &self.deps.ctx;
        let mut encoder = ctx.new_encoder();

        // /////////////////////////////////////////////////////////////////////////////
        // Prepare
        // /////////////////////////////////////////////////////////////////////////////

        // /////////////////////////////////////////////////////////////////////////////
        // Render
        // /////////////////////////////////////////////////////////////////////////////

        let (surface_texture, view) = ctx.new_surface_texture_and_view();
        // Main Pass Render
        let mut main_pass = self.new_hdr_target_render_pass(&mut encoder);
        for renderer in self.main_pass_renderers.iter() {
            renderer.render(&mut main_pass);
        }
        drop(main_pass);

        // Post processing in Hdr space
        for effect in self.post_processing_effects.iter() {
            effect.apply(
                &mut encoder,
                self.hdr_resolve_target.bind_group(),
                self.hdr_resolve_target.view(),
            )
        }

        // tone mapping from hdr resolve target to the surface view
        if let Some(tone_mapping) = &self.tone_mapping {
            tone_mapping.apply(&mut encoder, self.hdr_resolve_target.bind_group(), &view)
        } else {
            println!("Warning! No Tone Mapping Specified");
        }

        // /////////////////////////////////////////////////////////////////////////////
        // Present
        // /////////////////////////////////////////////////////////////////////////////

        ctx.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
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
}
