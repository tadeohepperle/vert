pub mod renderer;
use std::sync::Arc;

pub use renderer::{AcesToneMapping, Attribute, Bloom, BloomSettings, VertexT};

use winit::{event::WindowEvent, window::Window};

pub mod graphics_context;
pub use graphics_context::{GraphicsContext, GraphicsContextConfig};

pub mod input;
pub use input::Input;

pub mod time;
pub use time::Time;

pub mod arenas;

pub mod egui;
pub use egui::Egui;

pub mod ui;

use crate::{
    elements::{camera3d::Camera3dGR, Camera3d, Color, Screen, ScreenGR},
    App, Prepare, ReceiveWindowEvent, Resize, UpdateFlow,
};

use self::{
    renderer::{
        ColorMeshRenderer, Gizmos, ScreenTextures, TextRenderer, UiRectRenderer, WorldRectRenderer,
    },
    ui::{FontCache, UiRenderer},
};

pub struct DefaultModules {
    pub tokio: tokio::runtime::Runtime,
    pub ctx: GraphicsContext,
    pub window: Arc<Window>,
    pub input: Input,
    pub time: Time,

    pub screen: Screen,
    pub screen_gr: ScreenGR,
    pub camera: Camera3d,
    pub camera_gr: Camera3dGR,

    pub egui: Egui,

    pub screen_textures: ScreenTextures,
    pub color_mesh: ColorMeshRenderer,
    pub gizmos: Gizmos,

    pub ui_rect: UiRectRenderer,
    pub world_rect: WorldRectRenderer,

    #[warn(deprecated)]
    /// deprecated, because uses its own font atlas and it a bit clumsy.
    pub text: TextRenderer,

    pub fonts: FontCache,
    pub ui: UiRenderer,

    pub bloom: Bloom,
    pub tone_mapping: AcesToneMapping,
}

impl DefaultModules {
    pub fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let tokio = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let ctx = GraphicsContext::new(GraphicsContextConfig::default(), &tokio, &window)?;
        let input = Input::new();
        let time = Time::new();

        let screen = Screen::from_window(&window);
        let screen_gr = ScreenGR::new(&ctx, &screen);
        let camera = Camera3d::new(ctx.size.width, ctx.size.height);
        let camera_gr = Camera3dGR::new(&ctx, &camera);

        let egui = Egui::new(&ctx, &window);

        let screen_textures = ScreenTextures::new(&ctx);
        let color_mesh = ColorMeshRenderer::new(&ctx, &camera_gr);
        let gizmos = Gizmos::new(&ctx, &camera_gr);
        let ui_rect = UiRectRenderer::new(&ctx, &screen_gr);
        let world_rect = WorldRectRenderer::new(&ctx, &camera_gr);
        let text = TextRenderer::new(&ctx);
        let fonts = FontCache::new(&ctx);
        let ui = UiRenderer::new(&ctx, &screen_gr);
        let bloom = Bloom::new(&ctx, &screen_textures.screen_vertex_shader, &screen_gr);
        let tone_mapping = AcesToneMapping::new(&ctx, &screen_textures.screen_vertex_shader);

        Ok(DefaultModules {
            tokio,
            ctx,
            window,
            input,
            time,
            screen,
            screen_gr,
            camera,
            camera_gr,
            egui,
            screen_textures,
            gizmos,
            color_mesh,
            ui_rect,
            world_rect,
            text,
            fonts,
            ui,
            bloom,
            tone_mapping,
        })
    }

    pub fn begin_frame(&mut self) -> UpdateFlow {
        self.time.update();
        self.egui.begin_frame();

        if self.input.close_requested() {
            return UpdateFlow::Exit("Close Requested".into());
        }
        if let Some(resized) = self.input.resized() {
            self.ctx.resize(resized);
            self.camera.resize(resized);
            self.screen_textures.resize(&self.ctx);
            self.screen.resize(resized);
            self.bloom.resize(resized);
        }

        UpdateFlow::Continue
    }

    pub fn prepare_and_render(&mut self, clear_color: Color) {
        let mut encoder = self.ctx.new_encoder();
        self.prepare(&mut encoder);

        let (surface_texture, surface_view) = self.ctx.new_surface_texture_and_view();

        // Main Pass Render
        let mut render_pass = self
            .screen_textures
            .new_hdr_target_render_pass(&mut encoder, clear_color);
        self.color_mesh.render(&mut render_pass, &self.camera_gr);
        self.world_rect.render(&mut render_pass, &self.camera_gr);
        self.ui_rect.render(&mut render_pass, &self.screen_gr);
        self.gizmos.render(&mut render_pass, &self.camera_gr);

        drop(render_pass);

        // Post processing in Hdr space
        self.bloom.apply(
            &mut encoder,
            self.screen_textures.hdr_resolve_target.bind_group(),
            self.screen_textures.hdr_resolve_target.view(),
            &self.screen_gr,
        );
        // Tone mapping
        self.tone_mapping.apply(
            &mut encoder,
            self.screen_textures.hdr_resolve_target.bind_group(),
            &surface_view,
        );
        self.ui
            .render(&mut encoder, &surface_view, &self.screen_gr, &self.fonts);
        self.egui.render(&mut encoder, &surface_view);

        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    pub fn prepare(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let device = &self.ctx.device;
        let queue = &self.ctx.queue;

        self.egui.prepare(device, queue, encoder);

        self.camera_gr.prepare(queue, &self.camera);
        self.screen_gr.prepare(queue, &self.screen);

        self.color_mesh.prepare(device, queue, encoder);
        self.gizmos.prepare(device, queue, encoder);
        self.text.prepare(queue);
        self.ui_rect.prepare(device, queue, encoder);
        self.world_rect.prepare(device, queue, encoder);
        self.ui.prepare(device, queue, encoder);
        self.fonts.prepare(queue);
    }

    pub fn end_frame(&mut self) {
        self.input.end_frame();
    }

    pub fn receive_window_event(&mut self, event: &WindowEvent) {
        self.input.receive_window_event(event);
        self.egui.receive_window_event(event);
    }
}
