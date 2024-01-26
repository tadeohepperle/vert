use std::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

/// Has one owner, cannot be cloned.
#[derive(Debug)]
pub struct OwnedPtr<T> {
    _inner: Box<T>,
}

impl<T> OwnedPtr<T> {
    /// todo! add `new_in` custom allocator
    pub fn new(value: T) -> Self {
        OwnedPtr {
            _inner: Box::new(value),
        }
    }

    /// Warning! The Own<T> will be deallocated when dropped. Manually make sure all Ref<T> given out to it are not around anymore by then.
    #[inline]
    pub fn ptr(&self) -> Ptr<T> {
        Ptr {
            _inner: unsafe { std::mem::transmute(&*self._inner) },
        }
    }
}

impl<T> Deref for OwnedPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self._inner
    }
}

impl<T> DerefMut for OwnedPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self._inner
    }
}

impl<T> Borrow<T> for OwnedPtr<T> {
    fn borrow(&self) -> &T {
        &self._inner
    }
}

impl<T> BorrowMut<T> for OwnedPtr<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self._inner
    }
}

/// Ref is basically a raw pointer. When using it be very careful, that the backing memory is still around.
/// A common patterns should be to get one Own<T> at the start of the program or a Ref from Ref::eternal.
/// Then you give out as many Refs as you want but make sure that they are no longer in circulation when you end the program / go to the next stage.
///
/// Better than lifetime hell in games.
#[derive(Debug)]
pub struct Ptr<T> {
    _inner: NonNull<T>,
}

use std::hash::Hash;
impl<T> Hash for Ptr<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self._inner.hash(state);
    }
}

impl<T> Borrow<T> for Ptr<T> {
    fn borrow(&self) -> &T {
        unsafe { &*self._inner.as_ptr() }
    }
}

impl<T> Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self._inner.as_ptr() }
    }
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Self {
            _inner: self._inner.clone(),
        }
    }
}

unsafe impl<T> Send for Ptr<T> {}

unsafe impl<T> Sync for Ptr<T> {}

impl<T> Copy for Ptr<T> {}

impl<T> Ptr<T> {
    pub fn eternal(value: T) -> Ptr<T> {
        let reference = Box::leak(Box::new(value));
        let _inner = NonNull::from(reference);
        Ptr { _inner }
    }

    pub fn as_u64_hash(&self) -> u64 {
        self._inner.as_ptr() as u64
    }
}

// checks if they are pointing to the same thing
impl<T> PartialEq for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        self._inner.as_ptr() == self._inner.as_ptr()
    }
}

impl<T> PartialOrd for Ptr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self._inner.as_ptr().partial_cmp(&other._inner.as_ptr())
    }
}

impl<T> Ord for Ptr<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self._inner.as_ptr().cmp(&other._inner.as_ptr())
    }
}

impl<T> Eq for Ptr<T> {}
