use glam::{dvec2, vec2, vec3, Vec2};
use vert::{
    elements::{Color, Rect, Transform},
    modules::{
        arenas::Key,
        batteries::{FlyCam, GraphicsSettingsController},
        renderer::main_pass_renderer::{text_renderer::DrawText, ui_rect::UiRect},
        ui::{
            board::{
                egui_inspect_board, Align, Axis, Board, BoardInput, BorderRadius, DivProps,
                DivStyle, Id, Len, MainAlign, Text,
            },
            font_cache::FontSize,
            widgets::Button,
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
    // app.add::<GraphicsSettingsController>();
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

    fn new(config: Self::Config, mut deps: Self::Dependencies) -> anyhow::Result<Self> {
        deps.bloom.settings_mut().activated = false;
        deps.ui
            .ui_renderer
            .watch_rect_shader_file("./src/modules/ui/rect.wgsl");

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

        self.ui
            .start_frame(BoardInput::from_input_module(&self.deps.input));
        let parent = self
            .ui
            .add_non_text_div(
                DivProps {
                    width: Len::Px(700.0),
                    height: Len::ChildrenFraction(1.5),
                    axis: Axis::X,
                    main_align: MainAlign::SpaceBetween,
                    cross_align: Align::Center,
                },
                DivStyle {
                    color: Color::RED,
                    ..Default::default()
                },
                Id::from("Parent"),
                None,
            )
            .id;

        // let purp_parent = self
        //     .ui
        //     .add_non_text_div(
        //         DivProps {
        //             width: Len::Px(100.0),
        //             height: Len::Px(200.0),
        //             axis: Axis::Y,
        //             main_align: MainAlign::Center,
        //             cross_align: Align::Center,
        //         },
        //         DivStyle {
        //             color: Color::PURPLE.alpha(0.5),
        //             border_radius: BorderRadius::all(20.0),
        //             border_color: Color::GREEN,
        //             border_thickness: 6.0,
        //             border_softness: 1.0,
        //             ..Default::default()
        //         },
        //         Id::from("Purple Parent"),
        //         Some(parent),
        //     )
        //     .id;

        // self.ui.add_non_text_div(
        //     DivProps {
        //         width: Len::Px(50.0),
        //         height: Len::Px(50.0),
        //         axis: Axis::Y,
        //         main_align: MainAlign::Center,
        //         cross_align: Align::Center,
        //     },
        //     DivStyle {
        //         color: Color::GREEN,
        //         ..Default::default()
        //     },
        //     Id::from("child 1 in purple"),
        //     Some(purp_parent),
        // );

        // self.ui.add_non_text_div(
        //     DivProps {
        //         width: Len::Px(50.0),
        //         height: Len::Px(50.0),
        //         axis: Axis::Y,
        //         main_align: MainAlign::Center,
        //         cross_align: Align::Center,
        //     },
        //     DivStyle {
        //         color: Color::WHITE,
        //         ..Default::default()
        //     },
        //     Id::from("child 2 in purple"),
        //     Some(purp_parent),
        // );

        // self.ui.add_non_text_div(
        //     DivProps {
        //         width: Len::Px(100.0),
        //         height: Len::Px(20.0),
        //         axis: Axis::Y,
        //         main_align: MainAlign::Start,
        //         cross_align: Align::Start,
        //     },
        //     DivStyle {
        //         color: Color::BLACK,
        //         ..Default::default()
        //     },
        //     Id::from("other"),
        //     Some(parent),
        // );

        // let mut text_div_comm = self.ui.add_text_div(
        //     DivProps {
        //         width: Len::Px(300.0),
        //         height: Len::Px(400.0),
        //         axis: Axis::Y,
        //         main_align: MainAlign::Start,
        //         cross_align: Align::Start,
        //     },
        //     DivStyle {
        //         color: Color::YELLOW,
        //         border_radius: BorderRadius::all(20.0),
        //         border_thickness: 20.0,
        //         ..Default::default()
        //     },
        //     Text {
        //         color: Color::new(6.0, 2.0, 2.0),
        //         string: "Hover me please, I will show you something!".into(),
        //         size: FontSize(48),
        //         offset_x: Len::Px(30.0),
        //         offset_y: Len::Px(30.0),
        //         ..Default::default()
        //     },
        //     Id::from("text div"),
        //     Some(parent),
        // );

        // // can immediately edit the style and text without a 1-frame lag:
        // // 1 frame lag only applies to the layout rect (DivProps) itself.
        // if text_div_comm.is_hovered() {
        //     let style = text_div_comm.style_mut();
        //     style.color = Color::BLUE;
        //     style.border_color = Color::GREEN;
        //     style.border_thickness = 6.0;
        //     // style.border_radius = BorderRadius::new(40.0, 40.0, 40.0, 40.0);
        //     text_div_comm.text_mut().color = Color::BLACK;
        // }

        // let total_time = self.deps.time.total().as_secs_f64() * 4.0;
        // let total_time2 = self.deps.time.total().as_secs_f64() * 9.7;
        // if text_div_comm.is_hovered() {
        //     self.ui.add_non_text_div(
        //         DivProps {
        //             width: Len::Px(40.0),
        //             height: Len::Px(40.0),
        //             axis: Axis::Y,
        //             main_align: MainAlign::Start,
        //             cross_align: Align::Start,
        //         },
        //         DivStyle {
        //             color: Color::GREEN,
        //             offset_x: Len::Px(total_time.sin() * 20.0),
        //             offset_y: Len::Px(total_time2.cos() * 20.0),
        //             ..Default::default()
        //         },
        //         Id::from(2112213232),
        //         Some(parent),
        //     );
        // }

        self.ui.add(
            Button {
                text: "Hello Click".into(),
                text_color: Color::BLACK,
                color: Color::BLACK.alpha(0.5),
                hover_color: Color::LIGHTGREY,
                font: None,
            },
            Id::from("my button"),
            Some(parent),
        );
        // let mut ctx = self.deps.egui.context();
        // egui_inspect_board(&mut ctx, &mut self.ui);

        self.ui.end_frame(&mut self.deps.ui.fonts);
        self.deps.ui.ui_renderer.draw_billboard(&self.ui);
    }
}
