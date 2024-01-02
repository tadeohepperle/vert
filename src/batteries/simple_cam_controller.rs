use super::Battery;

pub struct SimpleCamController;

impl Battery for SimpleCamController {
    fn update(&mut self, modules: &mut crate::modules::Modules) {
        let wasd = modules.input().wasd_vec();
        let arrows = modules.input().arrow_vec();
        let updown = modules.input().rf_updown();

        egui::Window::new("Movement").show(&mut modules.egui(), |ui| {
            ui.label(format!("WASD: {wasd:?}"));
            ui.label(format!("ARROWS: {arrows:?}"));
        });

        // move camera around:
        const SPEED: f32 = 10.0;
        const ANGLE_SPEED: f32 = 1.8;

        let delta_time = modules.time().delta_secs();
        let cam_transform = &mut modules.camera_mut().transform;

        cam_transform.pos += cam_transform.forward() * wasd.y * SPEED * delta_time;
        cam_transform.pos += cam_transform.right() * wasd.x * SPEED * delta_time;
        cam_transform.pos.y += updown * SPEED * delta_time;

        cam_transform.pitch += arrows.y * ANGLE_SPEED * delta_time;
        cam_transform.yaw += arrows.x * ANGLE_SPEED * delta_time;
    }
}
