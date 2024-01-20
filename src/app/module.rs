use std::fmt::Display;

use super::{dependencies::Dependencies, handle::Handle, App};

/// A module that is part of an app.
pub trait Module: 'static + Sized {
    /// Some initial data that configures the module. Provided by the User when adding a module to an app.
    type Config: 'static + Sized + Clone + PartialEq + std::fmt::Debug = ();
    /// Other modules that are expected to be part of the app. Provided automatically during app setup, where the program
    /// resolves which dependencies each module has.
    type Dependencies: Dependencies = ();

    /// creates this module
    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self>;

    /// Is run once, after all modules have been initialized.
    /// This function is optional, it is given a handle to the module itself.
    /// other modules can be accessed if you cache the handles to them in the `new()` function.
    /// E.g. the LineRenderer could register its own handle (Handle<LineRenderer>) with a general Renderer module,
    /// if a `Handle<Renderer>` was part of the `Self::Dependencies` and cached in the `new` function.
    /// E.g. LineRenderer could have a field `renderer: Handle<Renderer>` that is populated in `new`.
    fn intialize(_handle: Handle<Self>) -> anyhow::Result<()> {
        Ok(())
    }
}

pub trait MainModule: Module {
    /// takes control over how to run the application
    fn main(&mut self, app: &App) -> anyhow::Result<()>;
}

/// Wraps a type id and a type name for a Module.
#[derive(Debug, Clone, Copy)]
pub struct ModuleId {
    type_id: std::any::TypeId,
    type_name: &'static str,
}

impl Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty_name = self.type_name.split("::").last().unwrap();
        f.write_str(ty_name)
    }
}

impl PartialEq for ModuleId {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl Eq for ModuleId {}

impl std::hash::Hash for ModuleId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}

impl ModuleId {
    pub fn of<T: Module>() -> Self {
        ModuleId {
            type_id: std::any::TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }
}
