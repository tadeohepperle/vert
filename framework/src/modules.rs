use std::sync::Arc;

use vert_core::arenas::Arenas;
use winit::dpi::PhysicalSize;

use crate::{flow::Flow, state::StateT};

use self::{
    egui::EguiState,
    graphics::{
        elements::camera::{CamTransform, Camera},
        graphics_context::{GraphicsContext, GraphicsContextUpdater},
        renderer::Renderer,
        Prepare,
    },
    input::Input,
    time::Time,
};

pub mod egui;
pub mod graphics;
pub mod input;
pub mod time;

pub struct Modules {
    arenas: Arenas,
    graphics_context: GraphicsContextUpdater,
    renderer: Renderer,
    camera: Camera,
    input: Input,
    time: Time,
    egui: EguiState,
    // todo: egui
}

impl Modules {
    pub async fn initialize(window: &winit::window::Window) -> anyhow::Result<Self> {
        let arenas = Arenas::new();
        let graphics_context = GraphicsContextUpdater::intialize(window).await?;
        let renderer = Renderer::initialize(graphics_context.context.clone()).await?;

        let input = Input::default();
        let time = Time::default();

        let camera = Camera::new_default(&graphics_context.context);
        let egui = EguiState::new(&graphics_context.context);

        Ok(Self {
            arenas,
            graphics_context,
            renderer,
            input,
            time,
            camera,
            egui,
        })
    }

    pub fn receive_window_event(&mut self, window_event: &winit::event::WindowEvent) {
        // todo!()
        // self.egui.receive_window_event(window_event);
        self.input.receive_window_event(window_event);
        if let Some(new_size) = self.input.resized() {
            self.resize(new_size);
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.graphics_context.resize(new_size); // needs to be before renderer resize
        self.renderer.resize();
        self.camera.resize(new_size.width, new_size.height);
    }

    pub fn begin_frame(&mut self) -> Flow {
        self.time.update();
        self.egui.begin_frame(self.time.total_secs_f64());
        self.time.egui_time_stats(self.egui.context());
        Flow::Continue
    }

    pub fn prepare(&mut self) -> Flow {
        Flow::Continue
    }

    pub fn prepare_and_render(&mut self, state: &mut impl StateT) -> Flow {
        // create the renderpipeline:

        // self.egui.prepare(&self.graphics_context.context);
        Flow::Continue
    }

    pub fn end_frame(&mut self) -> Flow {
        Flow::Continue
    }
}
