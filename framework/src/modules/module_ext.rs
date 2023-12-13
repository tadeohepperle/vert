use vert_core::{arenas::arena::ArenaIndex, component::Component};

use super::{graphics::elements::camera::CamTransform, input::Input, time::Time, Modules};

impl Modules {
    pub fn device(&self) -> &wgpu::Device {
        &self.graphics.context.device
    }

    pub fn spawn<C: Component>(&mut self, component: C) -> ArenaIndex {
        self.arenas.insert(component)
    }

    pub fn despawn<C: Component>(&mut self, i: ArenaIndex) -> Option<C> {
        self.arenas.remove(i)
    }

    pub fn get_mut<C: Component>(&mut self, i: ArenaIndex) -> Option<&mut C> {
        self.arenas.get_mut(i)
    }

    pub fn get<C: Component>(&self, i: ArenaIndex) -> Option<&C> {
        self.arenas.get(i)
    }

    pub fn egui(&self) -> egui::Context {
        self.egui.context()
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn cam_transform(&self) -> &CamTransform {
        self.camera.transform()
    }

    pub fn cam_transform_mut(&mut self) -> &mut CamTransform {
        self.camera.transform_mut()
    }

    pub fn time(&mut self) -> &Time {
        &self.time
    }
}
