use crate::{
    modules::{DefaultModules, Input, Time},
    utils::Timing,
};

pub struct FlyCam;

impl FlyCam {
    pub fn update(&self, deps: &mut DefaultModules) {
        let wasd = deps.input.wasd_vec();
        let arrows = deps.input.arrow_vec();
        let updown = deps.input.rf_updown();

        // move camera around:
        const SPEED: f32 = 10.0;
        const ANGLE_SPEED: f32 = 1.8;

        let delta_time = deps.time.delta().as_secs_f32();
        let cam = &mut deps.camera.transform;
        cam.pos += cam.forward() * wasd.y * SPEED * delta_time;
        cam.pos += cam.right() * wasd.x * SPEED * delta_time;
        cam.pos.y += updown * SPEED * delta_time;

        cam.pitch += arrows.y * ANGLE_SPEED * delta_time;
        cam.yaw += arrows.x * ANGLE_SPEED * delta_time;
    }
}
