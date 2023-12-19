use glam::vec3;

use super::Battery;

pub struct SpawnSomeCubes;

impl Battery for SpawnSomeCubes {
    fn initialize(&mut self, modules: &mut crate::modules::Modules) {
        // for i in 3..10 {
        //     for j in 3..10 {
        //         let color_mesh = SingleColorMesh::cube(
        //             vec3(i as f32 * 2.0, j as f32 * 2.0, j as f32 * 2.0).into(),
        //             modules.device(),
        //             None,
        //         );
        //         modules.spawn(color_mesh);
        //     }
        // }
        // needle
    }
}
