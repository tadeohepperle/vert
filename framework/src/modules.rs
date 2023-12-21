use std::{
    cell::{RefCell, UnsafeCell},
    net::Shutdown,
    sync::Arc,
};

use log::info;
use wgpu::CommandEncoder;
use winit::{dpi::PhysicalSize, keyboard::KeyCode};

use crate::{
    batteries::{self, Batteries},
    flow::Flow,
    modules::graphics::shader::{
        gizmos::Gizmos, text::TextRenderer, ui_rect::UiRectRenderer, world_rect::WorldRectRenderer,
    },
    state::StateT,
};

use self::{
    assets::{asset_store::AssetStore, fetchable_asset::AssetSource},
    // ui::ImmediateUi, // needle
    egui::EguiState,
    graphics::{
        graphics_context::{GraphicsContext, GraphicsContextOwner},
        renderer::Renderer,
        settings::GraphicsSettings,
        shader::color_mesh::ColorMeshRenderer,
        statics::{
            camera::Camera, screen_size::ScreenSize, static_texture::initialize_static_textures,
        },
    },
    input::Input,
    time::Time,
};

pub mod assets;
pub mod egui;
pub mod graphics;
pub mod input;
pub mod time;

// pub mod ui;

pub struct Modules {
    pub(crate) graphics: GraphicsContextOwner,
    pub(crate) renderer: Renderer,
    pub(crate) camera: Camera,
    pub(crate) screen_size: ScreenSize,
    pub(crate) input: Input,
    pub(crate) time: Time,
    pub(crate) egui: EguiState,
    // pub(crate) ui: ImmediateUi, // needle
    pub(crate) batteries: Option<Batteries>,
    // todo: egui
}

impl Modules {
    pub async fn initialize(window: &winit::window::Window) -> anyhow::Result<Self> {
        pretty_env_logger::try_init().unwrap();
        info!("such information");
        let graphics_context = GraphicsContextOwner::intialize(window).await?;

        initialize_static_textures(&graphics_context.context);
        let camera = Camera::new_default(&graphics_context.context);
        let screen_size = ScreenSize::new(&graphics_context.context);

        let graphics_settings = GraphicsSettings::default();
        let mut renderer =
            Renderer::initialize(graphics_context.context.clone(), graphics_settings)?;
        renderer.register_renderer::<ColorMeshRenderer>();
        renderer.register_renderer::<UiRectRenderer>();
        renderer.register_renderer::<WorldRectRenderer>();
        renderer.register_renderer::<TextRenderer>();
        renderer.register_renderer::<Gizmos>();

        let batteries = Batteries::new();

        let input = Input::default();
        let time = Time::default();
        let egui = EguiState::new(&graphics_context.context);

        // let ui = ImmediateUi::new(graphics_context.context.clone()); // needle

        Ok(Self {
            graphics: graphics_context,
            renderer,
            camera,
            screen_size,
            input,
            time,
            egui,
            batteries: Some(batteries),
        })
    }

    pub(crate) fn receive_window_event(&mut self, window_event: &winit::event::WindowEvent) {
        self.egui.receive_window_event(window_event);
        self.input.receive_window_event(window_event);
        if let Some(new_size) = self.input.resized() {
            self.resize(new_size);
        }
    }

    pub(crate) fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.graphics.resize(new_size); // needs to be before renderer resize
        self.renderer.resize();
        self.camera.resize(new_size.width, new_size.height);
        self.screen_size.resize(new_size.width, new_size.height);
    }

    pub(crate) fn begin_frame(&mut self) -> Flow {
        self.time.update();
        self.egui.begin_frame(self.time.total_secs_f64());
        self.time.egui_time_stats(self.egui.context());

        if self.input.keys().just_pressed(KeyCode::Escape) || self.input.close_requested() {
            return Flow::Exit;
        }

        if self.input.keys().just_pressed(KeyCode::KeyT) {
            dbg!(self.time.fps());
        }

        // take out of state and put back in later bc of ownership issues.
        let mut batteries = self.batteries.take().unwrap();
        batteries.update(self);
        self.batteries = Some(batteries);

        Flow::Continue
    }

    /// There are 2 ways to get data updated on the GPU:
    /// - write to the queue directly with `queue.write(...)`
    /// - add commands to a `CommandEncoder` and submit it later to be executed before the render commands.
    fn prepare_commands(&mut self, encoder: &mut wgpu::CommandEncoder, state: &mut impl StateT) {
        let context = &self.graphics.context;
        let queue: &wgpu::Queue = &context.queue;
        self.camera.prepare(queue);
        self.screen_size.prepare(queue);
        self.egui.prepare(&self.graphics.context, encoder);
        // self.ui.prepare(context, encoder); // needle

        self.batteries.as_mut().unwrap().prepare(queue, encoder);

        // user defined state:
        state.prepare(queue, encoder);

        // prepare renderer: (gizmos) todo!() probably not the right position here
        self.renderer.prepare(encoder);
    }

    pub(crate) fn prepare_and_render(&mut self, state: &mut impl StateT) {
        // construct prepare commands (copy stuff to GPU):
        let mut encoder = self.graphics.new_encoder();
        self.prepare_commands(&mut encoder, state);

        // queue up all the render commands:
        let (surface_texture, view) = self.graphics.new_surface_texture_and_view();
        let assets = AssetStore::lock();
        self.renderer.render(&view, &mut encoder, &assets);

        // render egui: (egui does its own render pass, does not need msaa and other stuff)
        self.egui.render(&mut encoder, &view);

        // execute render commands and present:
        self.graphics
            .context
            .queue
            .submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    pub fn end_frame(&mut self) -> Flow {
        self.input.clear_at_end_of_frame();
        Flow::Continue
    }

    pub fn graphics_context(&self) -> &GraphicsContext {
        &self.graphics.context
    }
}
