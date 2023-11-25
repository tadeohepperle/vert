use std::{
    any::TypeId,
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
};

use std::collections::HashMap;

use crate::{
    arena::{Arena, ArenaIndex, TypedArena},
    component::Component,
    // trait_companion::{MultiTraitCompanion, Reflectable, VTablePointer, VTablePointerWithMetadata},
    trait_reflection::{DynTrait, Implementor, MultipleReflectedTraits, VTablePtrWithMeta},
};

/// W is some user defined world state. Aka global resources
pub struct World<W> {
    state: W,
    arenas: Arenas,
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
}

type ArenaAddress = TypeId;

pub struct Arenas {
    pub arenas: HashMap<TypeId, ArenaWithMetadata>,
}

fn arena_address<C: Component>() -> ArenaAddress {
    C::id()
}

impl Arenas {
    pub fn new() -> Self {
        Arenas {
            arenas: HashMap::new(),
        }
    }

    fn get_arena<'a, C: Component>(&'a self) -> Option<&'a TypedArena<C>> {
        let arena = self.arenas.get(&arena_address::<C>())?;
        Some(Borrow::borrow(&arena.arena))
    }

    fn get_arena_mut_or_insert<C: Component, T: MultipleReflectedTraits>(
        &mut self,
    ) -> &mut TypedArena<C> {
        let key = TypeId::of::<C>();
        let arena = self
            .arenas
            .entry(key)
            .or_insert_with(|| ArenaWithMetadata::new::<C, T>());
        BorrowMut::borrow_mut(&mut arena.arena)
    }

    fn get_arena_mut<C: Component>(&mut self) -> Option<&mut TypedArena<C>> {
        let arena = self.arenas.get_mut(&arena_address::<C>())?;
        Some(BorrowMut::borrow_mut(&mut arena.arena))
    }

    pub fn insert<C: Component, T: MultipleReflectedTraits>(&mut self, component: C) -> ArenaIndex {
        // todo! if the first one, setup state for this component type
        let arena = self.get_arena_mut_or_insert::<C, T>();
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
        // todo! if the last one, teardown state for this component type
        let arena = self.get_arena_mut()?;
        arena.remove(i)
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
}

impl Debug for Arenas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arenas")
            .field("arenas", &self.arenas)
            .finish()
    }
}

#[derive(Debug)]
pub struct ArenaWithMetadata {
    arena: Arena,
    // Maps the type ids of the trait companion struct to pointers for the
    trait_obj_pointers: HashMap<TypeId, VTablePtrWithMeta>,
}

impl ArenaWithMetadata {
    fn new<C: Implementor, M: MultipleReflectedTraits>() -> Self {
        let arena = TypedArena::<C>::new();
        let ptrs = unsafe { C::dyn_traits() };
        let trait_obj_pointers: HashMap<TypeId, VTablePtrWithMeta> = ptrs
            .into_iter()
            .map(|v| (v.dyn_trait_type_id, *v))
            .collect();

        ArenaWithMetadata {
            arena: arena.into_untyped(),
            trait_obj_pointers,
        }
    }

    pub fn trait_iter<'a, T: DynTrait>(&'a self) -> Option<impl Iterator<Item = &'a T>> {
        let dyn_trait_type_id = T::id();
        let trait_obj_info = self.trait_obj_pointers.get(&dyn_trait_type_id)?;
        let iter = self.arena.iter_raw_ptrs().map(move |data_ptr| {
            let ptr_pair = (data_ptr, trait_obj_info.ptr);
            let trait_obj_ref: &'a T = unsafe { std::mem::transmute_copy(&ptr_pair) };
            trait_obj_ref
        });
        Some(iter)
    }

    /// Note: vtables for &mut T::Dyn and &T::Dyn trait objects are the same.
    pub fn trait_iter_mut<'a, T: DynTrait>(
        &'a mut self,
    ) -> Option<impl Iterator<Item = &'a mut T>> {
        let dyn_trait_type_id = T::id();
        let trait_obj_info = self.trait_obj_pointers.get(&dyn_trait_type_id)?;
        let iter = self.arena.iter_raw_ptrs().map(move |data_ptr| {
            let ptr_pair = (data_ptr, trait_obj_info.ptr);
            let trait_obj_ref: &'a mut T = unsafe { std::mem::transmute_copy(&ptr_pair) };
            trait_obj_ref
        });
        Some(iter)
    }

    // todo!() iterate over multiple traits at once.
}
