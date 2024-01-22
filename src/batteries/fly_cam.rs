use crate::{
    elements::Camera3d,
    modules::{DefaultModules, Input, Time},
};

pub struct FlyCam;

impl FlyCam {
    pub fn update(&self, deps: &mut DefaultModules) {
        Self.update2(&deps.input, &deps.time, &mut deps.camera)
    }

    pub fn update2(&self, input: &Input, time: &Time, camera: &mut Camera3d) {
        let wasd = input.wasd_vec();
        let arrows = input.arrow_vec();
        let updown = input.rf_updown();

        // move camera around:
        const SPEED: f32 = 10.0;
        const ANGLE_SPEED: f32 = 1.8;

        let delta_time = time.delta().as_secs_f32();
        let cam = &mut camera.transform;
        cam.pos += cam.forward() * wasd.y * SPEED * delta_time;
        cam.pos += cam.right() * wasd.x * SPEED * delta_time;
        cam.pos.y += updown * SPEED * delta_time;

        cam.pitch += arrows.y * ANGLE_SPEED * delta_time;
        cam.yaw += arrows.x * ANGLE_SPEED * delta_time;
    }
}
