use std::time::Duration;

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

    fn new(config: Self::Config, mut deps: Self::Dependencies) -> anyhow::Result<Self> {
        deps.bloom.settings_mut().activated = false;
        deps.ui
            .ui_renderer
            .watch_shader_file("./src/modules/ui/ui.wgsl");

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

        let size = self.deps.ctx.size;
        self.ui.start_frame(
            BoardInput::from_input_module(&self.deps.input),
            dvec2(size.width as f64, size.height as f64),
        );

        self.deps.world_rects.draw_textured_rect(
            UiRect {
                pos: Rect::new(0.0, 0.0, 1024.0, 1024.0),
                uv: Rect::UNIT,
                color: Color::WHITE,
                border_radius: Default::default(),
            },
            Transform::default(),
            self.deps.ui.fonts.atlas_texture(),
        );

        let parent = self
            .ui
            .add_non_text_div(
                DivProps {
                    width: Len::PARENT,
                    height: Len::CHILDREN,
                    axis: Axis::X,
                    main_align: MainAlign::SpaceBetween,
                    cross_align: Align::Center,
                    absolute: false,
                },
                DivStyle {
                    color: Color::RED.alpha(0.2),
                    ..Default::default()
                },
                Id::from("Parent"),
                None,
            )
            .id;

        // let text = self.ui.add_text_div(
        //     DivProps::default(),
        //     DivStyle::default(),
        //     Text {
        //         color: Color::WHITE,
        //         string: "This is all the text that we need".into(),
        //         font: None,
        //         size: FontSize(80),
        //         offset_x: Len::ZERO,
        //         offset_y: Len::ZERO,
        //     },
        //     Id::from("sasass"),
        //     Some(parent),
        // );

        // let text2 = self.ui.add_text_div(
        //     DivProps::default(),
        //     DivStyle::default(),
        //     Text {
        //         color: Color::RED,
        //         string: "This is some other text".into(),
        //         font: None,
        //         size: FontSize(20),
        //         offset_x: Len::ZERO,
        //         offset_y: Len::ZERO,
        //     },
        //     Id::from("asdsadsadsasdsad"),
        //     Some(parent),
        // );

        let purp_parent = self
            .ui
            .add_non_text_div(
                DivProps {
                    width: Len::Px(100.0),
                    height: Len::Px(200.0),
                    axis: Axis::Y,
                    main_align: MainAlign::Center,
                    cross_align: Align::Center,
                    absolute: false,
                },
                DivStyle {
                    color: Color::PURPLE.alpha(0.5),
                    border_radius: BorderRadius::all(20.0),
                    border_color: Color::GREEN,
                    border_thickness: 6.0,
                    border_softness: 1.0,
                    ..Default::default()
                },
                Id::from("Purple Parent"),
                Some(parent),
            )
            .id;

        self.ui.add_non_text_div(
            DivProps {
                width: Len::Px(50.0),
                height: Len::Px(50.0),
                axis: Axis::Y,
                main_align: MainAlign::Center,
                cross_align: Align::Center,
                absolute: false,
            },
            DivStyle {
                color: Color::GREEN,
                ..Default::default()
            },
            Id::from("child 1 in purple"),
            Some(purp_parent),
        );

        self.ui.add_non_text_div(
            DivProps {
                width: Len::Px(50.0),
                height: Len::Px(50.0),
                axis: Axis::Y,
                main_align: MainAlign::Center,
                cross_align: Align::Center,
                absolute: false,
            },
            DivStyle {
                color: Color::WHITE,
                ..Default::default()
            },
            Id::from("child 2 in purple"),
            Some(purp_parent),
        );

        self.ui.add_non_text_div(
            DivProps {
                width: Len::Px(100.0),
                height: Len::Px(20.0),
                axis: Axis::Y,
                main_align: MainAlign::Start,
                cross_align: Align::Start,
                absolute: false,
            },
            DivStyle {
                color: Color::BLACK,
                ..Default::default()
            },
            Id::from("other"),
            Some(parent),
        );

        let mut text_div_comm = self.ui.add_text_div(
            DivProps {
                width: Len::Px(300.0),
                height: Len::Px(400.0),
                axis: Axis::Y,
                main_align: MainAlign::Start,
                cross_align: Align::Start,
                absolute: false,
            },
            DivStyle {
                color: Color::YELLOW,
                border_radius: BorderRadius::all(20.0),
                border_thickness: 20.0,
                ..Default::default()
            },
            Text {
                color: Color::new(6.0, 2.0, 2.0),
                string: "Hover me please, I will show you something!".into(),
                size: FontSize(48),
                offset_x: Len::Px(30.0),
                offset_y: Len::Px(30.0),
                ..Default::default()
            },
            Id::from("text div"),
            Some(parent),
        );

        // can immediately edit the style and text without a 1-frame lag:
        // 1 frame lag only applies to the layout rect (DivProps) itself.
        if text_div_comm.mouse_in_rect() {
            let style = text_div_comm.style();
            style.color = Color::BLUE;
            style.border_color = Color::GREEN;
            style.border_thickness = 6.0;
            // style.border_radius = BorderRadius::new(40.0, 40.0, 40.0, 40.0);
            text_div_comm.text().color = Color::BLACK;
        }

        let total_time = self.deps.time.total().as_secs_f64() * 4.0;
        let total_time2 = self.deps.time.total().as_secs_f64() * 9.7;
        if text_div_comm.mouse_in_rect() {
            self.ui.add_non_text_div(
                DivProps {
                    width: Len::Px(40.0),
                    height: Len::Px(40.0),
                    axis: Axis::Y,
                    main_align: MainAlign::Start,
                    cross_align: Align::Start,
                    absolute: false,
                },
                DivStyle {
                    color: Color::GREEN,
                    offset_x: Len::Px(total_time.sin() * 20.0),
                    offset_y: Len::Px(total_time2.cos() * 20.0),
                    ..Default::default()
                },
                Id::from(2112213232),
                Some(parent),
            );
        }

        let container2 = self
            .ui
            .add_non_text_div(
                DivProps {
                    width: Len::CHILDREN,
                    height: Len::PARENT,
                    axis: Axis::Y,
                    main_align: MainAlign::SpaceAround,
                    cross_align: Align::Center,
                    absolute: false,
                },
                DivStyle::default(),
                Id::from("Container 2"),
                Some(parent),
            )
            .id;

        {
            let clicked = self
                .ui
                .add(
                    Button {
                        text: "Click".into(),
                        ..Default::default()
                    },
                    Id::from("my button"),
                    Some(container2),
                )
                .clicked;
            if clicked {
                println!("Hello 1");
            }
        }
        {
            let clicked = self
                .ui
                .add(
                    Button {
                        text: "Button 2".into(),
                        ..Default::default()
                    },
                    Id::from("my button 2"),
                    Some(container2),
                )
                .clicked;
            if clicked {
                println!("Hello 2");
            }
        }
        {
            let clicked = self
                .ui
                .add(
                    Button {
                        text: "Button 3".into(),
                        ..Default::default()
                    },
                    Id::from("my button 3"),
                    Some(container2),
                )
                .clicked;
            if clicked {
                println!("Hello 3");
            }
        }

        // let mut ctx = self.deps.egui.context();
        // egui_inspect_board(&mut ctx, &mut self.ui);

        self.ui.end_frame(&mut self.deps.ui.fonts);
        self.deps.ui.ui_renderer.draw_billboard(&self.ui);
        // std::thread::sleep(Duration::from_millis(150));
    }
}
