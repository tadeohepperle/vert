use glam::vec3;
use vert_framework::{
    app::App,
    flow::Flow,
    modules::{graphics::elements::color_mesh::SingleColorMesh, Modules},
    state::StateT,
};

pub struct MyState {}

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
        Ok(MyState {})
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

        Flow::Continue
    }

    fn prepare(&mut self, modules: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {}
}

fn main() {
    App::<MyState>::run().unwrap();
}
