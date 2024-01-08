use glam::{dvec2, vec2, vec3, Vec2};
use vert::{
    elements::{Color, Rect, Transform},
    modules::{
        arenas::Key,
        batteries::{FlyCam, GraphicsSettingsController},
        renderer::main_pass_renderer::{text_renderer::DrawText, ui_rect::UiRect},
        ui::{
            board::{
                Axis, Board, BoardInput, CrossAlign, DivId, DivProps, DivStyle, MainAlign, Size,
                Text,
            },
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
    ui: Board,
    font_key: Key<RasterizedFont>,
}

impl Module for MyApp {
    type Config = ();

    type Dependencies = DefaultDependencies;

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let mut fonts = deps.ui.fonts;
        let font_key = fonts.rasterize_default_font(40.0).unwrap();

        Ok(MyApp {
            deps,
            ui: Board::new(dvec2(800.0, 800.0)),
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
            .start_frame(BoardInput::from_input_module(&self.deps.input));
        let (d1, _) = self.ui.add_non_text_div(
            DivProps {
                width: Size::Px(700.0),
                height: Size::Px(700.0),
                axis: Axis::X,
                main_align: MainAlign::Start,
                cross_align: CrossAlign::Start,
            },
            DivStyle {
                color: Color::RED,
                z_bias: 0,
            },
            DivId::from(11),
            None,
        );

        self.ui.add_non_text_div(
            DivProps {
                width: Size::Px(100.0),
                height: Size::Px(50.0),
                axis: Axis::Y,
                main_align: MainAlign::Start,
                cross_align: CrossAlign::Start,
            },
            DivStyle {
                color: Color::BLUE,
                z_bias: 0,
            },
            DivId::from(12),
            Some(d1),
        );

        self.ui.add_non_text_div(
            DivProps {
                width: Size::Px(100.0),
                height: Size::Px(20.0),
                axis: Axis::Y,
                main_align: MainAlign::Start,
                cross_align: CrossAlign::Start,
            },
            DivStyle {
                color: Color::BLACK,
                z_bias: 0,
            },
            DivId::from(13),
            Some(d1),
        );

        let text_div_comm = self.ui.add_text_div(
            DivProps {
                width: Size::Px(300.0),
                height: Size::Px(300.0),
                axis: Axis::Y,
                main_align: MainAlign::Start,
                cross_align: CrossAlign::Start,
            },
            DivStyle {
                color: Color::BLACK,
                z_bias: 0,
            },
            Text {
                color: Color::new(6.0, 2.0, 2.0),
                string: "Hello World I really like it here!".into(),
                font: self.font_key,
            },
            DivId::from(2772),
            Some(d1),
        );

        if let Some(comm) = text_div_comm {
            if comm.hovered {
                self.ui.add_non_text_div(
                    DivProps {
                        width: Size::Px(40.0),
                        height: Size::Px(40.0),
                        axis: Axis::Y,
                        main_align: MainAlign::Start,
                        cross_align: CrossAlign::Start,
                    },
                    DivStyle {
                        color: Color::GREEN,
                        z_bias: 0,
                    },
                    DivId::from(2112213232),
                    Some(d1),
                );
            }
        }

        self.ui.end_frame(&self.deps.ui.fonts);
        self.deps.ui.ui_renderer.draw_billboard(&self.ui);
    }
}
