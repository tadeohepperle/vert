#![feature(const_fn_floating_point_arithmetic)]
#![feature(lazy_cell)]
#![feature(try_blocks)]
#![feature(associated_type_defaults)]
#![feature(entry_insert)]

pub mod app;
pub mod elements;
pub mod modules;
pub mod utils;

pub use app::{App, AppBuilder, Dependencies, Handle, MainModule, Module, Plugin};
pub use modules::WinitMain;

pub mod prelude {
    pub use super::app::*;
    pub use super::modules;
    pub use anyhow;
    pub use bytemuck;
    pub use egui;
    pub use fontdue;
    pub use glam;
    pub use slotmap;
    pub use wgpu;
    pub use winit;
    pub use winit::keyboard::*;
}
