use std::sync::Arc;

use crate::{collectable::CollectableTrait, system::System, world::World};

/// W is the world state.
pub struct App<W> {
    world: World<W>,
    system: Box<dyn System<W>>,
}

impl<W> App<W> {
    pub fn new(world_state: W, system: impl System<W> + 'static) -> Self {
        App {
            world: World::new(world_state),
            system: Box::new(system),
        }
    }

    pub fn register_collectable_trait<X: CollectableTrait>(&mut self) {
        // now what??? Every time we get a
        /*

        We need a way in the arenas to check for each

        CollectableTrait what






         */
    }
}

pub struct AppBuilder {}

impl AppBuilder {}

#[cfg(test)]
pub mod tests {
    use crate::collectable::CollectDescribeMe;

    use super::App;

    fn register_collectables() {
        let mut a = App::new((), ());
        a.register_collectable_trait::<CollectDescribeMe>();
    }
}
