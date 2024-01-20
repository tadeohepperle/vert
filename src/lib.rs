#![feature(const_fn_floating_point_arithmetic)]
#![feature(lazy_cell)]
#![feature(try_blocks)]
#![feature(associated_type_defaults)]
#![feature(entry_insert)]

pub mod app;
pub mod assets;
pub mod elements;
pub mod modules;
pub mod utils;

pub use app::{App, AppBuilder, Dependencies, Handle, MainModule, Module, Plugin};

pub mod ext {
    pub use anyhow;
    pub use bytemuck;
    pub use egui;
    pub use fontdue;
    pub use glam;
    pub use image;
    pub use slotmap;
    pub use tokio;
    pub use wgpu;
    pub use winit;
    pub use winit::keyboard::*;
}
