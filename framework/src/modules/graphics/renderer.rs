use std::sync::Arc;

use winit::dpi::PhysicalSize;

use super::{elements::texture::Texture, graphics_context::GraphicsContext};

pub struct Renderer {
    context: GraphicsContext,
    depth_texture: Texture,
}

impl Renderer {
    pub async fn initialize(context: GraphicsContext) -> anyhow::Result<Self> {
        let depth_texture = create_depth_texture(&context);
        Ok(Self {
            context,
            depth_texture,
        })
    }

    pub fn resize(&mut self) {
        // recreate the depth texture
        self.depth_texture = create_depth_texture(&self.context);
    }
}

fn create_depth_texture(context: &GraphicsContext) -> Texture {
    let surface_config = context.surface_config.get();
    let depth_texture =
        Texture::create_depth_texture(&context.device, &surface_config, "depth_texture");
    depth_texture
}
