use glam::{vec2, vec3, Vec2};
use morphorm::Units;
use vert::{
    elements::{Color, Rect, Transform},
    modules::{
        arenas::Key,
        batteries::{FlyCam, GraphicsSettingsController},
        renderer::main_pass_renderer::{text_renderer::DrawText, ui_rect::UiRect},
        ui::{
            billboard::{Billboard, BillboardInput, DivId, DivProps, DivText},
            font_cache::RasterizedFont,
        },
        DefaultDependencies, DefaultModules, MainPassRenderer, Schedule,
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
    ui: Billboard,
    font_key: Key<RasterizedFont>,
}

impl Module for MyApp {
    type Config = ();

    type Dependencies = DefaultDependencies;

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let mut fonts = deps.ui.fonts;
        let font_key = fonts.rasterize_default_font(50.0).unwrap();

        Ok(MyApp {
            deps,
            ui: Billboard::new(600.0, 600.0, morphorm::LayoutType::Row),
            font_key,
        })
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

        self.ui
            .start_frame(BillboardInput::from_input_module(&self.deps.input));
        self.ui.add_div(
            // Some(DivText {
            //     string: "Hello World".into(),
            //     font: self.font_key,
            // }),
            None,
            DivProps {
                width: Some(Units::Pixels(300.0)),
                height: Some(Units::Pixels(200.0)),
                color: Color::RED,
                ..Default::default()
            },
            DivId::from(11),
            None,
            0,
        );

        self.ui.end_frame(&self.deps.ui.fonts);
        self.deps.ui.ui_renderer.draw_billboard(&self.ui);
    }
}
