use glam::Vec2;
use vert::{
    elements::{Color, Transform},
    modules::{
        batteries::{FlyCam, GraphicsSettingsController},
        renderer::main_pass_renderer::text_renderer::DrawText,
        DefaultDependencies, DefaultModules, Schedule,
    },
    utils::Timing,
    AppBuilder, Module,
};

fn main() {
    let mut app = AppBuilder::new();
    app.add_plugin(DefaultModules);
    app.add::<FlyCam>();
    app.add::<GraphicsSettingsController>();
    app.add::<MyApp>();
    app.run().unwrap();
}

struct MyApp {
    deps: DefaultDependencies,
}

impl Module for MyApp {
    type Config = ();

    type Dependencies = DefaultDependencies;

    fn new(_config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        Ok(MyApp { deps })
    }

    fn intialize(handle: vert::Handle<Self>) -> anyhow::Result<()> {
        let scheduler = handle.deps.scheduler.get_mut();
        scheduler.register(handle, Schedule::Update, Timing::DEFAULT, Self::update);
        Ok(())
    }
}

impl MyApp {
    fn update(&mut self) {
        self.deps.gizmos.draw_xyz();
        self.deps
            .color_mesh
            .draw_cubes(&[Transform::new(1.0, 1.0, 1.0)], None);

        self.deps.text.draw_world_text(
            DrawText {
                text: "Hello".into(),
                pos: Vec2::ZERO,
                font_texture_size: 60.0,
                font_layout_size: 60.0,
                max_width: None,
                color: Color::BLUE,
            },
            Transform::default().face_minus_z(),
        )
    }
}
