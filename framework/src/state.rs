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
    async fn initialize(modules: &Modules) -> anyhow::Result<Self>;

    /// main game logic
    fn update(&mut self, modules: &Modules) -> Flow {
        Flow::Continue
    }

    // called before rendering is done. Perform GPU Updates here.
    fn prepare(&mut self, modules: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {}
}

impl StateT for () {
    async fn initialize(modules: &Modules) -> anyhow::Result<Self> {
        Ok(())
    }
}
