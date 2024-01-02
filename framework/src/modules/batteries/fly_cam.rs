use crate::{
    modules::{Egui, Input, MainCamera3D, Schedule, Scheduler, Time},
    utils::Timing,
    Dependencies, Handle, Module,
};

#[derive(Debug, Dependencies)]
pub struct Deps {
    cam: Handle<MainCamera3D>,
    input: Handle<Input>,
    time: Handle<Time>,
    scheduler: Handle<Scheduler>,
    egui: Handle<Egui>,
}

pub struct FlyCam {
    deps: Deps,
}

impl Module for FlyCam {
    type Config = ();

    type Dependencies = Deps;

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        Ok(FlyCam { deps })
    }

    fn intialize(handle: Handle<Self>) -> anyhow::Result<()> {
        let mut scheduler = handle.deps.scheduler;
        scheduler.register(handle, Schedule::Update, Timing::DEFAULT, Self::update);
        Ok(())
    }
}

impl FlyCam {
    pub fn update(&mut self) {
        let input = self.deps.input;
        let wasd = input.wasd_vec();
        let arrows = input.arrow_vec();
        let updown = input.rf_updown();

        let mut egui_ctx = self.deps.egui.context();
        egui::Window::new("Movement").show(&mut egui_ctx, |ui| {
            ui.label(format!("WASD: {wasd:?}"));
            ui.label(format!("ARROWS: {arrows:?}"));
        });

        // move camera around:
        const SPEED: f32 = 10.0;
        const ANGLE_SPEED: f32 = 1.8;

        let delta_time = self.deps.time.delta_secs();
        let cam = self.deps.cam.camera_mut();
        let cam_transform = &mut cam.transform;

        cam_transform.pos += cam_transform.forward() * wasd.y * SPEED * delta_time;
        cam_transform.pos += cam_transform.right() * wasd.x * SPEED * delta_time;
        cam_transform.pos.y += updown * SPEED * delta_time;

        cam_transform.pitch += arrows.y * ANGLE_SPEED * delta_time;
        cam_transform.yaw += arrows.x * ANGLE_SPEED * delta_time;
    }
}
