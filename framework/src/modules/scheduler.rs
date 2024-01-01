use std::{
    ops::{Add, Sub},
    vec,
};

use crate::{
    app::{ModuleId, UntypedHandle},
    utils::{EntryKey, Timing, TimingQueue},
    Handle, Module,
};

pub struct Scheduler {
    on_exit: TimingQueue<FunctionHandle>,
    on_update: TimingQueue<FunctionHandle>,
    /// if Some(reason) then at the end of the frame exit the game loop.
    exit_requested: Option<String>,
}

impl Module for Scheduler {
    type Config = ();
    type Dependencies = ();
    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        Ok(Self {
            on_exit: TimingQueue::new(),
            on_update: TimingQueue::new(),
            exit_requested: None,
        })
    }
}

impl Scheduler {
    /// # Warning!
    ///
    /// Call this function only once each frame in the main module's game loop. Nowhere else!
    /// Currently hard to protect, while keeping it exposed.
    pub fn update(&mut self) -> UpdateFlow {
        for e in self.on_update.iter() {
            e.execute();
        }
        if let Some(reason) = self.exit_requested.take() {
            return UpdateFlow::Exit(reason);
        }
        UpdateFlow::Continue
    }

    /// high timing functions will run after low timing ones.
    pub fn register<M: Module>(
        &mut self,
        handle: &Handle<M>,
        schedule: Schedule,
        timing: Timing,
        function: fn(&mut M) -> (),
    ) {
        // is this okay??
        let type_punned_function: fn(*mut ()) -> () = unsafe { std::mem::transmute(function) };
        let function = FunctionHandle {
            module_id: ModuleId::of::<M>(),
            module_handle: handle.untyped(),
            type_punned_function,
        };
        let schedule = self.schedule(schedule);
        schedule.insert(function, timing); // todo! return a key that contains the schedule, to allow for deregistering.
    }
    pub fn request_exit(&mut self, reason: impl ToString) {
        self.exit_requested = Some(reason.to_string());
    }

    fn schedule(&mut self, schedule: Schedule) -> &mut TimingQueue<FunctionHandle> {
        match schedule {
            Schedule::Exit => &mut self.on_exit,
            Schedule::Update => &mut self.on_update,
        }
    }

    // todo!()  pub fn deregister()
}

struct FunctionHandle {
    module_id: ModuleId,
    module_handle: UntypedHandle,
    type_punned_function: fn(*mut ()) -> (),
}

impl FunctionHandle {
    #[inline(always)]
    fn execute(&self) {
        let module_ptr: *mut () = self.module_handle.ptr as *mut ();
        (self.type_punned_function)(module_ptr);
    }
}

pub enum UpdateFlow {
    Exit(String),
    Continue,
}

#[derive(Debug, Clone, Copy)]
pub enum Schedule {
    Exit,
    Update,
}
