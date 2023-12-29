//! Run `RUST_LOG=INFO cargo run --example vert --release` to run this example.

use std::{f32::consts::PI, sync::Arc};

use glam::{vec2, vec3, Vec3};
use vert_framework::{
    app::App,
    batteries::{SimpleCamController, SpawnSomeCubes},
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
use winit::event::MouseButton;

pub struct MyState {
    ray_points: Vec<Vec3>,
}

impl StateT for MyState {
    async fn initialize(modules: &mut Modules) -> anyhow::Result<Self> {
        modules.add_battery(SimpleCamController);

        modules.graphics_settings_mut().bloom.activated = false;

        Ok(MyState { ray_points: vec![] })
    }

    fn update(&mut self, modules: &mut Modules) -> Flow {
        TextRenderer::draw_ui_text(
            DrawText::new("Press the left mouse button to shoot a ray")
                .pos(vec2(200.0, 0.0))
                .color(Color::BLACK),
        );

        let time = modules.time().total_secs();
        let mut environment: Vec<Transform> = vec![];

        for i in 0..300 {
            for j in 0..300 {
                let y = (time + i as f32).sin();
                environment.push(vec3(i as f32 * 2.0, y, j as f32 * 2.0).into());
            }
        }

        ColorMeshRenderer::draw_cubes(&environment, None);

        let input = modules.input();
        if input.mouse_buttons().just_pressed(MouseButton::Left) {
            let screen_pos = input.cursor_pos();

            dbg!(screen_pos);
            let ray = modules.camera().ray_from_screen_pos(screen_pos);

            self.ray_points = (5..50)
                .map(|i| ray.origin + ray.direction * i as f32)
                .collect();

            println!("Shot ray: {:?}", self.ray_points);
        }

        for (i, p) in self.ray_points.iter().enumerate() {
            ColorMeshRenderer::draw_cubes(
                &[Transform::from(*p).with_scale(0.2)],
                Some(Color::new(i as f32 / 50.0, 0.0, 0.0)),
            );
        }
        Flow::Continue
    }
}

fn main() {
    App::<MyState>::run().unwrap();
}
