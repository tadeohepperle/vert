use std::{fmt::Display, marker::PhantomData};

use slotmap::{Key as KeyT, KeyData};

/// An owned key cannot be cloned or in any way duplicated (except with unsafe of course). it is unique.
///
/// But it can be converted into any number of normal keys that can be passed around.
#[repr(C)]
pub struct OwnedKey<T: 'static + Sized>(pub(super) Key<T>);

impl<T: 'static + Sized> OwnedKey<T> {
    pub fn key(&self) -> Key<T> {
        self.0
    }
}

// only from Owned -> Normal is allowed!
impl<T: 'static + Sized> From<OwnedKey<T>> for Key<T> {
    fn from(value: OwnedKey<T>) -> Self {
        value.0
    }
}

#[repr(C)]
pub struct Key<T: 'static + Sized> {
    value: KeyData,
    phantom: PhantomData<T>,
}

impl<T: 'static + Sized> Key<T> {
    pub fn as_u64(&self) -> u64 {
        unsafe { std::mem::transmute(*self) }
    }
}

impl<T: 'static + Sized> Display for Key<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<T: 'static + Sized> Clone for Key<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            phantom: self.phantom,
        }
    }
}

impl<T: 'static + Sized> Copy for Key<T> {}

impl<T: 'static + Sized> std::fmt::Debug for Key<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Key").field("value", &self.value).finish()
    }
}

impl<T: 'static + Sized> std::hash::Hash for Key<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: 'static + Sized> PartialEq for Key<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: 'static + Sized> PartialOrd for Key<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: 'static + Sized> Default for Key<T> {
    fn default() -> Self {
        Self {
            value: Default::default(),
            phantom: Default::default(),
        }
    }
}

impl<T: 'static + Sized> Eq for Key<T> {}

impl<T: 'static + Sized> Ord for Key<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

unsafe impl<T: 'static + Sized> KeyT for Key<T> {
    fn data(&self) -> KeyData {
        self.value
    }
}
impl<T: 'static + Sized> From<KeyData> for Key<T> {
    fn from(value: KeyData) -> Self {
        Key {
            value,
            phantom: PhantomData,
        }
    }
}
