// pub struct SystemGraph<S> {}

use std::{marker::PhantomData, ops::DerefMut};

use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{events::Events, world::World};

pub struct SystemParams<'a, W> {
    pub world: &'a mut World<W>,
    pub events: &'a mut Events,
}

pub struct ParallelSystemParams<'a, W> {
    pub world: &'a World<W>,
    pub events: &'a Events,
}

impl<'a, W> From<SystemParams<'a, W>> for ParallelSystemParams<'a, W> {
    fn from(value: SystemParams<'a, W>) -> Self {
        Self {
            world: value.world,
            events: value.events,
        }
    }
}

pub trait System<W> {
    fn execute<'world>(&mut self, params: SystemParams<'world, W>);
}

trait ParallelSystem<W>: System<W> {
    fn execute<'world>(&mut self, world: ParallelSystemParams<'world, W>);
}

impl<W> System<W> for () {
    fn execute<'world>(&mut self, params: SystemParams<'world, W>) {}
}

impl<W> ParallelSystem<W> for () {
    fn execute<'world>(&mut self, world: ParallelSystemParams<'world, W>) {}
}

impl<S: System<W>, W> System<W> for Box<S> {
    fn execute<'world>(&mut self, params: SystemParams<'world, W>) {
        System::execute(self.deref_mut(), params)
    }
}

impl<S: ParallelSystem<W>, W> ParallelSystem<W> for Box<S> {
    fn execute<'world>(&mut self, params: ParallelSystemParams<'world, W>) {
        ParallelSystem::execute(self.deref_mut(), params)
    }
}

// impl<W, S: ParallelSystem<W>> System<W> for S {
//     fn execute(&mut self, state: &mut W) {
//         <Self as ParallelSystem<W>>::execute(self, &*state)
//     }
// }

// registers new players randomly
struct NetworkSystem {
    rng: ChaCha8Rng,
}

impl Default for NetworkSystem {
    fn default() -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(2),
        }
    }
}

pub struct Players {
    players: Vec<String>,
}

impl System<Players> for NetworkSystem {
    fn execute(&mut self, params: SystemParams<Players>) {
        let rng = &mut self.rng;
        let state = params.world.state_mut();
        match *["remove", "add"].choose(rng).unwrap() {
            "remove" => {
                if !state.players.is_empty() {
                    let removed = state.players.remove(0);
                    println!("NetworkSystem: removed {removed}");
                }
            }
            _ => {
                let rand_player = ["Peter", "Anna", "Paul", "Fred", "Jorge", "Manni"]
                    .choose(rng)
                    .unwrap();
                println!("NetworkSystem: added {rand_player}");
                state.players.push(rand_player.to_string());
            }
        }
    }
}

// calculates a layout of all ui components in the world
struct UiLayoutSystem {}

// logs out all components that are there.
struct LogSystem {}

impl ParallelSystem<Players> for UiLayoutSystem {
    fn execute(&mut self, params: ParallelSystemParams<Players>) {
        println!(
            "calcing layout for players: {:?}",
            params.world.state().players
        )
    }
}

// annoying right now, but ok.
impl System<Players> for UiLayoutSystem {
    fn execute(&mut self, params: SystemParams<Players>) {
        ParallelSystem::execute(self, params.into())
    }
}

impl ParallelSystem<Players> for LogSystem {
    fn execute(&mut self, params: ParallelSystemParams<Players>) {
        println!("LogSystem: printing players");
        for p in params.world.state().players.iter() {
            println!("    {p}");
        }
    }
}

// annoying right now, but ok.
impl System<Players> for LogSystem {
    fn execute(&mut self, params: SystemParams<Players>) {
        ParallelSystem::execute(self, params.into())
    }
}

// /////////////////////////////////////////////////////////////////////////////
// System Sequences
// /////////////////////////////////////////////////////////////////////////////

struct SystemSequence<W, S1: System<W>, S2: System<W>> {
    s1: S1,
    s2: S2,
    phantom_data: PhantomData<W>,
}

impl<W, S1: System<W>, S2: System<W>> SystemSequence<W, S1, S2> {
    pub fn new(s1: S1, s2: S2) -> Self {
        Self {
            s1,
            s2,
            phantom_data: PhantomData,
        }
    }
}

impl<W, S1: System<W>, S2: System<W>> System<W> for SystemSequence<W, S1, S2> {
    fn execute<'world>(&mut self, params: SystemParams<'world, W>) {
        let params1 = SystemParams {
            world: &mut *params.world,
            events: &mut *params.events,
        };
        self.s1.execute(params1);

        let params2 = SystemParams {
            world: &mut *params.world,
            events: &mut *params.events,
        };
        self.s2.execute(params2);
    }
}

fn sequence<W, S1: System<W>, S2: System<W>>(s1: S1, s2: S2) -> SystemSequence<W, S1, S2> {
    SystemSequence::new(s1, s2)
}

// /////////////////////////////////////////////////////////////////////////////
// System Parrelel sets:
// /////////////////////////////////////////////////////////////////////////////

struct ParralelSystemSet<W, S1: System<W>, S2: System<W>> {
    s1: S1,
    s2: S2,
    phantom_data: PhantomData<W>,
}

/// todo!() implement parallel systems using crossbeam::thread::scope. Or use UnsafeCell for the World.

// /////////////////////////////////////////////////////////////////////////////
// Tests
// /////////////////////////////////////////////////////////////////////////////

#[test]
fn test() {
    let network_system = NetworkSystem::default();
    let log_system = LogSystem {};
    let ui_layout_system = UiLayoutSystem {};

    let system = sequence(sequence(network_system, log_system), ui_layout_system);

    let mut system: Box<dyn System<Players>> = Box::new(system);

    let mut players = Players {
        players: vec!["Xenia".into(), "Lucas".into()],
    };
    let mut events = Events::new();
    let mut world = World::new(players);

    for i in 0..3 {
        println!("------- Tick {i} ---------------");
        dbg!(&world.state().players);
        println!("Run Systems for Tick {i}");
        system.execute(SystemParams {
            world: &mut world,
            events: &mut events,
        });
        println!("");
    }
}
