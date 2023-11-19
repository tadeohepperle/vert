use std::{
    any::{Any, TypeId},
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
};

use nohash_hasher::NoHashHasher;
use std::{collections::HashMap, hash::BuildHasherDefault};

use crate::{
    arena::{Arena, ArenaIndex, ArenaIter, TypedArena},
    component::Component,
    trait_companion::TraitCompanion,
};

/// W is some user defined world state. Aka global resources
pub struct World<W> {
    world_state: W,
    arenas: Arenas,
}

impl<W> World<W> {
    pub fn new(world_state: W) -> Self {
        World {
            world_state,
            arenas: Arenas::new(),
        }
    }
}

type ArenaAddress = TypeId;

pub struct Arenas {
    pub arenas: HashMap<TypeId, Arena>,
}

fn arena_address<C: Component>() -> ArenaAddress {
    TypeId::of::<C>()
}

impl Arenas {
    pub fn new() -> Self {
        Arenas {
            arenas: HashMap::new(),
        }
    }

    fn get_arena<'a, C: Component>(&'a self) -> Option<&'a TypedArena<C>> {
        let arena = self.arenas.get(&arena_address::<C>())?;
        Some(Borrow::borrow(arena))
    }

    fn get_arena_mut_or_insert<C: Component>(&mut self) -> &mut TypedArena<C> {
        let key = TypeId::of::<C>();
        let arena = self.arenas.entry(key).or_insert_with(|| Arena::new::<C>());
        BorrowMut::borrow_mut(arena)
    }

    fn get_arena_mut<C: Component>(&mut self) -> Option<&mut TypedArena<C>> {
        let arena = self
            .arenas
            .entry(arena_address::<C>())
            .or_insert_with(|| Arena::new::<C>());
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
        // todo! if the last one, teardown state for this component type
        let arena = self.get_arena_mut()?;
        arena.remove(i)
    }

    pub fn collect_collectables<'a, X: TraitCompanion>(
        &'a self,
    ) -> impl Iterator<Item = &'a X::Dyn> {
        std::iter::empty()
    }

    // pub fn iter<'a, C: Component>(&'a self) -> OptionArenaIter<'a, C> {
    //     match self.get_arena::<C>() {
    //         Some(a) => OptionArenaIter::Arena(a),
    //         None => OptionArenaIter::None,
    //     }
    // }

    pub fn iter<'a, C: Component>(&'a self) -> impl Iterator<Item = (ArenaIndex, &'a C)> {
        self.get_arena::<C>()
            .map(|e| e.iter())
            .into_iter()
            .flatten()
    }
    pub fn iter_mut<'a, C: Component>(
        &'a mut self,
    ) -> impl Iterator<Item = (ArenaIndex, &'a mut C)> {
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
