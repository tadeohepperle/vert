use std::{
    any::type_name,
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
    mem,
    ptr::NonNull,
};

#[derive(Debug)]
#[repr(C)]
pub struct SingletonBlob {
    data: NonNull<u8>,
    item_type_name: &'static str,
    item_size: usize,
    item_align: usize,
}

impl SingletonBlob {
    pub fn new<T>(t: T) -> SingletonBlob {
        let type_name = type_name::<T>();
        let item_size = mem::size_of::<T>();
        let item_align = mem::align_of::<T>();
        let data: NonNull<u8> = unsafe { std::mem::transmute(Box::new(t)) };
        SingletonBlob {
            item_type_name: type_name,
            item_size,
            item_align,
            data,
        }
    }

    pub fn get<T>(&self) -> &T {
        unsafe { std::mem::transmute(self.data) }
    }

    pub fn get_mut<T>(&self) -> &mut T {
        unsafe { std::mem::transmute(self.data) }
    }

    pub fn free<T>(self) -> T {
        let t: Box<T> = unsafe { std::mem::transmute(self.data) };
        *t
    }

    #[inline(always)]
    pub fn assert_t_matches<T>(&self) {
        debug_assert_eq!(type_name::<T>(), self.item_type_name);
        debug_assert_eq!(mem::size_of::<T>(), self.item_size);
        debug_assert_eq!(mem::align_of::<T>(), self.item_align);
    }

    pub fn into_typed<T>(self) -> TypedSingletonBlob<T> {
        self.assert_t_matches::<T>();
        TypedSingletonBlob {
            inner: self,
            phantom: PhantomData,
        }
    }
}

impl<T> Borrow<TypedSingletonBlob<T>> for SingletonBlob {
    fn borrow(&self) -> &TypedSingletonBlob<T> {
        self.assert_t_matches::<T>();
        let ptr_to_self = self as *const SingletonBlob;
        let imagine_it_was_typed = ptr_to_self as *const TypedSingletonBlob<T>;
        unsafe { &*imagine_it_was_typed }
    }
}

impl<T> BorrowMut<TypedSingletonBlob<T>> for SingletonBlob {
    fn borrow_mut(&mut self) -> &mut TypedSingletonBlob<T> {
        self.assert_t_matches::<T>();
        let ptr_to_self = self as *mut SingletonBlob;
        let imagine_it_was_typed = ptr_to_self as *mut TypedSingletonBlob<T>;
        unsafe { &mut *imagine_it_was_typed }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct TypedSingletonBlob<T> {
    inner: SingletonBlob,
    phantom: PhantomData<T>,
}

impl<T> TypedSingletonBlob<T> {
    pub fn get(&self) -> &T {
        self.inner.get()
    }

    pub fn get_mut(&self) -> &T {
        self.inner.get_mut()
    }

    pub fn free(self) -> T {
        self.inner.free::<T>()
    }
}
