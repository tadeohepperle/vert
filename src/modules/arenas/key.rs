use std::{fmt::Display, marker::PhantomData};

use slotmap::{Key as KeyT, KeyData};
pub struct Key<T: 'static + Sized> {
    value: KeyData,
    phantom: PhantomData<T>,
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
