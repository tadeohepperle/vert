use std::{marker::PhantomData, sync::Arc};

use crate::{
    system::System,
    trait_companion::{MultiTraitCompanion, TraitCompanion},
    world::World,
};

/// W is the world state.
pub struct App<W, T: MultiTraitCompanion> {
    world: World<W>,
    system: Box<dyn System<W>>,
    trait_companion: T,
}

impl<W, T: TraitCompanion> App<W, T> {
    pub fn new(world_state: W, system: impl System<W> + 'static, trait_companion: T) -> Self {
        App {
            world: World::new(world_state),
            system: Box::new(system),
            trait_companion,
        }
    }
}

pub struct AppBuilder {}

impl AppBuilder {}

#[cfg(test)]
pub mod tests {

    use super::App;

    fn register_collectables() {
        let mut a = App::new((), (), ());
    }
}
