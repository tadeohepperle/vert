//! Run `RUST_LOG=INFO cargo run --example vert --release` to run this example.

use glam::dvec2;
use smallvec::smallvec;
use vert::{
    batteries::{FlyCam, GraphicsSettingsController},
    elements::{Color, Rect, Transform},
    modules::{
        renderer::ui_rect::UiRect,
        ui::{
            Align, Axis, Board, BoardInput, BorderRadius, Button, FontSize, Len, MainAlign, Text,
            TextSection,
        },
        DefaultModules,
    },
    App, WinitConfig, WinitRunner,
};

fn main() {
    let runner = WinitRunner::new(WinitConfig::default());
    let mods = DefaultModules::new(runner.window()).unwrap();
    let mut my_state = MyApp::new(mods);
    _ = runner.run(&mut my_state);
}

struct MyApp {
    mods: DefaultModules,
    ui: Board,
    graphics_settings: GraphicsSettingsController,
}

impl App for MyApp {
    fn receive_window_event(&mut self, event: &winit::event::WindowEvent) {
        self.mods.receive_window_event(event);
    }

    fn update(&mut self) -> vert::UpdateFlow {
        self.mods.begin_frame()?;
        self.update();
        self.mods.prepare_and_render(Color::new(0.0, 0.4, 0.4));
        self.mods.end_frame();
        vert::UpdateFlow::Continue
    }
}

impl MyApp {
    fn new(mut mods: DefaultModules) -> Self {
        mods.bloom.settings_mut().activated = false;
        mods.ui.watch_shader_file("./src/modules/ui/ui.wgsl");
        let graphics_settings = GraphicsSettingsController::new(&mut mods);
        MyApp {
            ui: Board::new(dvec2(800.0, 800.0)),
            mods,
            graphics_settings,
        }
    }
    fn update(&mut self) {
        FlyCam.update(&mut self.mods);
        self.graphics_settings.update(&mut self.mods);

        self.mods.gizmos.draw_xyz();
        self.mods
            .color_mesh
            .draw_cubes(&[Transform::new(1.0, 1.0, 1.0)], None);

        let size = self.mods.ctx.size;
        self.ui.start_frame(
            BoardInput::from_input_module(&self.mods.input),
            dvec2(size.width as f64, size.height as f64),
        );

        self.mods.world_rect.draw_textured_rect(
            UiRect {
                pos: Rect::new(0.0, 0.0, 1024.0, 1024.0),
                uv: Rect::UNIT,
                color: Color::WHITE,
                border_radius: Default::default(),
            },
            Transform::default(),
            self.mods.fonts.atlas_texture(),
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
                sections: smallvec![TextSection {
                    color: Color::new(6.0, 2.0, 2.0),
                    string: "Hover me please, I will show you something!".into(),
                    size: FontSize(48),
                }],
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
            text_div.text().sections[0].color = Color::BLACK;
        }

        let total_time = self.mods.time.total().as_secs_f64() * 4.0;
        let total_time2 = self.mods.time.total().as_secs_f64() * 9.7;
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

        self.ui.end_frame(&mut self.mods.fonts, &self.mods.arenas);
        self.mods.ui.draw_ui_board(&self.ui);
        // std::thread::sleep(Duration::from_millis(150));
    }
}
