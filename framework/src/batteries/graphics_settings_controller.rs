use std::f32::consts::PI;

use crate::modules::{
    graphics::{
        settings::ToneMappingSettings,
        statics::camera::{Projection, ProjectionKind},
    },
    Modules,
};

use super::Battery;

pub struct GraphicsSettingsController {
    camera_settings: CameraSettings,
}

impl GraphicsSettingsController {
    pub fn new() -> Self {
        GraphicsSettingsController {
            camera_settings: CameraSettings {
                is_ortho: false,
                ortho_y_height: 16.0,
                perspective_fovy_degrees: 50.0,
            },
        }
    }
}

pub struct CameraSettings {
    is_ortho: bool,
    ortho_y_height: f32,
    perspective_fovy_degrees: f32,
}

impl CameraSettings {
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
}

impl Battery for GraphicsSettingsController {
    fn initialize(&mut self, modules: &mut Modules) {
        self.camera_settings
            .apply(&mut modules.camera_mut().projection);
    }

    fn update(&mut self, modules: &mut Modules) {
        let mut egui_context = modules.egui();
        egui::Window::new("Graphics Settings").show(&mut egui_context, |ui| {
            // /////////////////////////////////////////////////////////////////////////////
            // Graphics Settings
            // /////////////////////////////////////////////////////////////////////////////
            let graphics_settings = modules.graphics_settings_mut();

            ui.label("Bloom");
            ui.add(egui::Checkbox::new(
                &mut graphics_settings.bloom.activated,
                "Bloom Activated",
            ));
            if graphics_settings.bloom.activated {
                ui.add(egui::Slider::new(
                    &mut graphics_settings.bloom.blend_factor,
                    0.0..=1.0,
                ));
            }

            ui.label("Tonemapping");

            ui.radio_value(
                &mut graphics_settings.tonemapping,
                ToneMappingSettings::Disabled,
                "Disabled",
            );

            ui.radio_value(
                &mut graphics_settings.tonemapping,
                ToneMappingSettings::Aces,
                "Aces",
            );
            // /////////////////////////////////////////////////////////////////////////////
            // Camera Settings
            // /////////////////////////////////////////////////////////////////////////////

            ui.label("Camera Kind");
            let orthographic_radio =
                ui.radio_value(&mut self.camera_settings.is_ortho, true, "Orthographic");
            let perspective_radio =
                ui.radio_value(&mut self.camera_settings.is_ortho, false, "Perspective");

            let slider = if self.camera_settings.is_ortho {
                ui.label("Orthographic Camera Y Height");
                ui.add(egui::Slider::new(
                    &mut self.camera_settings.ortho_y_height,
                    0.1..=100.0,
                ))
            } else {
                ui.label("Perspective Camera FOV (y) in degrees");
                ui.add(egui::Slider::new(
                    &mut self.camera_settings.perspective_fovy_degrees,
                    2.0..=170.0,
                ))
            };

            if slider.changed() || orthographic_radio.changed() || perspective_radio.changed() {
                self.camera_settings
                    .apply(&mut modules.camera_mut().projection);
            }
        });
    }

    fn prepare(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {}
}
