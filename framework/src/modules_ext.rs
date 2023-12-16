use vert_core::{
    arenas::{arena::ArenaIndex, Arenas},
    component::Component,
};

use crate::{
    batteries::Battery,
    modules::{
        assets::AssetServer,
        graphics::elements::{camera::CamTransform, gizmos::GizmosRenderer},
        input::Input,
        time::Time,
        ui::ImmediateUi,
        Modules,
    },
};

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

    pub fn ui(&mut self) -> &mut ImmediateUi {
        &mut self.ui
    }

    pub fn time(&mut self) -> &Time {
        &self.time
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn assets(&mut self) -> &AssetServer {
        &self.assets
    }

    pub fn cam_transform(&self) -> &CamTransform {
        self.camera.transform()
    }

    pub fn cam_transform_mut(&mut self) -> &mut CamTransform {
        self.camera.transform_mut()
    }

    pub fn arenas_mut(&mut self) -> &mut Arenas {
        &mut self.arenas
    }

    pub fn gizmos(&mut self) -> &mut GizmosRenderer {
        &mut self.renderer.gizmos_renderer
    }

    pub fn add_battery<T: Battery>(&mut self, battery: T) {
        // todo! we dont even have a check here if this battery already exists... but anyway soon we will
        // through batteries away and implement proper systems.

        let mut batteries = self.batteries.take().unwrap();
        batteries.add(battery, self);
        self.batteries = Some(batteries);
    }
}
