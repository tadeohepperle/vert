use glam::vec3;
use vert_framework::{
    app::App,
    batteries::SimpleCamController,
    flow::Flow,
    modules::{
        graphics::{
            elements::{color::Color, transform::Transform},
            shader::color_mesh::{self, ColorMeshShader},
        },
        Modules,
    },
    state::StateT,
};

pub struct MyState;

impl StateT for MyState {
    async fn initialize(modules: &mut Modules) -> anyhow::Result<Self> {
        modules.add_battery(SimpleCamController);

        modules.graphics_settings_mut().bloom.activated = false;
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
        ColorMeshShader::draw_cubes(&transforms, None);
        modules.gizmos().draw_xyz();
        Flow::Continue
    }
}

fn main() {
    App::<MyState>::run().unwrap();
}
