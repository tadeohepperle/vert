use crate::{Dependencies, Handle, Plugin};

use self::font_cache::FontCache;

pub mod batching;
pub mod billboard;
pub mod font_cache;

#[derive(Debug, Clone, Dependencies)]
pub struct UiDeps {
    pub fonts: Handle<FontCache>,
}

pub struct UiPlugin {}

impl Plugin for UiPlugin {
    fn add(&self, app: &mut crate::AppBuilder) {
        app.add::<FontCache>();
    }
}
