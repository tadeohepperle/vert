use std::{cell::RefCell, net::Shutdown, sync::Arc};

use vert_core::arenas::Arenas;
use wgpu::CommandEncoder;
use winit::{dpi::PhysicalSize, keyboard::KeyCode};

use crate::{flow::Flow, state::StateT};

use self::{
    egui::EguiState,
    graphics::{
        elements::camera::{CamTransform, Camera},
        graphics_context::{GraphicsContext, GraphicsOwner},
        renderer::Renderer,
        Prepare,
    },
    input::Input,
    time::Time,
};

pub mod egui;
pub mod graphics;
pub mod input;
pub mod modules_ext;
pub mod time;

pub struct Modules {
    pub(crate) arenas: Arenas,
    pub(crate) graphics: GraphicsOwner,
    pub(crate) renderer: Renderer,
    pub(crate) camera: Camera,
    pub(crate) input: Input,
    pub(crate) time: Time,
    pub(crate) egui: EguiState,
    // todo: egui
}

impl Modules {
    pub async fn initialize(window: &winit::window::Window) -> anyhow::Result<Self> {
        let arenas = Arenas::new();
        let graphics_context = GraphicsOwner::intialize(window).await?;

        let camera = Camera::new_default(&graphics_context.context);
        let renderer =
            Renderer::initialize(graphics_context.context.clone(), camera.bind_group()).await?;

        let input = Input::default();
        let time = Time::default();

        let egui = EguiState::new(&graphics_context.context);

        Ok(Self {
            arenas,
            graphics: graphics_context,
            renderer,
            input,
            time,
            camera,
            egui,
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

        Flow::Continue
    }

    /// There are 2 ways to get data updated on the GPU:
    /// - write to the queue directly with `queue.write(...)`
    /// - add commands to a `CommandEncoder` and submit it later to be executed before the render commands.
    fn prepare_commands(&mut self, encoder: &mut wgpu::CommandEncoder, state: &mut impl StateT) {
        let context = &self.graphics.context;
        self.camera.prepare(&context.queue);
        self.egui.prepare(&self.graphics.context, encoder);
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
            .render(&view, &mut encoder, &self.arenas, &self.egui);

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
}
