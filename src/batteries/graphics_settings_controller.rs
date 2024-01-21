use std::f32::consts::PI;

use crate::{
    elements::camera3d::{Projection, ProjectionKind},
    modules::DefaultModules,
};

pub struct GraphicsSettingsController {
    is_ortho: bool,
    ortho_y_height: f32,
    perspective_fovy_degrees: f32,
}

impl GraphicsSettingsController {
    pub fn new(deps: &mut DefaultModules) -> Self {
        let settings = GraphicsSettingsController {
            is_ortho: false,
            ortho_y_height: 16.0,
            perspective_fovy_degrees: 50.0,
        };
        settings.apply(&mut deps.camera.projection);
        settings
    }

    pub fn apply(&self, p: &mut Projection) {
        if self.is_ortho {
            p.kind = ProjectionKind::Orthographic {
                y_height: self.ortho_y_height,
            }
        } else {
            p.kind = ProjectionKind::Perspective {
                fov_y_radians: self.perspective_fovy_degrees / 180.0 * PI,
            }
        }
    }

    pub fn update(&mut self, deps: &mut DefaultModules) {
        let mut egui_context = deps.egui.context();
        egui::Window::new("Graphics Settings").show(&mut egui_context, |ui| {
            // /////////////////////////////////////////////////////////////////////////////
            // Graphics Settings
            // /////////////////////////////////////////////////////////////////////////////
            let bloom_settings = deps.bloom.settings_mut();
            ui.label(format!(
                "{} fps / {:.3} ms",
                deps.time.fps().round() as i32,
                deps.time.delta().as_secs_f32() * 1000.0
            ));
            ui.label("Bloom");
            ui.add(egui::Checkbox::new(
                &mut bloom_settings.activated,
                "Bloom Activated",
            ));
            if bloom_settings.activated {
                ui.add(egui::Slider::new(
                    &mut bloom_settings.blend_factor,
                    0.0..=1.0,
                ));
            }

            let tone_mapping = deps.tone_mapping.enabled_mut();
            ui.label("Tonemapping");
            ui.radio_value(tone_mapping, false, "Disabled");
            ui.radio_value(tone_mapping, true, "Aces");
            // /////////////////////////////////////////////////////////////////////////////
            // Camera Settings
            // /////////////////////////////////////////////////////////////////////////////

            ui.label("Camera Kind");
            let orthographic_radio = ui.radio_value(&mut self.is_ortho, true, "Orthographic");
            let perspective_radio = ui.radio_value(&mut self.is_ortho, false, "Perspective");

            let slider = if self.is_ortho {
                ui.label("Orthographic Camera Y Height");
                ui.add(egui::Slider::new(&mut self.ortho_y_height, 0.1..=100.0))
            } else {
                ui.label("Perspective Camera FOV (y) in degrees");
                ui.add(egui::Slider::new(
                    &mut self.perspective_fovy_degrees,
                    2.0..=170.0,
                ))
            };

            if slider.changed() || orthographic_radio.changed() || perspective_radio.changed() {
                self.apply(&mut deps.camera.projection);
            }
        });
    }
}
