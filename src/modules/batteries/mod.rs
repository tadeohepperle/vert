//! Batteries are also modules, but more for specific usecases.
//! todo!() make batteries addable and removable at runtime by dynamically checking if their dependencies are satisfied.

pub mod fly_cam;
pub use fly_cam::FlyCam;

pub mod graphics_settings_controller;
pub use graphics_settings_controller::GraphicsSettingsController;
