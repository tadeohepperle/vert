use std::sync::Arc;

use vert_core::arenas::Arenas;
use winit::dpi::PhysicalSize;

use self::{
    graphics::{graphics_context::GraphicsContext, renderer::Renderer},
    input::Input,
    time::Time,
};

pub mod graphics;
pub mod input;
pub mod time;

pub struct Modules {
    arenas: Arenas,
    graphics_context: Arc<GraphicsContext>,
    renderer: Renderer,
    input: Input,
    time: Time,
    // todo: egui
}

impl Modules {
    pub async fn initialize(window: &winit::window::Window) -> anyhow::Result<Self> {
        let arenas = Arenas::new();
        let graphics_context = Arc::new(GraphicsContext::initialize(window).await?);
        let renderer = Renderer::initialize(graphics_context.clone()).await?;

        let input = Input::default();
        let time = Time::default();

        Ok(Self {
            arenas,
            graphics_context,
            renderer,
            input,
            time,
        })
    }

    pub fn receive_window_event(&mut self, window_event: &winit::event::WindowEvent) {
        // todo!()
        // self.egui.receive_window_event(window_event);
        self.input.receive_window_event(window_event);
        if let Some(new_size) = self.input.resized() {
            self.renderer.resize(new_size);
            // if let Screen::Ingame(i) = &mut self.screen {
            //     i.resize(new_size, self.renderer.queue())
            // } // todo!() this was for camera resize!!!
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.graphics_context.resize(new_size);
        self.renderer.resize(new_size);
    }
}
