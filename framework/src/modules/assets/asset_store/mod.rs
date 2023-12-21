use std::{
    marker::PhantomData,
    sync::{LazyLock, Mutex, MutexGuard},
};

use slotmap::{Key as KeyT, KeyData, SlotMap};

use crate::modules::graphics::elements::texture::BindableTexture;

mod key;
pub use key::Key;

static _ASSET_STORE: LazyLock<Mutex<AssetStoreInner>> =
    LazyLock::new(|| Mutex::new(AssetStoreInner::new()));

struct AssetStoreInner {
    textures: SlotMap<Key<BindableTexture>, BindableTexture>,
}

pub struct AssetStore<'a> {
    guard: MutexGuard<'a, AssetStoreInner>,
}

impl<'a> AssetStore<'a> {
    pub fn lock() -> Self {
        AssetStore {
            guard: _ASSET_STORE.lock().expect("_ASSET_STORE poisoned"),
        }
    }
}

impl AssetStoreInner {
    pub fn new() -> Self {
        AssetStoreInner {
            textures: Default::default(),
        }
    }
}

impl<'a> AssetStore<'a> {
    pub fn get_texture(&self, key: Key<BindableTexture>) -> Option<&BindableTexture> {
        self.guard.textures.get(key)
    }

    pub fn store_texture(&mut self, texture: BindableTexture) -> Key<BindableTexture> {
        self.guard.textures.insert(texture)
    }
}
