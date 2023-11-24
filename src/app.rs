use std::{any::Any, marker::PhantomData};

use crate::{
    events::Events, system::System, trait_reflection::MultipleReflectedTraits, world::World,
};

/// W is the world state.
///
/// W is world state
/// T is Traits that can be queries
/// E is external events
/// I is internal events
pub struct App<W = (), T: MultipleReflectedTraits = ()> {
    pub world: World<W>,
    pub system: Box<dyn System<W>>,
    pub phantom: PhantomData<T>,
    pub events: Events,
}

pub struct ShouldShutdown(pub bool);
pub struct ShutdownEvent;

impl<W, T: MultipleReflectedTraits> App<W, T> {
    pub fn new(world_state: W, system: Box<dyn System<W>>) -> Self {
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
        self.system.execute(&mut self.world, &mut self.events);
        // check for shutdown and clear events:
        if self.events.read_t::<ShutdownEvent>().next().is_some() {
            return ShouldShutdown(true);
        }
        self.events.clear();
        ShouldShutdown(false)
    }

    pub fn add_external_event(&mut self, event: impl Any) {
        self.events.write(event);
    }
}

pub struct AppBuilder {}

impl AppBuilder {}

#[cfg(test)]
pub mod tests {

    use crate::{events::Events, world::World};

    use super::App;

    #[test]
    fn construct_app() {
        fn my_system(params: &mut World<()>, events: &mut Events) {}
        let _a: App<(), ()> = App::new((), Box::new(my_system));
    }
}
