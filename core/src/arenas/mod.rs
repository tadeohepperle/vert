use std::{
    any::TypeId,
    borrow::{Borrow, BorrowMut},
    collections::hash_map::Entry,
    fmt::Debug,
    mem::size_of,
};

use std::collections::HashMap;

use smallvec::{smallvec, SmallVec};

pub mod arena;

use crate::{
    component::Component,
    // trait_companion::{MultiTraitCompanion, Reflectable, VTablePointer, VTablePointerWithMetadata},
    trait_reflection::{DynTrait, VTablePtr, VTablePtrWithMeta},
};

use self::arena::{Arena, ArenaIndex, TypedArena};

type ArenaAddress = TypeId;

#[derive(Default)]
pub struct Arenas {
    pub arenas: HashMap<TypeId, ComponentArena>,
    // maps a dyn_trait_id to the arena addresses and vtablepointers of all components that implement this trait
    dyn_traits_registry: DynTraitRegistry,
}

#[derive(Debug)]
pub struct DynTraitImplementors {
    dyn_trait_id: TypeId,
    dyn_trait_name: &'static str,
    component_vtables: SmallVec<[(ArenaAddress, VTablePtr); 8]>,
    component_resources_vtables: SmallVec<[(ArenaAddress, VTablePtr); 8]>,
}

#[derive(Debug, Default)]
struct DynTraitRegistry {
    /// maps dyn_trait_ids to implementors vector.
    inner: HashMap<TypeId, DynTraitImplementors>,
}

impl DynTraitRegistry {
    /// dyn_trait_id is the TypeId of `dyn MyTrait`
    #[inline]
    fn get(&self, dyn_trait_id: &TypeId) -> Option<&DynTraitImplementors> {
        self.inner.get(dyn_trait_id)
    }

    /// adds a vtable pointer to the components in a certain arena, or to the singleton component resource of that arena.
    ///
    /// ## Panics if pointer already present.
    fn insert_vtable_ptr(
        &mut self,
        arena: ArenaAddress,
        ptr_with_meta: VTablePtrWithMeta,
        is_resource: bool,
    ) {
        let implementors = self
            .inner
            .entry(ptr_with_meta.dyn_trait_id)
            .or_insert_with(|| DynTraitImplementors {
                dyn_trait_id: ptr_with_meta.dyn_trait_id,
                dyn_trait_name: ptr_with_meta.dyn_trait_name,
                component_vtables: smallvec![],
                component_resources_vtables: smallvec![],
            });

        // this arena should not be part of the arena vtables already:
        assert!(!implementors.component_vtables.iter().any(|e| e.0 == arena));

        implementors
            .component_vtables
            .push((arena, ptr_with_meta.ptr));
    }

    /// remove vtable pointer to the components in a certain arena, or to the singleton component resource of that arena.
    ///
    /// ## Panics if pointer already present.
    fn remove_vtable_ptr(
        &mut self,
        arena: ArenaAddress,
        ptr_with_meta: VTablePtrWithMeta,
        is_resource: bool,
    ) {
        let implementors = self
            .inner
            .get_mut(&ptr_with_meta.dyn_trait_id)
            .expect("dyn traits should contain this arena");

        // this arena should be part of the arena vtables already:
        assert!(implementors
            .component_vtables
            .iter()
            .any(|e| e.0 == arena && e.1 == ptr_with_meta.ptr));
        implementors.component_vtables.retain(|e| e.0 != arena);
    }
}

fn arena_address<C: Component>() -> ArenaAddress {
    C::id()
}

fn new_component_arena<C: Component>(dyn_trait_registry: &mut DynTraitRegistry) -> ComponentArena {
    let arena_address = arena_address::<C>();
    // set the vtable pointers for traits implemented by each component:
    let dyn_traits = unsafe { C::dyn_traits() };
    for ptr_with_meta in dyn_traits {
        dyn_trait_registry.insert_vtable_ptr(arena_address, *ptr_with_meta, false)
    }

    // set up a singleton resource for this type of component
    let resource_dyn_traits = unsafe { C::dyn_traits() };
    for ptr_with_meta in resource_dyn_traits {
        dyn_trait_registry.insert_vtable_ptr(arena_address, *ptr_with_meta, false)
    }

    ComponentArena {
        arena: TypedArena::<C>::new().into_untyped(),
    }
}

impl Arenas {
    pub fn new() -> Self {
        Default::default()
    }

    fn get_arena<'a, C: Component>(&'a self) -> Option<&'a TypedComponentArena<C>> {
        let arena = self.arenas.get(&arena_address::<C>())?;
        Some(Borrow::borrow(arena))
    }

    fn get_arena_mut_or_insert<C: Component>(&mut self) -> &mut TypedComponentArena<C> {
        let arena_address = arena_address::<C>();
        let arena = match self.arenas.entry(arena_address) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(vacant) => {
                let arena = new_component_arena::<C>(&mut self.dyn_traits_registry);
                vacant.insert(arena)
            }
        };
        BorrowMut::borrow_mut(arena)
    }

    fn get_arena_mut<C: Component>(&mut self) -> Option<&mut TypedComponentArena<C>> {
        let arena = self.arenas.get_mut(&arena_address::<C>())?;
        Some(BorrowMut::borrow_mut(arena))
    }

    pub fn insert<C: Component>(&mut self, component: C) -> ArenaIndex {
        // todo! if the first one, setup state for this component type
        let component_arena = self.get_arena_mut_or_insert::<C>();
        component_arena.arena.insert(component)
    }

    pub fn get<C: Component>(&self, i: ArenaIndex) -> Option<&C> {
        let arena = self.get_arena::<C>()?;
        arena.arena.get(i)
    }

    pub fn get_mut<C: Component>(&mut self, i: ArenaIndex) -> Option<&mut C> {
        let component_arena = self.get_arena_mut()?;
        component_arena.arena.get_mut(i)
    }

    pub fn remove<C: Component>(&mut self, i: ArenaIndex) -> Option<C> {
        let component_arena = self.get_arena_mut()?;
        let removed_value = component_arena.arena.remove(i);
        // if this component was the last one in this arena, remove the arena to not use up more space:
        if component_arena.arena.len() == 0 {
            let arena_address = arena_address::<C>();
            // remove arena:
            let arena = self
                .arenas
                .remove(&arena_address)
                .expect("Arena is present, because it was just queried");

            // set the vtable pointers for traits implemented by each component:
            let dyn_traits = unsafe { C::dyn_traits() };
            for ptr_with_meta in dyn_traits {
                self.dyn_traits_registry
                    .remove_vtable_ptr(arena_address, *ptr_with_meta, false)
            }

            // set up a singleton resource for this type of component
            let resource_dyn_traits = unsafe { C::dyn_traits() };
            for ptr_with_meta in resource_dyn_traits {
                self.dyn_traits_registry
                    .remove_vtable_ptr(arena_address, *ptr_with_meta, false)
            }

            // drop arena:
            // this triggers the Blob::free<C>() function that triggers the drop of all components stored in the blob e.g. Strings that need to free memory, vecs, etc...

            let removed_arena = self
                .arenas
                .remove(&arena_address)
                .expect("arena removed due to 0 elements was not found");
            removed_arena.into_typed::<C>().free();
        }
        // remove the vtable pointers for this area from the
        removed_value
    }

    pub fn iter<C: Component>(&self) -> impl Iterator<Item = (ArenaIndex, &C)> {
        self.get_arena::<C>()
            .map(|e| e.arena.iter())
            .into_iter()
            .flatten()
    }
    pub fn iter_mut<C: Component>(&mut self) -> impl Iterator<Item = (ArenaIndex, &mut C)> {
        self.get_arena_mut::<C>()
            .map(|e| e.arena.iter_mut())
            .into_iter()
            .flatten()
    }

    pub fn implementors<T: DynTrait + ?Sized>(&self) -> SmallVec<[(TypeId, VTablePtr); 8]> {
        let dyn_trait_id = T::id();
        let implementors = self
            .dyn_traits_registry
            .get(&dyn_trait_id)
            .map(|e| &e.component_vtables)
            .cloned() // cloned not ideal, but okay.
            .unwrap_or_default();
        implementors
    }

    pub fn iter_component_traits<'a, T: DynTrait + ?Sized>(
        &'a self,
    ) -> impl Iterator<Item = &'a T> {
        self.implementors::<T>()
            .into_iter()
            .flat_map(|(c_id, v_table_ptr)| {
                let component_arena = self
                    .arenas
                    .get(&c_id)
                    .expect("Arena that is registered in dyn_traits not found!");
                component_arena.arena.iter_raw_ptrs().map(move |data_ptr| {
                    // assemble a new trait object:
                    let ptr_pair = (data_ptr, v_table_ptr);
                    debug_assert_eq!(size_of::<&T>(), size_of::<(*const u8, &*const ())>()); // fat pointer
                    let trait_obj_ref: &'a T = unsafe { std::mem::transmute_copy(&ptr_pair) };
                    trait_obj_ref
                })
            })
    }

    pub fn iter_component_traits_mut<'a, T: DynTrait + ?Sized>(
        &'a mut self,
    ) -> impl Iterator<Item = &'a mut T> {
        self.implementors::<T>()
            .into_iter()
            .flat_map(|(c_id, v_table_ptr)| {
                // getting the arena as `get` instead of as `get_mut` here is
                // not good, but we use unsafe anyway to get our way, so we do not care much here.
                let component_arena = self
                    .arenas
                    .get(&c_id)
                    .expect("Arena that is registered in dyn_traits not found!");

                component_arena.arena.iter_raw_ptrs().map(move |data_ptr| {
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

#[derive(Debug)]
#[repr(C)]
pub struct ComponentArena {
    arena: Arena,
}

#[derive(Debug)]
#[repr(C)]
pub struct TypedComponentArena<C> {
    arena: TypedArena<C>,
}

impl<T> Borrow<TypedComponentArena<T>> for ComponentArena {
    fn borrow(&self) -> &TypedComponentArena<T> {
        self.arena.assert_t_matches::<T>();
        let ptr_to_self = self as *const ComponentArena;
        let imagine_it_was_typed = ptr_to_self as *const TypedComponentArena<T>;
        unsafe { &*imagine_it_was_typed }
    }
}

impl<T> BorrowMut<TypedComponentArena<T>> for ComponentArena {
    fn borrow_mut(&mut self) -> &mut TypedComponentArena<T> {
        self.arena.assert_t_matches::<T>();
        let ptr_to_self = self as *mut ComponentArena;
        let imagine_it_was_typed = ptr_to_self as *mut TypedComponentArena<T>;
        unsafe { &mut *imagine_it_was_typed }
    }
}
impl ComponentArena {
    fn into_typed<C>(self) -> TypedComponentArena<C> {
        TypedComponentArena {
            arena: self.arena.into_typed(),
        }
    }
}

impl<T> TypedComponentArena<T> {
    pub fn free(self) {
        self.arena.free();
    }
}
