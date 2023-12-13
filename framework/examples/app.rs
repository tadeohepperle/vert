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
        // for i in 0..10 {
        //     for j in 0..10 {
        //         let color_mesh = SingleColorMesh::cube(
        //             vec3(i as f32 * 2.0, j as f32 * 2.0, j as f32 * 2.0).into(),
        //             modules.device(),
        //         );
        //         modules.spawn(color_mesh);
        //     }
        // }
        Ok(MyState {})
    }

    fn update(&mut self, modules: &mut Modules) -> Flow {
        let vec = modules.input().wasd_vec();

        egui::Window::new("Hello Title")
            .show(&mut modules.egui(), |ui| ui.label(format!("{vec:?}")));

        Flow::Continue
    }

    fn prepare(&mut self, modules: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {}
}

fn main() {
    App::<MyState>::run().unwrap();
}
