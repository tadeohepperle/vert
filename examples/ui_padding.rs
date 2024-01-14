

use glam::{dvec2};
use vert::{
    elements::{Color, Transform},
    modules::{
        batteries::{GraphicsSettingsController},
        ui::{
            Align, Board, BoardInput, Len, MainAlign, Padding,
        },
        DefaultDependencies, DefaultModules, Schedule,
    },
    utils::Timing,
    AppBuilder, Module,
};

fn main() {
    let mut app = AppBuilder::new();
    app.add_plugin(DefaultModules);
    app.add::<GraphicsSettingsController>();
    app.add::<MyApp>();
    app.run().unwrap();
}

struct MyApp {
    deps: DefaultDependencies,
    ui: Board,
}

impl Module for MyApp {
    type Config = ();

    type Dependencies = DefaultDependencies;

    fn new(_config: Self::Config, mut deps: Self::Dependencies) -> anyhow::Result<Self> {
        deps.bloom.settings_mut().activated = false;

        Ok(MyApp {
            deps,
            ui: Board::new(dvec2(800.0, 800.0)),
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
        self.ui.start_frame(
            BoardInput::from_input_module(&self.deps.input),
            self.deps.ctx.size_dvec2(),
        );

        let mut parent = self.ui.add_div("Parent", None);
        parent.width(Len::PARENT);
        parent.height(Len::PARENT);
        parent.main_align = MainAlign::Center;
        parent.cross_align = Align::Center;
        parent.color = Color::RED;
        let parent = Some(parent.id);

        let mut rect = self.ui.add_div("rect", parent);
        rect.width(Len::px(400.0));
        rect.height(Len::px(400.0));
        rect.padding = Padding::new()
            .left(Len::px(50.0))
            .top(Len::px(50.0))
            .right(Len::parent(0.5));
        rect.color = Color::BLACK;
        rect.main_align = MainAlign::End;
        let rect = Some(rect.id);

        let mut inner = self.ui.add_div("inner", rect);
        inner.width(Len::PARENT);
        inner.height(Len::px(200.0));
        inner.color = Color::WHITE;

        self.ui.end_frame(&mut self.deps.ui.fonts);
        self.deps.ui.ui_renderer.draw_ui_board(&self.ui);
    }
}
