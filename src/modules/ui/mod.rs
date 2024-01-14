use crate::{Dependencies, Handle, Plugin};

use self::{font_cache::FontCache, ui_renderer::UiRenderer};

pub mod batching;
mod board;
pub use board::{
    Align, Axis, Board, BoardInput, BoardPhase, BorderRadius, ContainerId, Div, DivStyle,
    DivTexture, HotActive, Id, Len, MainAlign, Padding, Text,
};

pub mod font_cache;
pub mod ui_renderer;
mod widgets;
pub use widgets::{h_fill, next_hot_active, v_fill, Button, Slider, Widget};

#[derive(Debug, Clone, Dependencies)]
pub struct UiDeps {
    pub fonts: Handle<FontCache>,
    pub ui_renderer: Handle<UiRenderer>,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn add(&self, app: &mut crate::AppBuilder) {
        app.add::<FontCache>();
        app.add::<UiRenderer>();
    }
}
