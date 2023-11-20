// pub struct SystemGraph<S> {}

use std::marker::PhantomData;

use rand::{seq::SliceRandom, thread_rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub trait System<W> {
    fn execute(&mut self, state: &mut W);
}

trait ParallelSystem<W>: System<W> {
    fn execute(&mut self, state: &W);
}

impl<W> System<W> for () {
    fn execute(&mut self, _: &mut W) {}
}

impl<W> ParallelSystem<W> for () {
    fn execute(&mut self, _: &W) {}
}

// impl<W, S: ParallelSystem<W>> System<W> for S {
//     fn execute(&mut self, state: &mut W) {
//         <Self as ParallelSystem<W>>::execute(self, &*state)
//     }
// }

#[derive(Debug, Clone)]
struct World {
    players: Vec<String>,
}

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

impl System<World> for NetworkSystem {
    fn execute(&mut self, state: &mut World) {
        let rng = &mut self.rng;
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

impl ParallelSystem<World> for UiLayoutSystem {
    fn execute(&mut self, state: &World) {
        println!("calcing layout for players: {:?}", state.players)
    }
}

// annoying right now, but ok.
impl System<World> for UiLayoutSystem {
    fn execute(&mut self, state: &mut World) {
        ParallelSystem::execute(self, state)
    }
}

impl ParallelSystem<World> for LogSystem {
    fn execute(&mut self, state: &World) {
        println!("LogSystem: printing players");
        for p in state.players.iter() {
            println!("    {p}");
        }
    }
}

// annoying right now, but ok.
impl System<World> for LogSystem {
    fn execute(&mut self, state: &mut World) {
        ParallelSystem::execute(self, state)
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
    fn execute(&mut self, state: &mut W) {
        self.s1.execute(state);
        self.s2.execute(state);
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

    let mut system: Box<dyn System<World>> = Box::new(system);

    let mut world = World {
        players: vec!["Xenia".into(), "Lucas".into()],
    };
    for i in 0..3 {
        println!("------- Tick {i} ---------------");
        dbg!(&world);
        println!("Run Systems for Tick {i}");
        system.execute(&mut world);
        println!("");
    }
}
