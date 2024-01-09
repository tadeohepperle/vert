use std::{cell::UnsafeCell, fmt::Debug, ops::DerefMut};

use crate::elements::BindableTexture;

use super::{Dependencies, Module, ModuleId};

/// Warning: The Clone and Copy impls may be removed in the future? Are they safe to expose like that?
pub struct Handle<T: Module> {
    pub(super) ptr: &'static UnsafeCell<T>,
}

impl<T: Module> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T: Module> Copy for Handle<T> {}

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

    pub fn clone(&self) -> Self {
        Handle { ptr: self.ptr }
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
    pub ptr: *const (),
}

impl UntypedHandle {
    #[inline]
    pub(crate) fn typed<T: Module>(&self) -> Handle<T> {
        Handle {
            ptr: unsafe { std::mem::transmute(self.ptr) },
        }
    }

    #[inline]
    pub fn ptr(&self) -> *const () {
        self.ptr
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
