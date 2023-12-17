use std::{f32::consts::PI, sync::Arc};

use glam::{vec2, vec3};
use vert_framework::{
    app::App,
    batteries::{SimpleCamController, SpawnSomeCubes},
    flow::Flow,
    modules::{
        assets::fetchable_asset::{AssetSource, ImageAsset},
        graphics::elements::{
            buffer::ToRaw,
            color::Color,
            color_mesh::SingleColorMesh,
            rect::{Rect, RectTexture, RectWithTexture},
            rect_3d::Rect3D,
            texture::{BindableTexture, Texture},
            transform::Transform,
            ui_rect::UiRect,
        },
        ui::text_rasterizer::DrawText,
        Modules,
    },
    state::StateT,
};

pub struct MyState {
    test_texture: Arc<BindableTexture>,
}

impl StateT for MyState {
    async fn initialize(modules: &mut Modules) -> anyhow::Result<Self> {
        modules.add_battery(SimpleCamController);

        let image = AssetSource::from("./assets/test.png")
            .fetch::<ImageAsset>()
            .await
            .unwrap();

        let context = modules.graphics_context();
        let test_texture = BindableTexture::new(
            context,
            context.rgba_bind_group_layout,
            Texture::from_image(&context.device, &context.queue, &image.rgba),
        );

        for x in 0..30 {
            for y in 0..30 {
                for z in 0..30 {
                    let color = if (x + y) % 2 == 0 {
                        Some(Color::new(0.0, 0.0, x as f32 / 30.0))
                    } else {
                        Some(Color::new(0.0, 0.0, 0.001))
                    };

                    let color_mesh = SingleColorMesh::cube(
                        vec3(
                            x as f32 * 2.0 + 20.0 + (z as f32 * 0.1).sin() * 2.0,
                            y as f32 * 2.0 - 30.0 + (z as f32).sin() * 2.0,
                            z as f32 * 2.0 - 30.0 + ((x + y) % 2) as f32,
                        )
                        .into(),
                        modules.device(),
                        color,
                    );
                    modules.spawn(color_mesh);
                }
            }
        }

        // let color_mesh = SingleColorMesh::cube(
        //     vec3(7.0, 0.0, 2.0).into(),
        //     modules.device(),
        //     Some(Color::new(50.0, 50.0, 50.0)),
        // );
        // modules.spawn(color_mesh);

        Ok(MyState {
            test_texture: Arc::new(test_texture),
        })
    }

    fn update(&mut self, modules: &mut Modules) -> Flow {
        // /////////////////////////////////////////////////////////////////////////////
        // Draw some stuff (some things that are very bright)
        // /////////////////////////////////////////////////////////////////////////////

        modules.ui().draw_3d_text(
            &DrawText {
                text: "Vert".into(),
                font_layout_size: 100.0,
                font_texture_size: 200.0,
                max_width: Some(400.0),
                color: Color::new(10.0, 1.0, 3.0),
                ..Default::default()
            },
            &{
                // let the text face the camera
                let mut t = Transform::default();
                t.rotate_y(-PI / 2.0);
                t
            },
        );
        modules.ui().draw_3d_text(
            &DrawText {
                text: "Game Engine".into(),
                font_layout_size: 64.0,
                font_texture_size: 200.0,
                pos: vec2(0.0, 100.0),
                max_width: Some(400.0),
                color: Color::new(10.0, 1.0, 1.0),
                ..Default::default()
            },
            &{
                // let the text face the camera
                let mut t = Transform::default();
                t.rotate_y(-PI / 2.0);
                t
            },
        );

        // /////////////////////////////////////////////////////////////////////////////
        // Make bloom settings controllable by egui
        // /////////////////////////////////////////////////////////////////////////////

        let mut egui_context = modules.egui();
        let graphics_settings = modules.graphics_settings_mut();
        egui::Window::new("Graphics Settings").show(&mut egui_context, |ui| {
            ui.label("Bloom");
            ui.add(egui::Checkbox::new(
                &mut graphics_settings.bloom.activated,
                "Bloom Activated",
            ));
            if graphics_settings.bloom.activated {
                ui.add(egui::Slider::new(
                    &mut graphics_settings.bloom.blend_factor,
                    0.0..=1.0,
                ));
            }
        });

        Flow::Continue
    }
}

fn main() {
    App::<MyState>::run().unwrap();
}
