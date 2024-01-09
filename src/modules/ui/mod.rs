use crate::{Dependencies, Handle, Plugin};

use self::{font_cache::FontCache, ui_renderer::UiRenderer};

pub mod batching;
pub mod board;
pub mod font_cache;
pub mod ui_renderer;
pub mod widgets;

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
