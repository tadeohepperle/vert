use crate::{app::ModuleId, elements::Camera3D, Dependencies, Handle, Plugin};
use winit::{dpi::PhysicalSize, event::WindowEvent};

mod renderer;
pub use renderer::{
    AcesToneMapping, MainPassRenderer, PostProcessingEffect, Prepare, Renderer, ScreenVertexShader,
    ToneMappingSettings,
};

pub mod winit_main;
pub use winit_main::WinitMain;

pub mod graphics_context;
pub use graphics_context::GraphicsContext;

pub mod tokio_runtime;
pub use tokio_runtime::TokioRuntime;

pub mod input;
pub use input::Input;

pub mod scheduler;
pub use scheduler::{Schedule, Scheduler};

pub mod time;
pub use time::Time;

pub mod camera;
pub use camera::MainCamera3D;

pub mod screen_size;
pub use screen_size::{MainScreenSize, ScreenSize};

pub mod arenas;
pub use arenas::Arenas;

pub struct DefaultModules;

impl Plugin for DefaultModules {
    fn add(&self, app: &mut crate::AppBuilder) {
        app.add_main_module::<WinitMain>();
        app.add::<TokioRuntime>();
        app.add::<GraphicsContext>();
        app.add::<Scheduler>();
        app.add::<Input>();
        app.add::<Time>();
        app.add::<Renderer>();
        app.add::<Arenas>();
        app.add::<MainCamera3D>();
        app.add::<MainScreenSize>();
        app.add_with_config::<AcesToneMapping>(ToneMappingSettings::Aces);
    }
}

#[derive(Debug, Dependencies)]
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
