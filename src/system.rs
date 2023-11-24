use std::borrow::Borrow;

use crate::{events::Events, world::World};

pub trait System<W> {
    fn execute<'world>(&mut self, world: &'world mut World<W>, events: &'world mut Events);
}

impl<W> System<W> for () {
    fn execute<'world>(&mut self, world: &'world mut World<W>, events: &'world mut Events) {}
}

impl<W, F> System<W> for F
where
    F: for<'world> FnMut(&'world mut World<W>, &'world mut Events) -> (),
{
    fn execute<'world>(&mut self, world: &'world mut World<W>, events: &'world mut Events) {
        self(world, events)
    }
}
