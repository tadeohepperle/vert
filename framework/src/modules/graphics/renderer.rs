use std::sync::Arc;

use winit::dpi::PhysicalSize;

use super::{graphics_context::GraphicsContext, texture::Texture};

pub struct Renderer {
    graphics_context: Arc<GraphicsContext>,
    // depth_texture: Texture,
}

impl Renderer {
    pub async fn initialize(graphics_context: Arc<GraphicsContext>) -> anyhow::Result<Self> {
        Ok(Self {
            graphics_context,
            // depth_texture: todo!(),
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        // todo!
        // self.depth_texture = Texture::create_depth_texture(
        //     &self.graphics_context.device,
        //     self.graphics_context.surface_config(),
        //     "depth_texture",
        // );
    }
}
