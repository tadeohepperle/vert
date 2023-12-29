#![feature(try_trait_v2)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(lazy_cell)]
#![feature(try_blocks)]
#![feature(associated_type_defaults)]

pub mod app;
pub mod batteries;
pub mod constants;
pub mod flow;
pub mod module;
pub mod modules;
pub mod modules_ext;
pub mod state;
pub mod systems;
pub mod utils;

pub mod ext {
    pub use bytemuck;
    pub use egui;
    pub use glam;
    pub use wgpu;
    pub use winit;
}
