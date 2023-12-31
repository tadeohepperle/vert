pub mod winit_main;
use winit::{dpi::PhysicalSize, event::WindowEvent};
pub use winit_main::WinitMain;

pub mod graphics_context;
pub use graphics_context::GraphicsContext;

pub mod tokio_runtime;
pub use tokio_runtime::TokioRuntime;

pub mod input;
pub use input::Input;

pub mod scheduler;
pub use scheduler::{Schedule, Scheduler, Timing};

use crate::Plugin;

pub trait Resize {
    fn resize(&mut self, new_size: PhysicalSize<u64>);
}

pub trait WinitWindowEventReceiver {
    fn receive_window_event(&mut self, window_event: &WindowEvent);
}

pub trait Runnable {
    fn run(&mut self);
}

pub struct DefaultModules;

impl Plugin for DefaultModules {
    fn add(&self, app: &mut crate::AppBuilder) {
        app.add_main_module::<WinitMain>();
        app.add::<TokioRuntime>();
        app.add::<GraphicsContext>();
        app.add::<Scheduler>();
        app.add::<Input>();
    }
}
