use crate::{elements::Color, Dependencies, Handle, Module};

use self::screen_texture::{DepthTexture, HdrTexture};

use super::{GraphicsContext, Input, Resize, Schedule, Scheduler};
use vert_macros::Dependencies;

mod screen_texture;

#[derive(Dependencies)]
pub struct RendererDependencies {
    scheduler: Handle<Scheduler>,
    graphics: Handle<GraphicsContext>,
    input: Handle<Input>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RendererSettings {
    clear_color: Color,
}

impl Default for RendererSettings {
    fn default() -> Self {
        Self {
            clear_color: Color::new(0.5, 0.3, 0.8),
        }
    }
}

pub const SURFACE_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const HDR_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const MSAA_SAMPLE_COUNT: u32 = 4;

pub struct Renderer {
    ctx: Handle<GraphicsContext>,
    settings: RendererSettings,

    depth_texture: DepthTexture,
    hdr_msaa_texture: HdrTexture,
    hdr_resolve_target: HdrTexture,
}

impl Module for Renderer {
    type Config = RendererSettings;
    type Dependencies = RendererDependencies;

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        todo!()
    }

    fn intialize(handle: crate::Handle<Self>) -> anyhow::Result<()> {
        // register resize handler in input

        Ok(())
    }
}

impl Resize for Renderer {
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u64>) {
        self.depth_texture.recreate(&self.ctx);
        self.hdr_msaa_texture = HdrTexture::create_screen_sized(&self.ctx, MSAA_SAMPLE_COUNT);
        self.hdr_resolve_target = HdrTexture::create_screen_sized(&self.ctx, 1);
    }
}
