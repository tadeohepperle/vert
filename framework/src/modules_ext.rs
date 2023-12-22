//! public extension functions on the modules, that represent common operations that should be exposed to the user.

use crate::{
    batteries::Battery,
    modules::{
        graphics::{
            settings::GraphicsSettings,
            statics::camera::{CamTransform, Camera, Projection},
            Renderer,
        },
        input::Input,
        time::Time,
        Modules,
    },
};

impl Modules {
    pub fn device(&self) -> &wgpu::Device {
        &self.graphics.context.device
    }

    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    pub fn egui(&self) -> egui::Context {
        self.egui.context()
    }

    // pub fn ui(&mut self) -> &mut ImmediateUi {
    //     &mut self.ui
    // }

    pub fn time(&mut self) -> &Time {
        &self.time
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn cam_transform(&self) -> &CamTransform {
        self.camera.transform()
    }

    pub fn cam_transform_mut(&mut self) -> &mut CamTransform {
        self.camera.transform_mut()
    }

    pub fn cam_projection_mut(&mut self) -> &mut Projection {
        self.camera.projection_mut()
    }

    pub fn add_battery<T: Battery>(&mut self, battery: T) {
        // todo! we dont even have a check here if this battery already exists... but anyway soon we will
        // through batteries away and implement proper systems.

        let mut batteries = self.batteries.take().unwrap();
        batteries.add(battery, self);
        self.batteries = Some(batteries);
    }

    pub fn graphics_settings(&self) -> &GraphicsSettings {
        &self.renderer.settings()
    }

    pub fn graphics_settings_mut(&mut self) -> &mut GraphicsSettings {
        self.renderer.settings_mut()
    }
}
