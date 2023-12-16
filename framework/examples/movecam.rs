use std::sync::Arc;

use glam::{vec2, vec3};
use vert_framework::{
    app::App,
    flow::Flow,
    modules::{
        assets::fetchable_asset::{AssetSource, ImageAsset},
        graphics::elements::{
            color::Color,
            color_mesh::SingleColorMesh,
            rect::{Rect, RectTexture, RectWithTexture},
            texture::{BindableTexture, Texture},
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
        for i in 0..10 {
            for j in 0..10 {
                let color_mesh = SingleColorMesh::cube(
                    vec3(i as f32 * 2.0, j as f32 * 2.0, j as f32 * 2.0).into(),
                    modules.device(),
                );
                modules.spawn(color_mesh);
            }
        }

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
        let wasd = modules.input().wasd_vec();
        let arrows = modules.input().arrow_vec();
        let updown = modules.input().rf_updown();

        egui::Window::new("Movement").show(&mut modules.egui(), |ui| {
            ui.label(format!("WASD: {wasd:?}"));
            ui.label(format!("ARROWS: {arrows:?}"));
        });

        // move camera around:
        const SPEED: f32 = 10.0;
        const ANGLE_SPEED: f32 = 1.8;

        let delta_time = modules.time().delta_secs();
        let cam_transform = modules.cam_transform_mut();

        cam_transform.pos += cam_transform.forward() * wasd.y * SPEED * delta_time;
        cam_transform.pos += cam_transform.right() * wasd.x * SPEED * delta_time;
        cam_transform.pos.y += updown * SPEED * delta_time;

        cam_transform.pitch += arrows.y * ANGLE_SPEED * delta_time;
        cam_transform.yaw += arrows.x * ANGLE_SPEED * delta_time;

        // move the cubes up and down:
        let total_time = modules.time().total_secs();
        let fps = modules.time().fps().round() as i64;
        // for (_, cube) in modules.arenas_mut().iter_mut::<SingleColorMesh>() {
        //     let t = cube.transform_mut();
        //     t.position.y = ((total_time + t.position.z * 0.2) * 3.0).sin() * t.position.z * 0.1;
        // }

        // draw rects:
        let ui = modules.ui();
        ui.draw_ui_rect(RectWithTexture {
            rect: UiRect {
                pos: Rect::new([200.0, 500.0], [300.0, 150.0]),
                uv: Rect::default(),
                color: Color::new(total_time.sin().abs(), 1.0, 0.0),
                border_radius: [20.0, 20.0, 20.0, 20.0],
            },
            texture: RectTexture::White,
        });

        ui.draw_ui_rect(RectWithTexture {
            rect: UiRect {
                pos: Rect::new([200.0, 200.0], [150.0, 120.0]),
                uv: Rect::default(),
                color: Color::RED.alpha(0.1),
                border_radius: [50.0, 0.0, 25.0, 0.0],
            },
            texture: RectTexture::Custom(self.test_texture.clone()),
        });

        ui.draw_text(&DrawText {
            text: format!("FPS: {fps} "),
            pos: vec2(1200.0, 0.0),
            font_texture_size: 32.0,
            font_layout_size: 32.0,
            max_width: None,
            color: Color::RED,
        });

        ui.draw_text(&DrawText {
            text: "I render my fonts as quads with UV coordinates in one big atlas texture. (This is 56px)".into(),
            pos: vec2(700.0, 120.0),
            font_texture_size: 56.0,
            font_layout_size: 56.0,
            max_width: Some(800.0),
            color: Color::WHITE,
        });

        let layout_size = 24.0 + (((total_time * 0.5).sin().abs() * 5.0) as i32) as f32 * 8.0;
        ui.draw_text(&DrawText {
            text: format!("Texture size: 24.0, layout size: {layout_size}"),
            pos: vec2(750.0, 600.0),
            font_texture_size: 24.0,
            font_layout_size: layout_size,
            max_width: Some(900.0),
            color: Color::GREEN,
        });

        ui.draw_ui_rect(RectWithTexture {
            rect: UiRect {
                pos: Rect::new(
                    [200.0, 350.0],
                    [400.0, (total_time * 4.0).sin() * 30.0 + 50.0],
                ),
                uv: Rect::default(),
                color: Color::u8(249, 151, 0).alpha(0.9),
                border_radius: [0.0, 0.0, 16.0, 10.0],
            },
            texture: RectTexture::Custom(self.test_texture.clone()),
        });

        Flow::Continue
    }

    fn prepare(&mut self, modules: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {}
}

fn main() {
    App::<MyState>::run().unwrap();
}
