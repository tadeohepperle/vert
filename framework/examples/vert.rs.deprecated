//! Run `RUST_LOG=INFO cargo run --example vert --release` to run this example.

use std::{f32::consts::PI, sync::Arc};

use glam::{vec2, vec3};
use vert_framework::{
    app::App,
    batteries::{GraphicsSettingsController, SimpleCamController, SpawnSomeCubes},
    flow::Flow,
    modules::{
        assets::fetchable_asset::{AssetSource, ImageAsset},
        graphics::{
            elements::{
                buffer::ToRaw,
                color::Color,
                texture::{BindableTexture, Texture},
                transform::Transform,
            },
            shader::{
                color_mesh::ColorMeshRenderer,
                text::{DrawText, TextRenderer},
            },
            statics::camera::Projection,
        },
        Modules,
    },
    state::StateT,
};

pub struct MyState {
    blue_cubes: Vec<Transform>,
    black_cubes: Vec<Transform>,
    camera_orthographic: bool,
}

impl StateT for MyState {
    async fn initialize(modules: &mut Modules) -> anyhow::Result<Self> {
        modules.add_battery(SimpleCamController);
        modules.add_battery(GraphicsSettingsController::new());
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
        modules.graphics_settings_mut().clear_color = Color::new(2.0, 8.0, 2.0);

        Ok(MyState {
            black_cubes,
            blue_cubes,
            camera_orthographic: false,
        })
    }

    fn update(&mut self, modules: &mut Modules) -> Flow {
        // /////////////////////////////////////////////////////////////////////////////
        // Draw some stuff (some things that are very bright)
        // /////////////////////////////////////////////////////////////////////////////

        let oscillator = ((modules.time().total_secs() * 10.0).sin() + 1.0) / 2.0;
        let oscillator2 = modules.time().total_secs().sin() * 0.3;

        // let the text face the camera
        let text_rotation = {
            let mut t = Transform::default();
            t.rotate_y(-PI / 2.0);
            t.position.y += 0.5;
            t
        };

        TextRenderer::draw_3d_text(
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
        );
        TextRenderer::draw_3d_text(
            DrawText {
                text: "Game Engine".into(),
                font_layout_size: 64.0,
                font_texture_size: 200.0,
                pos: vec2(0.0, 100.0),
                max_width: Some(400.0),
                color: Color::new(10.0, 1.0, 1.0),
                ..Default::default()
            },
            text_rotation,
        );

        for c in self.blue_cubes.iter_mut() {
            c.rotation.x = oscillator2;
        }

        ColorMeshRenderer::draw_cubes(&self.blue_cubes, None);
        ColorMeshRenderer::draw_cubes(&self.black_cubes, None);

        Flow::Continue
    }
}

fn main() {
    App::<MyState>::run().unwrap();
}
