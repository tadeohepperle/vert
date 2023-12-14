use glam::UVec2;
use vert_core::prelude::*;
use wgpu::Color;

reflect!(UiRect:);
impl Component for UiRect {}
#[derive(Debug, Clone, Copy)]
pub struct UiRect {
    min: UVec2,
    max: UVec2,
    color: Color,
}
