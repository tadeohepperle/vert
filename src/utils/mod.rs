use std::{
    borrow::Borrow,
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub mod timing_queue;
pub use timing_queue::{EntryKey, Timing, TimingQueue};
pub mod watcher;

/// Returns the file location of a .wgsl file with the same name as the .rs file, this was invoked in.
#[macro_export]
macro_rules! wgsl_file {
    () => {{
        // drop the rs, add wgsl
        let mut wgsl_file = format!("./{}", file!()).replace("./framework", ".");
        // replace because we want it to not be from the workspace parent folder.
        wgsl_file.pop();
        wgsl_file.pop();
        wgsl_file.push_str("wgsl");
        wgsl_file
    }};
}

/// Returns the next _^2 number such that it is greater or euqual to n.
/// Is at least 2.
pub fn next_pow2_number(mut n: usize) -> usize {
    let mut e = 2;
    loop {
        if e >= n {
            return e;
        }
        e *= 2;
    }
}

/// Thin wrapper around UnsafeCell to make it less annoying.
///
/// Like RefCell but we don't keep count of borrowing, so it is a bit more unsafe, but free.
#[derive(Debug)]
pub struct YoloCell<T> {
    _inner: UnsafeCell<T>,
}

impl<T> YoloCell<T> {
    pub const fn new(value: T) -> Self {
        YoloCell {
            _inner: UnsafeCell::new(value),
        }
    }

    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self._inner.get() }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self._inner.get() }
    }
}

impl<T> Deref for YoloCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self._inner.get() }
    }
}

impl<T> DerefMut for YoloCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self._inner.get_mut()
    }
}
