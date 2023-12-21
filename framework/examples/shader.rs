use glam::{vec2, vec3, Vec2};
use vert_framework::{
    app::App,
    batteries::SimpleCamController,
    flow::Flow,
    modules::{
        graphics::{
            elements::{color::Color, transform::Transform},
            shader::{
                color_mesh::ColorMeshRenderer,
                gizmos::Gizmos,
                text::{DrawText, TextRenderer},
                ui_rect::{Rect, UiRect, UiRectRenderer},
            },
        },
        Modules,
    },
    state::StateT,
};

pub struct MyState;

impl StateT for MyState {
    async fn initialize(modules: &mut Modules) -> anyhow::Result<Self> {
        modules.add_battery(SimpleCamController);

        // modules.graphics_settings_mut().bloom.activated = false;
        Ok(MyState)
    }

    fn update(&mut self, modules: &mut Modules) -> Flow {
        let transforms: Vec<Transform> = (0..1000)
            .map(|e| {
                let y = e / 30;
                let x = e % 30;

                Transform::from(vec3(x as f32, (e as f32 * 0.01).sin(), y as f32))
            })
            .collect();
        // ColorMeshRenderer::draw_cubes(&transforms, None);
        Gizmos::draw_xyz();

        UiRectRenderer::draw_rect(UiRect {
            pos: Rect::new([300., 200.0], [300., 200.0]),
            uv: Default::default(),
            color: Color::RED,
            border_radius: [30.0, 0.0, 20.0, 0.0],
        });

        TextRenderer::draw_ui_text(DrawText {
            text: "Hello".into(),
            pos: vec2(500.0, 200.0),
            font_texture_size: 60.0,
            font_layout_size: 60.0,
            max_width: None,
            color: Color::BLUE,
        });
        Flow::Continue
    }
}

fn main() {
    App::<MyState>::run().unwrap();
}
