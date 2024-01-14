use std::f32::consts::PI;

use crate::{
    elements::camera3d::{Projection, ProjectionKind},
    modules::{
        DefaultDependencies, Schedule,
        ToneMappingSettings,
    },
    utils::Timing, Handle, Module,
};

pub struct GraphicsSettingsController {
    camera_settings: CameraSettings,
    deps: DefaultDependencies,
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

impl Module for GraphicsSettingsController {
    type Config = ();

    type Dependencies = DefaultDependencies;

    fn new(_config: Self::Config, mut deps: Self::Dependencies) -> anyhow::Result<Self> {
        let camera_settings = CameraSettings {
            is_ortho: false,
            ortho_y_height: 16.0,
            perspective_fovy_degrees: 50.0,
        };

        camera_settings.apply(&mut deps.camera_3d.camera_mut().projection);

        Ok(GraphicsSettingsController {
            camera_settings,
            deps,
        })
    }

    fn intialize(handle: Handle<Self>) -> anyhow::Result<()> {
        let mut scheduler = handle.deps.scheduler;
        scheduler.register(handle, Schedule::Update, Timing::DEFAULT, Self::update);
        Ok(())
    }
}

impl GraphicsSettingsController {
    fn update(&mut self) {
        let mut egui_context = self.deps.egui.context();
        egui::Window::new("Graphics Settings").show(&mut egui_context, |ui| {
            // /////////////////////////////////////////////////////////////////////////////
            // Graphics Settings
            // /////////////////////////////////////////////////////////////////////////////
            let bloom_settings = self.deps.bloom.settings_mut();
            ui.label(format!(
                "{} fps / {:.3} ms",
                self.deps.time.fps().round() as i32,
                self.deps.time.delta().as_secs_f32() * 1000.0
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

            let tone_mapping_settings = self.deps.tone_mapping.settings_mut();
            ui.label("Tonemapping");
            ui.radio_value(
                tone_mapping_settings,
                ToneMappingSettings::Disabled,
                "Disabled",
            );
            ui.radio_value(tone_mapping_settings, ToneMappingSettings::Aces, "Aces");
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
                    .apply(&mut self.deps.camera_3d.camera_mut().projection);
            }
        });
    }
}
