use crate::{
    app::{self, ModuleId, UntypedHandle},
    elements::Color,
    utils::{Timing, TimingQueue},
    Dependencies, Handle, Module,
};

use self::screen_texture::{DepthTexture, HdrTexture};

use super::{input::ResizeEvent, GraphicsContext, Input, Schedule, Scheduler};
use log::error;
use vert_macros::Dependencies;
use wgpu::{CommandEncoder, RenderPass};

mod screen_texture;

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
}

impl Module for Renderer {
    type Config = RendererSettings;
    type Dependencies = RendererDependencies;

    fn new(settings: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let depth_texture = DepthTexture::create(&deps.ctx);
        let hdr_msaa_texture = HdrTexture::create_screen_sized(&deps.ctx, 4);
        let hdr_resolve_target = HdrTexture::create_screen_sized(&deps.ctx, 1);

        let renderer = Renderer {
            settings,
            deps,
            depth_texture,
            hdr_msaa_texture,
            hdr_resolve_target,
            main_pass_renderers: TimingQueue::new(),
            post_processing_effects: TimingQueue::new(),
            tone_mapping: None,
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

pub trait MainPassRenderer {
    /// The renderpass here is expected to be 4xMSAA and has HDR_COLOR_FORMAT as its format.
    fn render<'pass, 'encoder>(&'encoder self, render_pass: &'pass mut wgpu::RenderPass<'encoder>);
}

struct MainPassRendererHandle {
    module_id: ModuleId,
    handle: UntypedHandle,
    /// A type punned fn render<'pass, 'encoder>(&'encoder self, render_pass: &'pass mut wgpu::RenderPass<'encoder>);
    render_fn: fn(*const (), render_pass: *const ()) -> (),
}

impl MainPassRendererHandle {
    fn new<R: MainPassRenderer + Module>(handle: Handle<R>) -> Self {
        return MainPassRendererHandle {
            module_id: ModuleId::of::<R>(),
            handle: handle.untyped(),
            render_fn: render::<R>,
        };

        fn render<R: MainPassRenderer>(obj: *const (), render_pass: *const ()) {
            unsafe {
                <R as MainPassRenderer>::render(
                    std::mem::transmute(obj),
                    std::mem::transmute(render_pass),
                );
            }
        }
    }

    fn render<'encoder>(&self, render_pass: &mut wgpu::RenderPass<'encoder>) {
        let obj_ptr = self.handle.ptr();
        let render_pass_ptr = render_pass as *const wgpu::RenderPass<'encoder> as *const ();
        (self.render_fn)(obj_ptr, render_pass_ptr);
    }
}

pub trait PostProcessingEffect {
    fn apply<'e>(
        &'e mut self,
        encoder: &'e mut CommandEncoder,
        input_texture: &wgpu::BindGroup,
        output_texture: &wgpu::TextureView,
    );
}

struct PostProcessingEffectHandle {
    module_id: ModuleId,
    handle: UntypedHandle,
    /// A type punned  fn apply<'e>(&'e mut self, encoder: &'e mut CommandEncoder, input_texture: &wgpu::BindGroup, output_texture: &wgpu::TextureView, );
    apply_fn: fn(
        *const (),
        encoder: *const (),
        input_texture: *const (),
        output_texture: *const (),
    ) -> (),
}

impl PostProcessingEffectHandle {
    fn new<R: PostProcessingEffect + Module>(handle: Handle<R>) -> Self {
        return PostProcessingEffectHandle {
            module_id: ModuleId::of::<R>(),
            handle: handle.untyped(),
            apply_fn: apply::<R>,
        };

        fn apply<R: PostProcessingEffect>(
            obj: *const (),
            encoder: *const (),
            input_texture: *const (),
            output_texture: *const (),
        ) {
            unsafe {
                <R as PostProcessingEffect>::apply(
                    std::mem::transmute(obj),
                    std::mem::transmute(encoder),
                    std::mem::transmute(input_texture),
                    std::mem::transmute(output_texture),
                );
            }
        }
    }

    fn apply<'e>(
        &'e self,
        encoder: &'e mut CommandEncoder,
        input_texture: &wgpu::BindGroup,
        output_texture: &wgpu::TextureView,
    ) {
        let obj_ptr = self.handle.ptr();
        let encoder_ptr = encoder as *const CommandEncoder as *const ();
        let input_texture_ptr = input_texture as *const wgpu::BindGroup as *const ();
        let output_texture_ptr = output_texture as *const wgpu::TextureView as *const ();
        (self.apply_fn)(obj_ptr, encoder_ptr, input_texture_ptr, output_texture_ptr);
    }
}
