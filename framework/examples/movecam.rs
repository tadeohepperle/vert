use std::sync::Arc;

use glam::vec3;
use vert_framework::{
    app::App,
    flow::Flow,
    modules::{
        assets::fetchable_asset::{AssetSource, ImageAsset},
        graphics::elements::{
            color::Color,
            color_mesh::SingleColorMesh,
            texture::{BindableTexture, Texture},
            ui_rect::{UiRect, UiRectInstance},
        },
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
        for (_, cube) in modules.arenas_mut().iter_mut::<SingleColorMesh>() {
            let t = cube.transform_mut();
            t.position.y = ((total_time + t.position.z * 0.2) * 3.0).sin() * t.position.z * 0.1;
        }

        // draw rects:
        let ui = modules.ui();
        ui.draw_rect(UiRect {
            instance: UiRectInstance {
                posbb: [200.0, 200.0, 300.0, 500.0],
                uvbb: [0.0, 0.0, 1.0, 0.5],
                color: Color::RED,
                border_radius: [20.0, 20.0, 20.0, 20.0],
            },
            texture: None,
        });

        ui.draw_rect(UiRect {
            instance: UiRectInstance {
                posbb: [400.0, 400.0, 600.0, 600.0],
                uvbb: [0.0, 0.0, 1.0, 1.0],
                color: Color::RED.alpha(0.0),
                border_radius: [50.0, 0.0, 0.0, 0.0],
            },
            texture: Some(self.test_texture.clone()),
        });

        ui.draw_rect(UiRect {
            instance: UiRectInstance {
                posbb: [700.0, 200.0, 2000.0, 1600.0],
                uvbb: [0.0, 0.0, 1.0, 1.0],
                color: Color::RED.alpha(0.2),
                border_radius: [0.0, 0.0, 10.0, 10.0],
            },
            texture: Some(self.test_texture.clone()),
        });

        Flow::Continue
    }

    fn prepare(&mut self, modules: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {}
}

fn main() {
    App::<MyState>::run().unwrap();
}
