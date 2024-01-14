use super::{Handle, Module, ModuleId, UntypedHandle};

/// Combines a Handle to a module with a function that can be called on that module.
#[derive(Debug)]
#[allow(dead_code)]
pub struct FunctionHandle<E> {
    module_id: ModuleId,
    handle: UntypedHandle,
    func: fn(*const (), event: E) -> (),
}

impl<E> FunctionHandle<E> {
    pub fn new<M: Module>(handle: Handle<M>, func: fn(&mut M, event: E) -> ()) -> Self {
        FunctionHandle {
            module_id: ModuleId::of::<M>(),
            handle: handle.untyped(),
            func: unsafe { std::mem::transmute(func) },
        }
    }

    #[inline(always)]
    pub fn call(&self, e: E) {
        let ptr = self.handle.ptr();
        (self.func)(ptr, e);
    }
}

/// Like FunctionHandle but a reference of the event gets passed into the function instead of the event itself.
pub struct RefFunctionHandle<E> {
    module_id: ModuleId,
    handle: UntypedHandle,
    func: fn(*const (), event: &E) -> (),
}

impl<E> RefFunctionHandle<E> {
    pub fn new<M: Module>(handle: Handle<M>, func: fn(&mut M, event: &E) -> ()) -> Self {
        RefFunctionHandle {
            module_id: ModuleId::of::<M>(),
            handle: handle.untyped(),
            func: unsafe { std::mem::transmute(func) },
        }
    }

    #[inline(always)]
    pub fn call(&self, e: &E) {
        let ptr = self.handle.ptr();
        (self.func)(ptr, e);
    }
}

/// Combines a Handle to a module with a function that can be called on that module.
pub struct VoidFunctionHandle {
    module_id: ModuleId,
    handle: UntypedHandle,
    func: fn(*const ()) -> (),
}

impl VoidFunctionHandle {
    pub fn new<M: Module>(handle: Handle<M>, func: fn(&mut M) -> ()) -> Self {
        VoidFunctionHandle {
            module_id: ModuleId::of::<M>(),
            handle: handle.untyped(),
            func: unsafe { std::mem::transmute(func) },
        }
    }

    #[inline(always)]
    pub fn call(&self) {
        let ptr = self.handle.ptr();
        (self.func)(ptr);
    }
}
