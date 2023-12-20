// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

// /////////////////////////////////////////////////////////////////////////////
// Renderer
// /////////////////////////////////////////////////////////////////////////////

pub struct UiRectRenderer {
    pipeline: wgpu::RenderPipeline,
}

// /////////////////////////////////////////////////////////////////////////////
// Data
// /////////////////////////////////////////////////////////////////////////////

use crate::modules::graphics::elements::color::Color;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiRect {
    pub pos: Rect,
    pub uv: Rect,
    pub color: Color,
    pub border_radius: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rect {
    /// min x, min y (top left corner)
    pub offset: [f32; 2],
    /// size x, size y
    pub size: [f32; 2],
}

impl Rect {
    pub const fn new(offset: [f32; 2], size: [f32; 2]) -> Self {
        Self { offset, size }
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            size: [1.0, 1.0],
        }
    }
}
