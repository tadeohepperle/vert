use crate::{utils::ChillCell, Module};
use slotmap::SlotMap;
use std::{
    any::TypeId,
    collections::HashMap,
    ops::{DerefMut, Index, IndexMut},
};

mod key;
pub use key::{Key, OwnedKey};

pub struct Arenas {
    /// Todo! doing ChillCell + HashMap lookup is absolutely disgusting.
    /// It would be better if could construct something at compile time.
    /// This is just an intermediate solution, to get something working.
    any: ChillCell<HashMap<TypeId, UntypedArena>>,
}

impl Module for Arenas {
    type Config = ();
    type Dependencies = ();

    fn new(_config: Self::Config, _deps: Self::Dependencies) -> anyhow::Result<Self> {
        Ok(Arenas::new())
    }
}

impl Default for Arenas {
    fn default() -> Self {
        Self::new()
    }
}

impl Arenas {
    pub fn new() -> Self {
        Arenas {
            any: ChillCell::new(HashMap::new()),
        }
    }

    pub fn arena<A: 'static + Sized>(&self) -> &Arena<A> {
        self._any_arena_internal::<A>()
    }

    pub fn any_arena_mut<A: 'static + Sized>(&mut self) -> &mut Arena<A> {
        self._any_arena_internal::<A>()
    }

    #[inline]
    pub fn _any_arena_internal<A: 'static + Sized>(&self) -> &mut Arena<A> {
        let type_key = TypeId::of::<A>();
        let arena = self
            .any
            .get_mut()
            .entry(type_key)
            .or_insert_with(|| Arena::<A>::new().into_untyped())
            .typed_mut::<A>();
        arena
    }

    pub fn insert<A: 'static + Sized>(&mut self, value: A) -> OwnedKey<A> {
        let key = self._any_arena_internal::<A>().insert(value);
        OwnedKey(key)
    }

    /// This consumes the OwnedKey, to make it impossible to use it later.
    pub fn remove<A: 'static + Sized>(&mut self, key: OwnedKey<A>) -> Option<A> {
        self._any_arena_internal::<A>().remove(key.0)
    }

    pub fn get_mut<A: 'static + Sized>(&self, key: &OwnedKey<A>) -> &mut A {
        self._any_arena_internal::<A>()
            .get_mut(key.0)
            .expect("owned key resource always present")
    }

    pub fn get<A: 'static + Sized>(&self, key: Key<A>) -> Option<&A> {
        self._any_arena_internal::<A>().get(key)
    }
}

impl<T: 'static + Sized> Index<Key<T>> for Arenas {
    type Output = T;

    fn index(&self, key: Key<T>) -> &Self::Output {
        self._any_arena_internal().get(key).unwrap()
    }
}

// Note: no IndexMut implementation for Key<A> only for OwnedKey<A>

impl<T: 'static + Sized> Index<&OwnedKey<T>> for Arenas {
    type Output = T;

    fn index(&self, key: &OwnedKey<T>) -> &Self::Output {
        self._any_arena_internal().get(key.0).unwrap()
    }
}

impl<T: 'static + Sized> IndexMut<&OwnedKey<T>> for Arenas {
    fn index_mut(&mut self, key: &OwnedKey<T>) -> &mut Self::Output {
        self._any_arena_internal().get_mut(key.0).unwrap()
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
