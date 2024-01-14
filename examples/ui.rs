use std::time::Duration;

use glam::{dvec2, vec2, vec3, Vec2};
use vert::{
    elements::{Color, Rect, Transform},
    modules::{
        arenas::Key,
        batteries::{FlyCam, GraphicsSettingsController},
        renderer::main_pass_renderer::{text_renderer::DrawText, ui_rect::UiRect},
        ui::{
            font_cache::FontSize, Align, Axis, Board, BoardInput, BorderRadius, Button, DivStyle,
            Id, Len, MainAlign, Text,
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

        let mut parent = self.ui.add_div("Parent", None);

        parent.width(Len::PARENT);
        parent.axis = Axis::X;
        parent.main_align = MainAlign::SpaceBetween;
        parent.cross_align = Align::Center;
        parent.color = Color::RED.alpha(0.2);

        let parent = Some(parent.id);

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
        //     "sasass",
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
        //     "asdsadsadsasdsad",
        //     Some(parent),
        // );

        let mut purp_parent = self.ui.add_div("Purple Parent", parent);
        purp_parent.width(Len::px(100.0));
        purp_parent.height(Len::px(200.0));
        purp_parent.axis = Axis::Y;
        purp_parent.main_align = MainAlign::Center;
        purp_parent.cross_align = Align::Center;
        purp_parent.color = Color::PURPLE.alpha(0.5);
        purp_parent.border_radius = BorderRadius::all(20.0);
        purp_parent.border_color = Color::GREEN;
        purp_parent.border_thickness = 6.0;
        purp_parent.border_softness = 1.0;
        let purp_parent = Some(purp_parent.id);

        let mut c1 = self.ui.add_div("child 1 in purple", purp_parent);
        c1.width(Len::px(50.0));
        c1.height(Len::px(50.0));
        c1.main_align = MainAlign::Center;
        c1.cross_align = Align::Center;
        c1.color = Color::GREEN;

        let mut c2 = self.ui.add_div("child 2 in purple", purp_parent);
        c2.width(Len::px(70.0));
        c2.height(Len::px(30.0));
        c2.main_align = MainAlign::Center;
        c2.cross_align = Align::Center;
        c2.color = Color::WHITE;

        let mut other = self.ui.add_div("other", parent);
        other.width(Len::px(100.0));
        other.height(Len::px(20.0));
        other.color = Color::BLACK;

        let mut text_div = self.ui.add_text_div(
            Text {
                color: Color::new(6.0, 2.0, 2.0),
                string: "Hover me please, I will show you something!".into(),
                size: FontSize(48),
                offset_x: Len::px(30.0),
                offset_y: Len::px(30.0),
                ..Default::default()
            },
            "text div",
            parent,
        );

        text_div.width(Len::px(300.0));
        text_div.height(Len::px(400.0));

        text_div.color = Color::YELLOW;
        text_div.border_radius = BorderRadius::all(20.0);
        text_div.border_thickness = 20.0;

        // can immediately edit the style and text without a 1-frame lag:
        // 1 frame lag only applies to the layout rect (DivProps) itself.
        if text_div.mouse_in_rect() {
            let style = text_div.style();
            style.color = Color::BLUE;
            style.border_color = Color::GREEN;
            style.border_thickness = 6.0;
            // style.border_radius = BorderRadius::new(40.0, 40.0, 40.0, 40.0);
            text_div.text().color = Color::BLACK;
        }

        let total_time = self.deps.time.total().as_secs_f64() * 4.0;
        let total_time2 = self.deps.time.total().as_secs_f64() * 9.7;
        if text_div.mouse_in_rect() {
            let mut green_square = self.ui.add_div(2112213232, parent);

            green_square.width(Len::px(40.0));
            green_square.height(Len::px(40.0));
            green_square.color = Color::GREEN;
            green_square.offset_x = Len::px(total_time.sin() * 20.0);
            green_square.offset_y = Len::px(total_time2.cos() * 20.0);
        }

        let mut container2 = self.ui.add_div("Container 2", parent);
        container2.height(Len::PARENT);
        container2.main_align = MainAlign::SpaceAround;
        container2.cross_align = Align::Center;

        let container2 = Some(container2.id);

        {
            let clicked = self
                .ui
                .add(
                    Button {
                        text: "Click".into(),
                        ..Default::default()
                    },
                    "my button",
                    container2,
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
                    "my button 2",
                    container2,
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
                    "my button 3",
                    container2,
                )
                .clicked;
            if clicked {
                println!("Hello 3");
            }
        }

        // let mut ctx = self.deps.egui.context();
        // egui_inspect_board(&mut ctx, &mut self.ui);

        self.ui.end_frame(&mut self.deps.ui.fonts);
        self.deps.ui.ui_renderer.draw_ui_board(&self.ui);
        // std::thread::sleep(Duration::from_millis(150));
    }
}
