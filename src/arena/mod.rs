use std::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    iter::{self, Enumerate},
    marker::PhantomData,
    slice::{Iter, IterMut},
};

use self::blob::{Blob, TypedBlobMut, TypedBlobRef};

mod blob;
pub mod singleton_blob;
// can index into an Arena
#[derive(Debug, Clone, Copy)]
pub struct ArenaIndex {
    index: usize,
    generation: Generation,
}

pub type Generation = u64;

/// Important! Do not change the #[repr(C, u8, align(8))].
/// We rely on skipping over 16 bytes (8 for tag and 8 for generation) to get to value: T.
#[repr(C, u8, align(8))]
#[derive(Clone, Debug)]
enum Entry<T> {
    Free { next_free: Option<usize> } = 0,
    Occupied { gen: Generation, value: T } = 1,
}
impl<T> Entry<T> {
    fn is_occupied_by(&self, generation: Generation) -> bool {
        matches!(&self, Entry::Occupied { gen, .. } if *gen == generation)
    }

    fn is_free(&self) -> bool {
        matches!(&self, Entry::Free { .. })
    }
}

/// an arena holds a type erazed blob in memory, that can be indexed by an ArenaIndex.
/// Each entry is either filled or empty. Filled entries are reused when new entities come it.
///
/// Shamelessly inspired by https://docs.rs/generational-arena/latest/generational_arena/
pub struct Arena {
    blob: Blob,
    generation: Generation,
    free_list_head: Option<usize>,
    len: usize,
}

impl Debug for Arena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arena")
            .field("type_name", &self.blob.item_type_name())
            .field("len", &self.len)
            .finish()
    }
}

impl Arena {
    pub fn new<T>() -> Arena {
        Arena {
            blob: Blob::new::<Entry<T>>(),
            generation: 0,
            free_list_head: None,
            len: 0,
        }
    }

    pub fn with_capacity<T>(cap: usize) -> Arena {
        let mut arena = Arena {
            blob: Blob::new::<Entry<T>>(),
            generation: 0,
            free_list_head: None,
            len: 0,
        };
        arena.reserve::<T>(cap);
        arena
    }
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    fn blob_mut<'a, T>(&'a mut self) -> &'a mut TypedBlobMut<'a, Entry<T>> {
        self.assert_t_matches::<T>();
        let local_lifetime = &mut self.blob.typed_mut::<Entry<T>>();
        // do this trick again, smh:
        let blob_mut: &'a mut TypedBlobMut<'a, Entry<T>> =
            unsafe { &mut *(local_lifetime as *mut TypedBlobMut<'a, Entry<T>>) };
        blob_mut
    }

    #[inline(always)]
    fn blob_ref<'a, T>(&'a self) -> &'a TypedBlobRef<'a, Entry<T>> {
        self.assert_t_matches::<T>();
        let local_lifetime = &self.blob.typed_ref::<Entry<T>>();
        // do this trick again, smh:
        let parent_lifetime: &'a TypedBlobRef<'a, Entry<T>> =
            unsafe { &*(local_lifetime as *const TypedBlobRef<'a, Entry<T>>) };
        parent_lifetime
    }

    #[inline(always)]
    pub fn assert_t_matches<T>(&self) {
        self.blob.assert_t_matches::<Entry<T>>();
    }

    /// allocate space for additional_capacity more elements in the arena.
    pub fn reserve<T>(&mut self, additional_capacity: usize) {
        self.assert_t_matches::<T>();

        let start = self.blob.len();
        let end = self.blob.len() + additional_capacity;
        let old_head = self.free_list_head;

        for i in start..end {
            let next_free = if i == end - 1 { old_head } else { Some(i + 1) };
            let free_entry: Entry<T> = Entry::Free { next_free };
            self.blob.push::<Entry<T>>(free_entry);
        }

        self.free_list_head = Some(start);
    }

    #[inline]
    pub fn insert<T>(&mut self, value: T) -> ArenaIndex {
        match self.try_insert_into_inner_empty_slot(value) {
            Ok(i) => i,
            Err(value) => self.insert_with_expanding(value),
        }
    }

    #[inline]
    pub fn try_insert_into_inner_empty_slot<T>(&mut self, value: T) -> Result<ArenaIndex, T> {
        self.assert_t_matches::<T>();

        match self.try_find_inner_empty_slot::<T>() {
            None => Err(value),
            Some(index) => {
                let generation = self.generation;
                let mut_blob = self.blob_mut::<T>();
                mut_blob[index.index] = Entry::Occupied {
                    gen: generation,
                    value,
                };
                Ok(index)
            }
        }
    }

    #[inline]
    fn try_find_inner_empty_slot<T>(&mut self) -> Option<ArenaIndex> {
        self.assert_t_matches::<T>();

        match self.free_list_head {
            None => None,
            Some(i) => match self.blob.typed_ref::<Entry<T>>()[i] {
                Entry::Occupied { .. } => panic!("corrupt free list"),
                Entry::Free { next_free } => {
                    self.free_list_head = next_free;
                    self.len += 1;
                    Some(ArenaIndex {
                        index: i,
                        generation: self.generation,
                    })
                }
            },
        }
    }

    #[inline(never)]
    fn insert_with_expanding<T>(&mut self, value: T) -> ArenaIndex {
        self.assert_t_matches::<T>();

        // this essentially doubles the size of the arena. (Except when zero, then it just adds one element.)
        let additional_capacity = self.blob.len().max(1);
        self.reserve::<T>(additional_capacity);

        self.try_insert_into_inner_empty_slot::<T>(value)
            .map_err(|_| ())
            .expect("inserting will always succeed after reserving additional space")
    }

    pub fn remove<T>(&mut self, i: ArenaIndex) -> Option<T> {
        self.assert_t_matches::<T>();

        if i.index >= self.blob.len() {
            return None;
        }

        let free_list_head = self.free_list_head;
        let blob_mut = self.blob_mut::<T>();
        let entry_at_index = &mut blob_mut[i.index];

        // if the entry is free, or filled but generation does not match, return None.

        if !entry_at_index.is_occupied_by(i.generation) {
            return None;
        }
        let new_free_entry_at_index = Entry::Free {
            next_free: free_list_head,
        };
        let entry = std::mem::replace(entry_at_index, new_free_entry_at_index);
        self.generation += 1;
        self.free_list_head = Some(i.index);
        self.len -= 1;

        // switch the entry out for a free entry:

        let removed_value = match entry {
            Entry::Occupied { gen: _, value } => value,
            _ => unreachable!(), // because checked before.
        };

        Some(removed_value)
    }

    pub fn contains<T>(&self, i: ArenaIndex) -> bool {
        self.get::<T>(i).is_some()
    }

    pub fn get<'a, T>(&'a self, i: ArenaIndex) -> Option<&'a T> {
        match self.blob_ref::<T>().get(i.index) {
            Some(Entry::Occupied {
                gen: ref generation,
                value,
            }) if *generation == i.generation => {
                // should be safe
                let e: &'a T = unsafe { &*(value as *const T) };
                Some(e)
            }
            _ => None,
        }
    }

    pub fn get_mut<'a, T>(&'a mut self, i: ArenaIndex) -> Option<&'a mut T> {
        match self.blob_mut::<T>().get_mut(i.index) {
            Some(Entry::Occupied {
                gen: ref generation,
                value,
            }) if *generation == i.generation => {
                // should be safe
                let e: &'a mut T = unsafe { &mut *(value as *mut T) };
                Some(e)
            }
            _ => None,
        }
    }

    pub fn retain<T>(&mut self, mut predicate: impl FnMut(ArenaIndex, &mut T) -> bool) {
        self.assert_t_matches::<T>();

        for i in 0..self.blob.len() {
            let blob_mut = self.blob_mut::<T>();
            let entry = &mut blob_mut[i];

            let remove = match entry {
                Entry::Occupied {
                    gen: ref generation,
                    value,
                } => {
                    let index = ArenaIndex {
                        index: i,
                        generation: *generation,
                    };
                    if predicate(index, value) {
                        None
                    } else {
                        Some(index)
                    }
                }

                _ => None,
            };
            if let Some(index) = remove {
                self.remove::<T>(index);
            }
        }
    }

    fn iter<'a, T>(&'a self) -> ArenaIter<'a, T> {
        let blob: &'a TypedBlobRef<'a, Entry<T>> = self.blob_ref::<T>();
        let iter: Enumerate<Iter<'a, Entry<T>>> = blob.iter().enumerate();
        ArenaIter {
            len: self.len(),
            iter,
        }
    }

    fn iter_mut<'a, T>(&'a mut self) -> ArenaIterMut<'a, T> {
        let len = self.len();
        let blob = self.blob_mut::<T>();
        let iter: Enumerate<IterMut<'a, Entry<T>>> = blob.iter_mut().enumerate();
        ArenaIterMut { len, iter }
    }

    pub fn into_typed<T>(self) -> TypedArena<T> {
        self.assert_t_matches::<T>();
        TypedArena {
            arena: self,
            phantom: PhantomData,
        }
    }

    pub fn iter_raw_ptrs<'a>(&'a self) -> RawPtrIter<'a> {
        RawPtrIter {
            iter: self.blob.iter_raw_ptrs(),
        }
    }
}

// impl Drop for Arena {
//     fn drop(&mut self) {
//         panic!(
//             "Untyped Arena dropped!
// This is illegal, because the objects in the blob will not be dropped properly and leak memory.
// Blob::free::<T>() needs to be called on the blob. This is done in the drop implementation of TypedArena<T>.
// So convert the Untyped Arena into a TypedArena<T> first, before dropping it. Dropping without knowning the type T
// is not possible because the T's stored in the arena could leak memory if their Drop implementation is not called."
//         )
//     }
// }

impl<T> Borrow<TypedArena<T>> for Arena {
    fn borrow(&self) -> &TypedArena<T> {
        self.assert_t_matches::<T>();
        let ptr_to_self = self as *const Arena;
        let imagine_it_was_typed = ptr_to_self as *const TypedArena<T>;
        unsafe { &*imagine_it_was_typed }
    }
}

impl<T> BorrowMut<TypedArena<T>> for Arena {
    fn borrow_mut(&mut self) -> &mut TypedArena<T> {
        self.assert_t_matches::<T>();
        let ptr_to_self = self as *mut Arena;
        let imagine_it_was_typed = ptr_to_self as *mut TypedArena<T>;
        unsafe { &mut *imagine_it_was_typed }
    }
}

pub struct TypedArena<T> {
    arena: Arena,
    phantom: PhantomData<T>,
}

impl<T> TypedArena<T> {
    pub fn new() -> TypedArena<T> {
        TypedArena {
            arena: Arena::new::<T>(),
            phantom: PhantomData,
        }
    }

    pub fn with_capacity(cap: usize) -> TypedArena<T> {
        TypedArena {
            arena: Arena::with_capacity::<T>(cap),
            phantom: PhantomData,
        }
    }
    pub fn into_untyped(self) -> Arena {
        self.arena
    }
    pub fn get(&self, i: ArenaIndex) -> Option<&T> {
        self.arena.get::<T>(i)
    }
    pub fn get_mut(&mut self, i: ArenaIndex) -> Option<&mut T> {
        self.arena.get_mut::<T>(i)
    }
    pub fn remove(&mut self, i: ArenaIndex) -> Option<T> {
        self.arena.remove::<T>(i)
    }
    pub fn insert(&mut self, e: T) -> ArenaIndex {
        self.arena.insert::<T>(e)
    }
    pub fn retain(&mut self, predicate: impl FnMut(ArenaIndex, &mut T) -> bool) {
        self.arena.retain::<T>(predicate);
    }
    pub fn len(&self) -> usize {
        self.arena.len
    }

    pub fn iter<'a>(&'a self) -> ArenaIter<'a, T> {
        self.arena.iter()
    }

    pub fn iter_mut<'a>(&'a mut self) -> ArenaIterMut<'a, T> {
        self.arena.iter_mut()
    }

    pub fn free(self) {
        self.arena.blob.free::<T>();
    }
}

impl<T: Debug> Debug for TypedArena<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let blob = self.arena.blob_ref::<T>();
        f.debug_struct("TypedArena").field("arena", blob).finish()
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Iterators
// /////////////////////////////////////////////////////////////////////////////

pub struct ArenaIter<'a, T> {
    len: usize,
    iter: iter::Enumerate<Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for ArenaIter<'a, T> {
    type Item = (ArenaIndex, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some((_, &Entry::Free { .. })) => continue,
                Some((
                    index,
                    &Entry::Occupied {
                        gen: generation,
                        ref value,
                    },
                )) => {
                    self.len -= 1;
                    let idx = ArenaIndex { index, generation };
                    return Some((idx, value));
                }
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for ArenaIter<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

pub struct ArenaIterMut<'a, T> {
    len: usize,
    iter: iter::Enumerate<IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for ArenaIterMut<'a, T> {
    type Item = (ArenaIndex, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some((_, &mut Entry::Free { .. })) => continue,
                Some((
                    index,
                    &mut Entry::Occupied {
                        gen: generation,
                        ref mut value,
                    },
                )) => {
                    self.len -= 1;
                    let idx = ArenaIndex { index, generation };
                    return Some((idx, value));
                }
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for ArenaIterMut<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

/// iterates over the n elements of type T stored,
/// skipping over empty entries. For each T,
/// just a raw *const u8 pointer to the first byte of T is returned.
pub struct RawPtrIter<'a> {
    iter: blob::RawPtrIter<'a>,
}

impl<'a> Iterator for RawPtrIter<'a> {
    type Item = *const u8;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next_ptr = self.iter.next()?;
            // memory layout of the Entry is like this:
            //          |   8b   |   8b   |   8b   |
            // Free:     Free____|SomeNone|Nextfree|.....
            // Occupied: Occupied|gggggggg|T.........

            // check if this entry is free, if so hop to next pointer:
            let tag_value = unsafe { *next_ptr };
            if tag_value == 0 {
                continue;
            }
            // jump 16 bytes ahead to our actual data:
            let t_ptr = unsafe { next_ptr.add(16) };
            return Some(t_ptr);
        }
    }
}

// /////////////////////////////////////////////////////////////////////////////
// tests
// /////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::TypedArena;

    #[test]
    fn basic_functionality() {
        let mut arena: TypedArena<String> = TypedArena::new();
        arena.insert("Hans".into());
        let goes = arena.insert("goes".into());
        arena.insert("Home".into());
        arena.remove(goes);
        arena.insert("owns a".into());

        // should print: Hans has a home.
        for (ai, name) in arena.iter() {
            println!("{ai:?}   {name}")
        }

        arena.retain(|_, e| !e.starts_with("H"));

        assert_eq!(arena.iter().len(), 1);
    }
}
