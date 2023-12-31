use std::{
    ops::{Add, Sub},
    vec,
};

use crate::{
    app::{ModuleId, UntypedHandle},
    Handle, Module,
};

pub struct Scheduler {
    on_exit: ScheduledFunctions,
    on_update: ScheduledFunctions,
    exit_ordered: Option<String>,
}

impl Module for Scheduler {
    type Config = ();
    type Dependencies = ();
    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        Ok(Self {
            on_exit: ScheduledFunctions {
                schedule: Schedule::Exit,
                execution_order: vec![],
            },
            on_update: ScheduledFunctions {
                schedule: Schedule::Update,
                execution_order: vec![],
            },
            exit_ordered: None,
        })
    }
}

impl Scheduler {
    /// # Warning!
    ///
    /// Call this function only once each frame in the main module's game loop. Nowhere else!
    /// Currently hard to protect, while keeping it exposed.
    pub fn update(&mut self) -> UpdateFlow {
        self.on_update.execute();
        if let Some(reason) = self.exit_ordered.take() {
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
            timing,
            module_id: ModuleId::of::<M>(),
            module_handle: handle.untyped(),
            type_punned_function,
        };
        let schedule = self.schedule(schedule);
        schedule.insert(function);
    }

    pub fn request_exit(&mut self, reason: impl ToString) {
        self.exit_ordered = Some(reason.to_string());
    }

    fn schedule(&mut self, schedule: Schedule) -> &mut ScheduledFunctions {
        match schedule {
            Schedule::Exit => &mut self.on_exit,
            Schedule::Update => &mut self.on_update,
        }
    }

    // todo!()  pub fn deregister()
}

struct ScheduledFunctions {
    schedule: Schedule,
    execution_order: Vec<FunctionHandle>,
}
impl ScheduledFunctions {
    #[inline(always)]
    fn execute(&self) {
        for f in self.execution_order.iter() {
            f.execute();
        }
    }

    fn insert(&mut self, function: FunctionHandle) {
        // search for index, where the function has a higher timing than this one.
        let insertion_index = self.execution_order.iter().enumerate().find_map(|(i, e)| {
            if e.timing > function.timing {
                Some(i)
            } else {
                None
            }
        });
        match insertion_index {
            Some(i) => self.execution_order.insert(i, function),
            None => self.execution_order.push(function),
        }
    }
}

/// Timing can be thought of as the inverse of Priority.
/// A high timing value means, a function will be executed later in a schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timing(i32);

impl Timing {
    pub const START: Timing = Timing(-10000);
    pub const MIDDLE: Timing = Timing(0);
    pub const END: Timing = Timing(10000);
}

impl Add<i32> for Timing {
    type Output = Timing;

    fn add(self, rhs: i32) -> Self::Output {
        Timing(self.0 + rhs)
    }
}

impl Sub<i32> for Timing {
    type Output = Timing;

    fn sub(self, rhs: i32) -> Self::Output {
        Timing(self.0 - rhs)
    }
}

struct FunctionHandle {
    timing: Timing,
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
