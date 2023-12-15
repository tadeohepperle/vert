use std::sync::Arc;

use glam::{vec2, vec3};
use vert_framework::{
    app::App,
    batteries::{SimpleCamController, SpawnSomeCubes},
    flow::Flow,
    modules::{
        assets::fetchable_asset::{AssetSource, ImageAsset},
        graphics::elements::{
            color::Color,
            color_mesh::SingleColorMesh,
            rect::{Rect, RectTexture, RectWithTexture},
            texture::{BindableTexture, Texture},
            transform::Transform,
            ui_rect::UiRect,
        },
        ui::text_rasterizer::DrawText,
        Modules,
    },
    state::StateT,
};

pub struct MyState;

impl StateT for MyState {
    async fn initialize(modules: &mut Modules) -> anyhow::Result<Self> {
        modules.add_battery(SpawnSomeCubes);
        modules.add_battery(SimpleCamController);
        Ok(MyState)
    }

    fn update(&mut self, modules: &mut Modules) -> Flow {
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

        Flow::Continue
    }
}

fn main() {
    App::<MyState>::run().unwrap();
}
