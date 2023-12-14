use wgpu::{CommandEncoder, Queue};

use crate::{
    flow::Flow,
    modules::{
        graphics::graphics_context::{self, GraphicsContext},
        Modules,
    },
};

/// User defined application state
pub trait StateT: Sized {
    #[allow(async_fn_in_trait)]
    async fn initialize(modules: &mut Modules) -> anyhow::Result<Self>;

    /// main game logic
    /// todo!() it would be better to have &Modules with interior mutibility.
    fn update(&mut self, modules: &mut Modules) -> Flow {
        Flow::Continue
    }

    // called before rendering is done. Perform GPU Updates here.
    fn prepare(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {}

    // todo!() implement on shutdown e.g. for saving game state.
}

impl StateT for () {
    async fn initialize(modules: &mut Modules) -> anyhow::Result<Self> {
        Ok(())
    }
}
