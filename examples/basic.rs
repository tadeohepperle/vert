use glam::{vec2, vec3, Vec2};
use vert::{
    elements::{Color, Rect, Transform},
    modules::{
        batteries::{FlyCam, GraphicsSettingsController},
        renderer::main_pass_renderer::{text_renderer::DrawText, ui_rect::UiRect},
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

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
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
        if self
            .deps
            .input
            .keys()
            .just_pressed(winit::keyboard::KeyCode::Space)
        {
            let fps = self.deps.time.fps();
            println!("Fps: {fps}");
        }

        self.deps.gizmos.draw_xyz();
        self.deps
            .color_mesh
            .draw_cubes(&[Transform::new(1.0, 1.0, 1.0)], None);

        self.deps.ui_rects.draw_rect(UiRect {
            pos: Rect::new(100.0, 100.0, 200.0, 50.0),
            uv: Rect::unit(),
            color: Color::RED,
            border_radius: [20.0, 0.0, 20.0, 0.0],
        });

        self.deps.text.draw_ui_text(DrawText {
            text: "Hello".to_string(),
            pos: vec2(20.0, 200.0),
            font_texture_size: 70.0,
            max_width: None,
            color: Color::GREEN,
            font_layout_size: 70.0,
        });

        self.deps.text.draw_world_text(
            DrawText {
                text: "Hello".to_string(),
                pos: vec2(20.0, 200.0),
                font_texture_size: 256.0,
                max_width: None,
                color: Color::new(5.0, 2.0, 2.0),
                font_layout_size: 128.0,
            },
            Transform::new(5.0, 0.0, 0.0).face_minus_z(),
        );

        if self
            .deps
            .input
            .keys()
            .just_pressed(winit::keyboard::KeyCode::Escape)
        {
            self.deps.scheduler.request_exit("Escape");
        }

        let egui = self.deps.egui;
        let mut egui_ctx = egui.context();
        egui::Window::new("Hellow World").show(&mut egui_ctx, |ui| {
            ui.label("Wow!");
        });
    }
}
