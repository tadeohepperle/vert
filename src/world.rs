use std::{
    any::TypeId,
    borrow::{Borrow, BorrowMut},
    collections::hash_map::Entry,
    fmt::Debug,
    mem::size_of,
};

use std::collections::HashMap;

use smallvec::{smallvec, SmallVec};

use crate::{
    arena::{Arena, ArenaIndex, TypedArena},
    component::Component,
    // trait_companion::{MultiTraitCompanion, Reflectable, VTablePointer, VTablePointerWithMetadata},
    trait_reflection::{DynTrait, Implementor, VTablePtr, VTablePtrWithMeta},
};

/// W is some user defined world state. Aka global resources
pub struct World<W> {
    pub state: W,
    pub arenas: Arenas,
}

impl<W> World<W> {
    pub fn state(&self) -> &W {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut W {
        &mut self.state
    }

    pub fn arenas(&self) -> &Arenas {
        &self.arenas
    }

    pub fn arenas_mut(&mut self) -> &mut Arenas {
        &mut self.arenas
    }

    pub fn new(world_state: W) -> Self {
        World {
            state: world_state,
            arenas: Arenas::new(),
        }
    }

    pub fn spawn<C: Component>(&mut self, component: C) -> ArenaIndex {
        self.arenas.insert(component)
    }
}

type ArenaAddress = TypeId;

pub struct DynTraitImplementors {
    dyn_trait_id: TypeId,
    dyn_trait_name: &'static str,
    arena_vtables: SmallVec<[(ArenaAddress, VTablePtr); 8]>,
}

pub struct Arenas {
    pub arenas: HashMap<TypeId, Arena>,
    // maps a dyn_trait_id to the arena addresses and vtablepointers of all components that implement this trait
    pub dyn_traits: HashMap<TypeId, DynTraitImplementors>,
}

fn arena_address<C: Component>() -> ArenaAddress {
    C::id()
}

impl Arenas {
    pub fn new() -> Self {
        Arenas {
            arenas: HashMap::new(),
            dyn_traits: HashMap::new(),
        }
    }

    fn get_arena<'a, C: Component>(&'a self) -> Option<&'a TypedArena<C>> {
        let arena = self.arenas.get(&arena_address::<C>())?;
        Some(Borrow::borrow(arena))
    }

    fn get_arena_mut_or_insert<C: Component>(&mut self) -> &mut TypedArena<C> {
        let arena_address = arena_address::<C>();
        let arena = match self.arenas.entry(arena_address) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(vacant) => {
                // dbg!("insert the vtable for all traits:");
                // set the vtable pointers:

                let dyn_traits = unsafe { C::dyn_traits() };
                // dbg!(&dyn_traits);
                for v in dyn_traits {
                    let implementors = self.dyn_traits.entry(v.dyn_trait_id).or_insert_with(|| {
                        DynTraitImplementors {
                            dyn_trait_id: v.dyn_trait_id,
                            dyn_trait_name: v.dyn_trait_name,
                            arena_vtables: smallvec![],
                        }
                    });

                    // this arena should not be part of the arena vtables already:
                    assert!(!implementors
                        .arena_vtables
                        .iter()
                        .any(|e| e.0 == arena_address));

                    implementors.arena_vtables.push((arena_address, v.ptr));
                }
                // dbg!("after");
                // create a new arena:
                let arena = TypedArena::<C>::new();
                // dbg!("created arena");
                vacant.insert(arena.into_untyped())
            }
        };
        BorrowMut::borrow_mut(arena)
    }

    fn get_arena_mut<C: Component>(&mut self) -> Option<&mut TypedArena<C>> {
        let arena = self.arenas.get_mut(&arena_address::<C>())?;
        Some(BorrowMut::borrow_mut(arena))
    }

    pub fn insert<C: Component>(&mut self, component: C) -> ArenaIndex {
        // todo! if the first one, setup state for this component type
        let arena = self.get_arena_mut_or_insert::<C>();
        arena.insert(component)
    }

    pub fn get<C: Component>(&self, i: ArenaIndex) -> Option<&C> {
        let arena = self.get_arena::<C>()?;
        arena.get(i)
    }

    pub fn get_mut<C: Component>(&mut self, i: ArenaIndex) -> Option<&mut C> {
        let arena = self.get_arena_mut()?;
        arena.get_mut(i)
    }

    pub fn remove<C: Component>(&mut self, i: ArenaIndex) -> Option<C> {
        let arena = self.get_arena_mut()?;
        let removed_value = arena.remove(i);
        // if this component was the last one in this arena, remove the arena to not use up more space:
        if arena.len() == 0 {
            let arena_address = arena_address::<C>();
            // remove arena:
            let arena = self
                .arenas
                .remove(&arena_address)
                .expect("Arena is present, because it was just queried");

            // remove arena vtable pointers:
            for v in unsafe { C::dyn_traits() } {
                let implementors = self
                    .dyn_traits
                    .get_mut(&v.dyn_trait_id)
                    .expect("dyn traits should contain this arena");

                // this arena should be part of the arena vtables already:
                assert!(implementors
                    .arena_vtables
                    .iter()
                    .any(|e| e.0 == arena_address && e.1 == v.ptr));
                implementors.arena_vtables.retain(|e| e.0 != arena_address);
            }

            // drop arena:
            // this triggers the Blob::drop_t<C>() function that triggers the drop of all components stored in the blob e.g. Strings that need to free memory, vecs, etc...
            arena.into_typed::<C>().drop_the_blob();
        }
        // remove the vtable pointers for this area from the
        removed_value
    }

    pub fn iter<C: Component>(&self) -> impl Iterator<Item = (ArenaIndex, &C)> {
        self.get_arena::<C>()
            .map(|e| e.iter())
            .into_iter()
            .flatten()
    }
    pub fn iter_mut<C: Component>(&mut self) -> impl Iterator<Item = (ArenaIndex, &mut C)> {
        self.get_arena_mut::<C>()
            .map(|e| e.iter_mut())
            .into_iter()
            .flatten()
    }

    pub fn implementors<T: DynTrait + ?Sized>(&self) -> SmallVec<[(TypeId, VTablePtr); 8]> {
        let dyn_trait_id = T::id();
        let implementors = self
            .dyn_traits
            .get(&dyn_trait_id)
            .map(|e| &e.arena_vtables)
            .cloned() // cloned not ideal, but okay.
            .unwrap_or_default();
        implementors
    }

    pub fn trait_iter<'a, T: DynTrait + ?Sized>(&'a self) -> impl Iterator<Item = &'a T> {
        self.implementors::<T>()
            .into_iter()
            .flat_map(|(c_id, v_table_ptr)| {
                let arena = self
                    .arenas
                    .get(&c_id)
                    .expect("Arena that is registered in dyn_traits not found!");
                arena.iter_raw_ptrs().map(move |data_ptr| {
                    // assemble a new trait object:
                    let ptr_pair = (data_ptr, v_table_ptr);
                    debug_assert_eq!(size_of::<&T>(), size_of::<(*const u8, &*const ())>()); // fat pointer
                    let trait_obj_ref: &'a T = unsafe { std::mem::transmute_copy(&ptr_pair) };
                    trait_obj_ref
                })
            })
    }

    pub fn trait_iter_mut<'a, T: DynTrait + ?Sized>(
        &'a mut self,
    ) -> impl Iterator<Item = &'a mut T> {
        self.implementors::<T>()
            .into_iter()
            .flat_map(|(c_id, v_table_ptr)| {
                // getting the arena as `get` instead of as `get_mut` here is
                // not good, but we use unsafe anyway to get our way, so we do not care much here.
                let arena = self
                    .arenas
                    .get(&c_id)
                    .expect("Arena that is registered in dyn_traits not found!");

                arena.iter_raw_ptrs().map(move |data_ptr| {
                    // assemble a new trait object:
                    let ptr_pair = (data_ptr, v_table_ptr);
                    debug_assert_eq!(size_of::<&mut T>(), size_of::<(*const u8, &*const ())>()); // fat pointer
                    let trait_obj_ref: &'a mut T = unsafe { std::mem::transmute_copy(&ptr_pair) };
                    trait_obj_ref
                })
            })
    }
}

impl Debug for Arenas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arenas")
            .field("arenas", &self.arenas)
            .finish()
    }
}
