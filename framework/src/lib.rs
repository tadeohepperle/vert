#![feature(try_trait_v2)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(lazy_cell)]
#![feature(try_blocks)]
#![feature(associated_type_defaults)]

pub mod app;
pub mod modules;
pub mod utils;

pub use app::{App, AppBuilder, Dependencies, Handle, MainModule, Module, Plugin};
pub use modules::WinitMain;

pub mod ext {
    pub use bytemuck;
    pub use egui;
    pub use glam;
    pub use wgpu;
    pub use winit;
}
