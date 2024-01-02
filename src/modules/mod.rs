use crate::{Dependencies, Handle, Plugin};
pub mod renderer;
pub use renderer::{
    AcesToneMapping, Attribute, Bloom, BloomSettings, MainPassRenderer, PostProcessingEffect,
    Prepare, Renderer, ScreenVertexShader, ToneMappingSettings, VertexT,
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

pub mod egui;
pub use egui::Egui;

use self::renderer::main_pass_renderer::{
    ColorMeshRenderer, Gizmos, TextRenderer, UiRectRenderer, WorldRectRenderer,
};

pub mod batteries;

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
        app.add_with_config::<Bloom>(BloomSettings::default());
        app.add_with_config::<AcesToneMapping>(ToneMappingSettings::Aces);
        app.add::<Gizmos>();
        app.add::<ColorMeshRenderer>();
        app.add::<Egui>();
        app.add::<UiRectRenderer>();
        app.add::<WorldRectRenderer>();
        app.add::<TextRenderer>();
    }
}

#[derive(Debug, Dependencies)]
pub struct DefaultDependencies {
    pub winit: Handle<WinitMain>,
    pub tokio: Handle<TokioRuntime>,
    pub ctx: Handle<GraphicsContext>,
    pub scheduler: Handle<Scheduler>,
    pub input: Handle<Input>,
    pub time: Handle<Time>,
    pub renderer: Handle<Renderer>,
    pub arenas: Handle<Arenas>,
    pub camera_3d: Handle<MainCamera3D>,
    pub screen_size: Handle<MainScreenSize>,
    pub bloom: Handle<Bloom>,
    pub tone_mapping: Handle<AcesToneMapping>,
    pub egui: Handle<Egui>,
    pub gizmos: Handle<Gizmos>,
    pub color_mesh: Handle<ColorMeshRenderer>,
    pub ui_rects: Handle<UiRectRenderer>,
    pub world_rects: Handle<WorldRectRenderer>,
    pub text: Handle<TextRenderer>,
}
