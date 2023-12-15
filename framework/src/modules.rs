use std::{cell::RefCell, net::Shutdown, sync::Arc};

use vert_core::arenas::Arenas;
use wgpu::CommandEncoder;
use winit::{dpi::PhysicalSize, keyboard::KeyCode};

use crate::{
    batteries::{self, Batteries},
    flow::Flow,
    state::StateT,
};

use self::{
    assets::AssetServer,
    egui::EguiState,
    graphics::{
        elements::{
            camera::{CamTransform, Camera},
            screen_space::{self, ScreenSpace},
        },
        graphics_context::{GraphicsContext, GraphicsOwner},
        renderer::Renderer,
        Prepare,
    },
    input::Input,
    time::Time,
    ui::ImmediateUi,
};

pub mod assets;
pub mod egui;
pub mod graphics;
pub mod input;
pub mod time;
pub mod ui;

pub struct Modules {
    pub(crate) arenas: Arenas,
    pub(crate) graphics: GraphicsOwner,
    pub(crate) renderer: Renderer,
    pub(crate) camera: Camera,
    pub(crate) screen_space: ScreenSpace,
    pub(crate) input: Input,
    pub(crate) time: Time,
    pub(crate) egui: EguiState,
    pub(crate) ui: ImmediateUi,
    pub(crate) assets: AssetServer,
    pub(crate) batteries: Option<Batteries>,
    // todo: egui
}

impl Modules {
    pub async fn initialize(window: &winit::window::Window) -> anyhow::Result<Self> {
        let arenas = Arenas::new();
        let graphics_context = GraphicsOwner::intialize(window).await?;

        let camera = Camera::new_default(&graphics_context.context);
        let screen_space = ScreenSpace::new(&graphics_context.context);
        let renderer = Renderer::initialize(
            graphics_context.context.clone(),
            camera.bind_group(),
            screen_space.bind_group(),
        )?;

        let batteries = Batteries::new();

        let input = Input::default();
        let time = Time::default();
        let assets = AssetServer::new();
        let egui = EguiState::new(&graphics_context.context);
        let ui = ImmediateUi::new(graphics_context.context.clone());

        Ok(Self {
            arenas,
            graphics: graphics_context,
            renderer,
            camera,
            screen_space,
            input,
            time,
            egui,
            assets,
            ui,
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
        self.screen_space.resize(new_size.width, new_size.height);
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
        self.screen_space.prepare(queue);
        self.egui.prepare(&self.graphics.context, encoder);
        self.ui.prepare(context, encoder);

        self.batteries.as_mut().unwrap().prepare(queue, encoder);

        // user defined state:
        state.prepare(queue, encoder);

        // collect all the components that need preparation in this command encoder
        for e in self.arenas.iter_component_traits_mut::<dyn Prepare>() {
            e.prepare(context, encoder);
        }
    }

    pub(crate) fn prepare_and_render(&mut self, state: &mut impl StateT) {
        // construct prepare commands (copy stuff to GPU):
        let mut encoder = self.graphics.new_encoder();
        self.prepare_commands(&mut encoder, state);

        // queue up all the render commands:
        let (surface_texture, view) = self.graphics.new_surface_texture_and_view();
        self.renderer
            .render(&view, &mut encoder, &self.arenas, &self.ui);

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
