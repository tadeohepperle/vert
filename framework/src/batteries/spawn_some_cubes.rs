use glam::vec3;

use crate::modules::graphics::elements::color_mesh::SingleColorMesh;

use super::Battery;

pub struct SpawnSomeCubes;

impl Battery for SpawnSomeCubes {
    fn initialize(&mut self, modules: &mut crate::modules::Modules) {
        for i in 0..10 {
            for j in 0..10 {
                let color_mesh = SingleColorMesh::cube(
                    vec3(i as f32 * 2.0, j as f32 * 2.0, j as f32 * 2.0).into(),
                    modules.device(),
                );
                modules.spawn(color_mesh);
            }
        }
    }
}
