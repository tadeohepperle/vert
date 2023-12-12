use std::alloc::{self, Layout};
use std::any::type_name;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::{mem, ptr};

/// like a Vec<T> but untyped.
pub(super) struct Blob {
    item_type_name: &'static str,
    item_size: usize,
    item_align: usize,
    ptr: NonNull<u8>,
    cap: usize,
    len: usize,
}
unsafe impl Send for Blob {}
unsafe impl Sync for Blob {}

impl Blob {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn item_type_name(&self) -> &'static str {
        self.item_type_name
    }

    pub fn new<T>() -> Blob {
        let type_name = type_name::<T>();
        let item_size = mem::size_of::<T>();
        let item_align = mem::align_of::<T>();
        Blob {
            item_type_name: type_name,
            item_size,
            item_align,
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }

    #[inline(always)]
    fn layout_for_cap(&self, cap: usize) -> Layout {
        Layout::from_size_align(self.item_size * cap, self.item_align).unwrap()
    }

    fn grow(&mut self) {
        let (new_cap, new_layout) = if self.cap == 0 {
            (1, self.layout_for_cap(1))
        } else {
            let new_cap = 2 * self.cap;
            (new_cap, self.layout_for_cap(new_cap))
        };

        // Ensure that the new allocation doesn't exceed `isize::MAX` bytes.
        assert!(
            new_layout.size() <= isize::MAX as usize,
            "Allocation too large"
        );

        let new_ptr = if self.cap == 0 {
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = self.layout_for_cap(self.cap);
            let old_ptr = self.ptr.as_ptr();
            unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        // If allocation fails, `new_ptr` will be null, in which case we abort.
        self.ptr = match NonNull::new(new_ptr) {
            Some(p) => p,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }

    /// This should be debug only and compiled away in release builds.
    #[inline(always)]
    pub fn assert_t_matches<T>(&self) {
        debug_assert_eq!(type_name::<T>(), self.item_type_name);
        debug_assert_eq!(mem::size_of::<T>(), self.item_size);
        debug_assert_eq!(mem::align_of::<T>(), self.item_align);
    }

    #[inline(always)]
    fn ptr_t<T>(&self) -> *mut T {
        self.ptr.as_ptr() as *mut T
    }

    pub fn push<T>(&mut self, elem: T) {
        self.assert_t_matches::<T>();

        if self.len == self.cap {
            self.grow();
        }

        unsafe {
            ptr::write(self.ptr_t::<T>().add(self.len), elem);
        }

        // Can't fail, we'll OOM first.
        self.len += 1;
    }

    pub fn pop<T>(&mut self) -> Option<T> {
        self.assert_t_matches::<T>();

        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(ptr::read(self.ptr_t::<T>().add(self.len) as *mut T)) }
        }
    }

    pub fn insert<T>(&mut self, index: usize, elem: T) {
        self.assert_t_matches::<T>();

        // Note: `<=` because it's valid to insert after everything
        // which would be equivalent to push.
        assert!(index <= self.len, "index out of bounds");
        if self.cap == self.len {
            self.grow();
        }

        unsafe {
            // ptr::copy(src, dest, len): "copy from src to dest len elems"
            ptr::copy(
                self.ptr_t::<T>().add(index),
                self.ptr_t::<T>().add(index + 1),
                self.len - index,
            );
            ptr::write(self.ptr_t::<T>().add(index), elem);
            self.len += 1;
        }
    }

    pub fn remove<T>(&mut self, index: usize) -> T {
        // Note: `<` because it's *not* valid to remove after everything
        assert!(index < self.len, "index out of bounds");
        unsafe {
            self.len -= 1;
            let result = ptr::read(self.ptr_t::<T>().add(index));
            ptr::copy(
                self.ptr_t::<T>().add(index + 1),
                self.ptr_t::<T>().add(index),
                self.len - index,
            );
            result
        }
    }

    /// Warning!!! This **needs** to be called in orderto not leake resources!!!
    pub fn free<T>(mut self) {
        self.assert_t_matches::<T>();
        if self.cap != 0 {
            while let Some(_) = self.pop::<T>() {}
            let layout = self.layout_for_cap(self.cap);
            unsafe {
                alloc::dealloc(self.ptr.as_ptr(), layout);
            }
        }
    }

    pub fn typed_ref<'a, T>(&'a self) -> TypedBlobRef<'a, T> {
        self.assert_t_matches::<T>();
        TypedBlobRef {
            blob: self,
            phantom: PhantomData,
        }
    }

    pub fn typed_mut<'a, T>(&'a mut self) -> TypedBlobMut<'a, T> {
        self.assert_t_matches::<T>();
        TypedBlobMut {
            blob: self,
            phantom: PhantomData,
        }
    }

    pub fn iter_raw_ptrs<'a>(&'a self) -> RawPtrIter<'a> {
        RawPtrIter::new(self)
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Typed Blobs
// /////////////////////////////////////////////////////////////////////////////

pub struct TypedBlobRef<'a, T> {
    blob: &'a Blob,
    phantom: PhantomData<T>,
}

impl<'a, T: Debug> Debug for TypedBlobRef<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct BlobEntries<'a, T> {
            blob: &'a TypedBlobRef<'a, T>,
        }
        impl<'a, T: Debug> Debug for BlobEntries<'a, T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_list().entries(self.blob.iter()).finish()
            }
        }

        f.debug_struct("TypedBlob")
            .field("type_name", &self.blob.item_type_name)
            .field("item_size", &self.blob.item_size)
            .field("item_align", &self.blob.item_align)
            .field("entries", &BlobEntries { blob: self })
            .finish()
    }
}

impl<'a, T> Deref for TypedBlobRef<'a, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.blob.ptr_t::<T>(), self.blob.len) }
    }
}

pub struct TypedBlobMut<'a, T> {
    blob: &'a mut Blob,
    phantom: PhantomData<T>,
}

use std::ops::DerefMut;

impl<'a, T> Deref for TypedBlobMut<'a, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.blob.ptr_t::<T>(), self.blob.len) }
    }
}

impl<'a, T> DerefMut for TypedBlobMut<'a, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.blob.ptr_t::<T>(), self.blob.len) }
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Iterating over a blob as raw pointers
// /////////////////////////////////////////////////////////////////////////////

/// the pointers are each offset apart from each other, where offset is the item size in the blob.
pub(super) struct RawPtrIter<'a> {
    blob: &'a Blob,
    /// at the beginning this is the same as the ptr of the blob.
    byte_ptr: *const u8,
    /// the remaining length of this iterator is blob.len - elements_done;
    elements_done: usize,
}

impl<'a> RawPtrIter<'a> {
    fn new(blob: &'a Blob) -> RawPtrIter<'a> {
        RawPtrIter {
            blob,
            byte_ptr: blob.ptr.as_ptr(),
            elements_done: 0,
        }
    }
}

impl<'a> Iterator for RawPtrIter<'a> {
    type Item = *const u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.elements_done == self.blob.len {
            return None;
        }
        let ptr = self.byte_ptr;
        self.byte_ptr = unsafe { self.byte_ptr.add(self.blob.item_size) };
        self.elements_done += 1;
        Some(ptr)
    }
}

impl<'a> ExactSizeIterator for RawPtrIter<'a> {
    fn len(&self) -> usize {
        self.blob.len - self.elements_done
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Tests
// /////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::arenas::arena::blob::Blob;

    #[test]
    fn pushing_and_popping() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        struct Human {
            name: String,
            age: u32,
            male: bool,
        }

        let mut blob = Blob::new::<Human>();

        let jeff = Human {
            name: "Jeff".into(),
            age: 340,
            male: true,
        };
        let last_human = Human {
            name: "99".into(),
            age: 99,
            male: false,
        };

        for i in 0..100u32 {
            let human = if i == 42 {
                jeff.clone()
            } else {
                Human {
                    name: i.to_string(),
                    age: i,
                    male: i % 2 == 0,
                }
            };
            blob.push(human);
        }

        let typed_blob = blob.typed_ref::<Human>();
        assert_eq!(&typed_blob[42], &jeff);
        assert_ne!(&typed_blob[43], &jeff);
        assert_eq!(&typed_blob[99], &last_human);
        assert_eq!(blob.pop::<Human>(), Some(last_human));

        blob.free::<Human>();
    }

    /// this test can take about 20 seconds to run.
    #[test]
    fn no_memory_leaks() {
        // if the cleanup does not work, this test will have produced about 50GB of leaked memory -> fails.

        let some_long_string: String = "a".repeat(10000);

        struct S {
            str: String,
        }

        for _test_run in 0..1000 {
            // 1000 times create a blob and each time, push 5000 objects on it, each 10kb bytes.
            let mut blob = Blob::new::<S>();
            for _ in 0..5000 {
                let s = S {
                    str: some_long_string.clone(),
                };
                blob.push(s);
            }
            blob.free::<S>();
            // std::thread::sleep(Duration::from_millis(1));
            // println!("test_run {test_run} successful");
        }
    }
}
