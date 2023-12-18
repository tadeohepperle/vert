#![feature(try_trait_v2)]
#![feature(const_fn_floating_point_arithmetic)]

pub mod app;
pub mod batteries;
pub mod constants;
pub mod flow;
pub mod modules;
pub mod modules_ext;
pub mod state;
pub mod systems;
pub mod utils;

pub mod ext {
    pub use egui;
}
