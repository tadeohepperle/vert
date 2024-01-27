//! Run `RUST_LOG=INFO cargo run --example vert --release` to run this example.

use std::borrow::Cow;

use fontdue::{Font, FontSettings};
use glam::dvec2;
use smallvec::{smallvec, SmallVec};
use vert::{
    batteries::{FlyCam, GraphicsSettingsController},
    elements::{Color, Rect, Transform},
    modules::{
        renderer::ui_rect::UiRect,
        ui::{
            Align, Axis, Board, BoardInput, BorderRadius, Button, FontSize, Len, MainAlign,
            Padding, Span, Text, TextSection,
        },
        DefaultModules,
    },
    App, OwnedPtr, WinitConfig, WinitRunner,
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
    font: OwnedPtr<Font>,
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

        let font_bytes = include_bytes!("../assets/Lora.ttf");
        let font = fontdue::Font::from_bytes(&font_bytes[..], FontSettings::default()).unwrap();

        dbg!(&font);
        let font = OwnedPtr::new(font);
        MyApp {
            ui: Board::new(dvec2(800.0, 800.0)),
            mods,
            graphics_settings,
            font,
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
            self.mods.fonts.atlas_texture().ptr(),
        );

        let mut parent = self.ui.add_div("Parent", None);
        parent.width(Len::PARENT);
        parent.height(Len::PARENT);
        parent.axis = Axis::X;
        parent.main_align = MainAlign::Center;
        parent.cross_align = Align::Center;
        parent.color = Color::RED.alpha(0.2);
        let parent = Some(parent.id);

        let mut little_box = self.ui.add_unbound_div("lil box");
        little_box.width(Len::px(90.0));
        little_box.height(Len::px(40.0));
        little_box.color = Color::BLUE;
        little_box.add_z_bias(5);

        if little_box.mouse_in_rect() {
            little_box.color = Color::RED;
        }

        let little_box = little_box.id;

        let mut quad = self.ui.add_text_div(
            Text {
                spans: smallvec![
                    Span::Text(TextSection {
                        color: Color::BLACK,
                        string: Cow::Borrowed("hello"),
                        size: FontSize(40)
                    }),
                    Span::FixedSizeDiv {
                        id: little_box,
                        width: 90.0,
                        height: 40.0,
                        font_size: FontSize(40)
                    },
                    Span::Text(TextSection {
                        color: Color::BLACK,
                        string: Cow::Borrowed("hello I want to eat a cheeseburger"),
                        size: FontSize(40)
                    })
                ],
                font: Some(self.font.ptr()),
                offset_x: Len::ZERO,
                offset_y: Len::ZERO,
                line_height: 1.0,
            },
            219912,
            parent,
        );
        quad.width(Len::parent(0.3));
        quad.padding = Padding::all(Len::px(24.0));
        quad.color = Color::WHITE.alpha(0.2);

        let mut quad2 = self.ui.add(Button::default(), "wuad2", parent);

        self.ui.end_frame(&mut self.mods.fonts);
        self.mods.ui.draw_ui_board(&self.ui);
        // std::thread::sleep(Duration::from_millis(150));
    }
}
