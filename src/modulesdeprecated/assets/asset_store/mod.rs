use std::{
    any::TypeId,
    collections::HashMap,
    marker::PhantomData,
    ops::DerefMut,
    sync::{LazyLock, Mutex, MutexGuard},
};

use slotmap::{Key as KeyT, KeyData, SlotMap};

use crate::modules::graphics::elements::texture::BindableTexture;

mod key;
pub use key::Key;

static _ASSET_STORE: LazyLock<Mutex<AssetStoreInner>> =
    LazyLock::new(|| Mutex::new(AssetStoreInner::new()));

struct AssetStoreInner {
    textures: Arena<BindableTexture>,
    fonts: Arena<fontdue::Font>,
    any: HashMap<TypeId, UntypedArena>,
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
            fonts: Default::default(),
            any: Default::default(),
        }
    }
}

pub struct Arena<T: 'static + Sized> {
    inner: SlotMap<Key<T>, T>,
}

/// This is a bit lazy, we want to use our own functions in the future to supports,
/// e.g. static keys that guarantee the user a static reference back and so on. I know, this is a bit difficult due to pinning/moving of the slotmap, now.
impl<T: 'static + Sized> std::ops::Deref for Arena<T> {
    type Target = SlotMap<Key<T>, T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: 'static + Sized> DerefMut for Arena<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: 'static + Sized> Arena<T> {
    pub fn new() -> Self {
        Arena {
            inner: Default::default(),
        }
    }

    fn into_untyped(self) -> UntypedArena {
        unsafe { std::mem::transmute(self) }
    }
}

impl<T: 'static + Sized> Default for Arena<T> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<'a> AssetStore<'a> {
    pub fn textures(&self) -> &Arena<BindableTexture> {
        &self.guard.textures
    }

    pub fn textures_mut(&mut self) -> &mut Arena<BindableTexture> {
        &mut self.guard.textures
    }

    pub fn fonts(&self) -> &Arena<fontdue::Font> {
        &self.guard.fonts
    }

    pub fn fonts_mut(&mut self) -> &mut Arena<fontdue::Font> {
        &mut self.guard.fonts
    }

    // pub fn any_arena<A: 'static + Sized>(&mut self) -> &Arena<A> {
    //     let type_key = TypeId::of::<A>();
    //     let arena = self
    //         .guard
    //         .any
    //         .get(&type_key)
    //         .unwrap_or_else(|| {
    //             self.guard
    //                 .any
    //                 .insert(type_key, Arena::<A>::new().into_untyped());
    //             self.guard.any.get(&type_key).unwrap()
    //         })
    //         .typed::<A>();
    //     arena
    // }

    pub fn any_arena_mut<A: 'static + Sized>(&mut self) -> &mut Arena<A> {
        let type_key = TypeId::of::<A>();
        let arena = self
            .guard
            .any
            .entry(type_key)
            .or_insert_with(|| Arena::<A>::new().into_untyped())
            .typed_mut::<A>();
        arena
    }
}

enum Never {}
struct UntypedArena {
    inner: SlotMap<Key<Never>, Never>,
}

impl UntypedArena {
    fn into_typed<T: 'static + Sized>(self) -> Arena<T> {
        unsafe { std::mem::transmute(self) }
    }

    fn typed<T: 'static + Sized>(&self) -> &Arena<T> {
        unsafe { std::mem::transmute(self) }
    }

    fn typed_mut<T: 'static + Sized>(&mut self) -> &mut Arena<T> {
        unsafe { std::mem::transmute(self) }
    }
}
