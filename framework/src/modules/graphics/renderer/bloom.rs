use crate::modules::graphics::{elements::texture::Texture, graphics_context::GraphicsContext};

/// The input to the BloomPipeline is an HDR texture A that has a bindgroup.
/// We need to be able to use this texture A as a render attachment.
/// The steps this bloom pipeline takes, each bullet point is one render pass:
///
/// B1 has 1/2 the resolution of the original image, B2 has 1/4 the resolution and so on...
///
/// # 1. Downsampling:
///
/// - threshold and downsample the image, store result in B1
/// - downsample B1 store the result in B2
/// - downsample B2 store the result in B3
/// - downsample B3 store the result in B4
///
/// note: we need to be able to use B1..BX as bindgroups of textures, to sample them in fragment shaders.
/// # 2. Upsampling:
///
/// - upsample B4 and add it to B3
/// - upsample B3 and add it to B2
/// - upsample B2 and add it to B1
/// - upsample B1 and add it to the original HDR image A.
///
/// This should result in a bloom.
pub struct BloomPipeline {
    downsample: wgpu::RenderPipeline,
    upsample: wgpu::RenderPipeline,
}

impl BloomPipeline {
    pub fn new(context: &GraphicsContext) -> Self {
        todo!()
    }

    pub fn resize(context: &GraphicsContext) {}

    pub fn apply_bloom(context: &GraphicsContext, texture: &Texture) {}
}
