//! batteries are some simple systems that run every frame.

use vert_core::prelude::*;
use vert_core::{arenas::arena::TypedArena, component::Component, reflect};

use crate::modules::Modules;
pub mod simple_cam_controller;

pub use simple_cam_controller::SimpleCamController;

/// a first draft for systems that run each frame.
/// Later we want to introduce more complicated systems and replace batteries, this is just a workaround right now.
pub struct Batteries {
    // always some, just Option to drop it properly
    arena: Option<TypedArena<DynBattery>>,
}

impl Batteries {
    pub fn new() -> Self {
        Batteries {
            arena: Some(TypedArena::new()),
        }
    }

    pub fn update(&mut self, modules: &mut Modules) {
        for (_, b) in self.arena.as_mut().unwrap().iter_mut() {
            b.update(modules);
        }
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {
        for (_, b) in self.arena.as_mut().unwrap().iter_mut() {
            b.prepare(queue, encoder);
        }
    }

    pub fn add<T: Battery>(&mut self, battery: T) {
        // todo! we dont even have a check here if this battery already exists... but anyway soon we will
        // through batteries away and implement proper systems.

        self.arena
            .as_mut()
            .unwrap()
            .insert(DynBattery::new(battery));
    }
}

impl Drop for Batteries {
    fn drop(&mut self) {
        self.arena.take().unwrap().free();
    }
}

reflect!(Battery);
pub trait Battery: 'static {
    fn update(&mut self, modules: &mut Modules);
    fn prepare(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {}
    // todo: explicit order?
}

impl Component for DynBattery {}
reflect!(DynBattery: Battery);
pub struct DynBattery {
    inner: Box<dyn Battery>,
}

impl Battery for DynBattery {
    fn update(&mut self, modules: &mut Modules) {
        self.inner.update(modules);
    }
}

impl DynBattery {
    pub fn new<T: Battery>(battery: T) -> Self {
        DynBattery {
            inner: Box::new(battery),
        }
    }
}
