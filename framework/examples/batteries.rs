use std::sync::Arc;

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
        modules.add_battery(SpawnSomeCubes);
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

        Ok(MyState {
            test_texture: Arc::new(test_texture),
        })
    }

    fn update(&mut self, modules: &mut Modules) -> Flow {
        modules
            .gizmos()
            .draw_cube(vec3(0.0, 0.0, 0.0), 1.0, Color::WHITE);

        modules.gizmos().draw_xyz();

        modules.ui().draw_text(&DrawText {
            text: "Hello".into(),
            font_layout_size: 64.0,
            font_texture_size: 64.0,
            ..Default::default()
        });

        modules.ui().draw_3d_text(
            &DrawText {
                text: "Hello".into(),
                font_layout_size: 64.0,
                font_texture_size: 64.0,
                ..Default::default()
            },
            &Transform::default(),
        );

        modules.ui().draw_3d_rect(RectWithTexture {
            instance: Rect3D {
                ui_rect: UiRect {
                    pos: Rect {
                        offset: [100.0, 100.0],
                        size: [100.0, 200.0],
                    },
                    uv: Rect::default(),
                    color: Color::YELLOW,
                    border_radius: [0.0, 0.0, 0.0, 0.0],
                },
                transform: Transform::default().to_raw(),
            },
            texture: RectTexture::Custom(self.test_texture.clone()),
        });

        Flow::Continue
    }
}

fn main() {
    App::<MyState>::run().unwrap();
}
