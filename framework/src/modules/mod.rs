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

pub mod time;
pub use time::Time;

use crate::{app::ModuleId, Dependencies, Handle, Plugin};

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
        app.add::<Time>();
    }
}

pub struct DefaultDependencies {
    pub tokio: Handle<TokioRuntime>,
    pub input: Handle<Input>,
    pub time: Handle<Time>,
    pub winit: Handle<WinitMain>,
    pub graphics: Handle<GraphicsContext>,
    pub scheduler: Handle<Scheduler>,
    // tokio: Handle<TokioRuntime>,
    // tokio: Handle<TokioRuntime>,
    // tokio: Handle<TokioRuntime>,
}

impl Dependencies for DefaultDependencies {
    fn type_ids() -> Vec<ModuleId> {
        vec![
            ModuleId::of::<TokioRuntime>(),
            ModuleId::of::<Input>(),
            ModuleId::of::<Time>(),
            ModuleId::of::<WinitMain>(),
            ModuleId::of::<GraphicsContext>(),
            ModuleId::of::<Scheduler>(),
        ]
    }

    fn from_untyped_handles(ptrs: &[crate::app::UntypedHandle]) -> Self {
        DefaultDependencies {
            tokio: ptrs[0].typed(),
            input: ptrs[1].typed(),
            time: ptrs[2].typed(),
            winit: ptrs[3].typed(),
            graphics: ptrs[4].typed(),
            scheduler: ptrs[5].typed(),
        }
    }
}
