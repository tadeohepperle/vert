//! Run `RUST_LOG=INFO cargo run --example vert --release` to run this example.

use std::{f32::consts::PI, sync::Arc};

use glam::{vec2, vec3};
use vert::{
    batteries::{FlyCam, GraphicsSettingsController},
    elements::{Color, Transform},
    modules::{renderer::text_renderer::DrawText, DefaultModules},
    App, WinitConfig, WinitRunner,
};

fn main() {
    let runner = WinitRunner::new(WinitConfig::default());
    let mut my_state = MyState::new(runner.window());
    _ = runner.run(&mut my_state);
}

pub struct MyState {
    blue_cubes: Vec<Transform>,
    black_cubes: Vec<Transform>,
    mods: DefaultModules,
    graphics_controller: GraphicsSettingsController,
}

impl MyState {
    fn new(window: Arc<winit::window::Window>) -> Self {
        let mut blue_cubes: Vec<Transform> = vec![];
        let mut black_cubes: Vec<Transform> = vec![];

        for x in 0..30 {
            for y in 0..30 {
                for z in 0..30 {
                    let pos = vec3(
                        x as f32 * 2.0 + 20.0 + (z as f32 * 0.1).sin() * 2.0,
                        y as f32 * 2.0 - 30.0 + (z as f32).sin() * 2.0,
                        z as f32 * 2.0 - 30.0 + ((x + y) % 2) as f32,
                    );
                    if (x + y) % 2 == 0 {
                        blue_cubes.push(pos.into());
                    } else {
                        black_cubes.push(pos.into());
                    };
                }
            }
        }

        // use a very high energy green to get a nice background bloom

        let mut mods = DefaultModules::new(window).unwrap();
        let graphics_controller = GraphicsSettingsController::new(&mut mods);
        MyState {
            black_cubes,
            blue_cubes,
            mods,
            graphics_controller,
        }
    }

    fn update(&mut self) {
        // move camera:

        self.mods.gizmos.draw_xyz();
        FlyCam.update(&mut self.mods);
        self.graphics_controller.update(&mut self.mods);
        // /////////////////////////////////////////////////////////////////////////////
        // Draw some stuff (some things that are very bright)
        // /////////////////////////////////////////////////////////////////////////////

        let oscillator = ((self.mods.time.total().as_secs_f32() * 10.0).sin() + 1.0) / 2.0;
        let oscillator2 = self.mods.time.total().as_secs_f32().sin() * 0.3;

        // let the text face the camera
        let text_rotation = {
            let mut t = Transform::default();
            t.rotate_y(-PI / 2.0);
            t.position.y += 0.5;
            t
        };

        self.mods.text.draw_world_text(
            DrawText {
                text: "Vert".into(),
                font_layout_size: 100.0,
                font_texture_size: 200.0,
                max_width: Some(400.0),
                color: Color::new(
                    3.0 + oscillator * 10.0,
                    3.0 + (1.0 - oscillator) * 10.0,
                    3.0,
                ),
                ..Default::default()
            },
            text_rotation,
            &mut self.mods.world_rect,
        );

        self.mods.text.draw_world_text(
            DrawText {
                text: "Game Engine".into(),
                font_layout_size: 64.0,
                font_texture_size: 200.0,
                pos: vec2(0.0, 100.0),
                max_width: Some(400.0),
                color: Color::new(10.0, 1.0, 1.0),
            },
            text_rotation,
            &mut self.mods.world_rect,
        );

        for c in self.blue_cubes.iter_mut() {
            c.rotation.x = oscillator2;
        }

        self.mods
            .color_mesh
            .draw_cubes(&self.blue_cubes, Some(Color::from_hex("#02050d")));
        self.mods
            .color_mesh
            .draw_cubes(&self.black_cubes, Some(Color::from_hex("#000000")));
    }
}

impl App for MyState {
    fn receive_window_event(&mut self, event: &winit::event::WindowEvent) {
        self.mods.receive_window_event(event);
    }

    fn update(&mut self) -> vert::UpdateFlow {
        self.mods.begin_frame()?;
        self.update();
        self.mods.prepare_and_render(Color::new(0.5, 5.0, 0.7));
        self.mods.end_frame();
        vert::UpdateFlow::Continue
    }
}
