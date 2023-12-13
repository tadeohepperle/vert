use vert_core::{arenas::arena::ArenaIndex, component::Component};

use super::{input::Input, Modules};

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
}
