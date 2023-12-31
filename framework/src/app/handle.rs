use std::{cell::UnsafeCell, fmt::Debug, ops::DerefMut};

use super::{Dependencies, Module, ModuleId};

pub struct Handle<T: Module> {
    pub(super) ptr: &'static UnsafeCell<T>,
}

impl<T: Module> Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle<{}>", ModuleId::of::<T>())
    }
}

impl<T: Module> Handle<T> {
    /// Warning! Use this function carefully.
    pub fn get_mut(&self) -> &'static mut T {
        let reference: &'static mut T = unsafe { &mut *self.ptr.get() };
        reference
    }
}

impl<T: Module> std::ops::Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let reference: &'static T = unsafe { &*self.ptr.get() };
        reference
    }
}

impl<T: Module> DerefMut for Handle<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let reference: &'static mut T = unsafe { &mut *self.ptr.get() };
        reference
    }
}

impl<T: Module> Handle<T> {
    pub fn untyped(&self) -> UntypedHandle {
        UntypedHandle {
            ptr: unsafe { std::mem::transmute(self.ptr) },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UntypedHandle {
    pub(crate) ptr: *const (),
}

impl UntypedHandle {
    pub(crate) fn typed<T: Module>(&self) -> Handle<T> {
        Handle {
            ptr: unsafe { std::mem::transmute(self.ptr) },
        }
    }
}

impl<T: Module> Dependencies for Handle<T> {
    fn type_ids() -> Vec<ModuleId> {
        vec![ModuleId::of::<T>()]
    }

    fn from_untyped_handles(ptrs: &[UntypedHandle]) -> Self {
        assert!(ptrs.len() == 1);
        ptrs[0].typed()
    }
}
