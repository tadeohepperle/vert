use std::marker::PhantomData;

use crate::modules::Modules;

use self::extract::Extract;

pub mod extract;

/// E is extracted from the user-defined state.
pub trait System<E = ()>: 'static {
    // type In: 'static;
    // type Out: 'static;

    /// runs every frame.
    fn run(&mut self, modules: &mut Modules, state: E) -> SystemOutput;
}

pub enum SystemOutput {
    End,
    KeepRunning,
}

/// S is the user defined StateT.
pub struct SystemStore<S> {
    phantom: PhantomData<S>,
}

impl<S> SystemStore<S> {
    pub fn register_system<F, E>()
    where
        F: System<E>,
        E: for<'a> Extract<'a, S>,
    {
    }
}
