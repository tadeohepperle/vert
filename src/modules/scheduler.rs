use crate::{
    app::function_handle::VoidFunctionHandle,
    utils::{Timing, TimingQueue},
    Handle, Module,
};

pub struct Scheduler {
    on_exit: TimingQueue<VoidFunctionHandle>,
    on_update: TimingQueue<VoidFunctionHandle>,
    /// if Some(reason) then at the end of the frame exit the game loop.
    exit_requested: Option<String>,
}

impl Module for Scheduler {
    type Config = ();
    type Dependencies = ();
    fn new(_config: Self::Config, _deps: Self::Dependencies) -> anyhow::Result<Self> {
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
            e.call();
        }
        if let Some(reason) = self.exit_requested.take() {
            for e in self.on_exit.iter() {
                e.call();
            }
            return UpdateFlow::Exit(reason);
        }
        UpdateFlow::Continue
    }

    pub fn register_update<M: Module>(
        &mut self,
        handle: Handle<M>,
        timing: Timing,
        func: fn(&mut M) -> (),
    ) {
        self.register(handle, Schedule::Update, timing, func)
    }

    /// high timing functions will run after low timing ones.
    pub fn register<M: Module>(
        &mut self,
        handle: Handle<M>,
        schedule: Schedule,
        timing: Timing,
        func: fn(&mut M) -> (),
    ) {
        // is this okay??
        let _type_punned_function: fn(*mut ()) -> () = unsafe { std::mem::transmute(func) };
        let void_function_handle = VoidFunctionHandle::new(handle, func);
        let schedule = self.schedule(schedule);
        schedule.insert(void_function_handle, timing); // todo! return a key that contains the schedule, to allow for deregistering.
    }
    pub fn request_exit(&mut self, reason: impl ToString) {
        self.exit_requested = Some(reason.to_string());
    }

    fn schedule(&mut self, schedule: Schedule) -> &mut TimingQueue<VoidFunctionHandle> {
        match schedule {
            Schedule::Exit => &mut self.on_exit,
            Schedule::Update => &mut self.on_update,
        }
    }

    // todo!()  pub fn deregister()
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
