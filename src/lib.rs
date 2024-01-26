#![feature(const_fn_floating_point_arithmetic)]
#![feature(lazy_cell)]
#![feature(try_blocks)]
#![feature(associated_type_defaults)]
#![feature(entry_insert)]
#![feature(try_trait_v2)]
#![feature(sync_unsafe_cell)]

pub mod app;
pub use app::{App, UpdateFlow, WinitConfig, WinitRunner};

pub mod assets;
pub use assets::{OwnedPtr, Ptr};

pub mod batteries;
pub mod elements;
pub mod lifecycle;
pub mod modules;
pub mod utils;

pub use lifecycle::{Prepare, ReceiveWindowEvent, Resize, Resized};

pub mod ext {
    pub use anyhow;
    pub use bytemuck;
    pub use egui;
    pub use fontdue;
    pub use glam;
    pub use image;
    pub use slotmap;
    pub use smallvec;
    pub use tokio;
    pub use wgpu;
    pub use winit;
    pub use winit::keyboard::*;
}
