use std::{any::Any, marker::PhantomData};

use crate::{
    events::Events,
    system::{System, SystemParams},
    trait_reflection::MultipleReflectedTraits,
    world::World,
};

/// W is the world state.
///
/// W is world state
/// T is Traits that can be queries
/// E is external events
/// I is internal events
pub struct App<W = (), T: MultipleReflectedTraits = (), S: System<W> = ()> {
    pub world: World<W>,
    pub system: S,
    pub phantom: PhantomData<T>,
    pub events: Events,
}

pub struct ShouldShutdown(pub bool);
pub struct ShutdownEvent;

impl<W, T: MultipleReflectedTraits, S: System<W>> App<W, T, S> {
    pub fn new(world_state: W, system: S) -> Self {
        App {
            world: World::new(world_state),
            system,
            phantom: PhantomData::<T>,
            events: Events::new(),
        }
    }

    /// this function should be called externally in an event_loop.
    pub fn run_1_frame(&mut self) -> ShouldShutdown {
        // run systems:
        let params = SystemParams {
            world: &mut self.world,
            events: &mut self.events,
        };
        self.system.execute(params);
        // check for shutdown and clear events:
        if self.events.read_t::<ShutdownEvent>().next().is_some() {
            return ShouldShutdown(true);
        }
        self.events.clear();
        ShouldShutdown(false)
    }

    pub fn add_external_event<Ev: Into<Box<dyn Any>>>(&mut self, event: Ev) {
        self.events.write(event.into());
    }
}

pub struct AppBuilder {}

impl AppBuilder {}

#[cfg(test)]
pub mod tests {

    use super::App;

    #[test]
    fn construct_app() {
        let _a: App<(), ()> = App::new((), ());
    }
}
