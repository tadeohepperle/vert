use std::{
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
    fmt::Display,
    ops::DerefMut,
};

use anyhow::anyhow;
use bumpalo::Bump;

pub trait ModuleT: 'static + Sized {
    type Config: 'static + Sized + Clone = ();
    type Dependencies: DependenciesT = ();

    /// creates this module
    fn new(config: Self::Config, deps: Self::Dependencies) -> Self;

    /// Is run once, after all modules have been initialized.
    /// This function is optional, it is given a handle to the module itself.
    /// other modules can be accessed if you cache the handles to them in the `new()` function.
    /// E.g. the LineRenderer could register its own handle (Handle<LineRenderer>) with a general Renderer module,
    /// if a `Handle<Renderer>` was part of the `Self::Dependencies` and cached in the `new` function.
    /// E.g. LineRenderer could have a field `renderer: Handle<Renderer>` that is populated in `new`.
    fn intialize(handle: Handle<Self>) {}
}

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
    pub fn of<T: ModuleT>() -> Self {
        ModuleId {
            type_id: std::any::TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }
}

pub trait DependenciesT {
    fn type_ids() -> Vec<ModuleId>;
    fn from_untyped_handles(ptrs: &[UntypedHandle]) -> Self;
}

impl DependenciesT for () {
    fn type_ids() -> Vec<ModuleId> {
        vec![]
    }

    fn from_untyped_handles(ptrs: &[UntypedHandle]) -> Self {
        assert_eq!(ptrs.len(), 0);
        ()
    }
}

pub struct Handle<T: 'static> {
    ptr: &'static UnsafeCell<T>,
}

impl<T: 'static> std::ops::Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let reference: &'static T = unsafe { &*self.ptr.get() };
        reference
    }
}

impl<T: 'static> DerefMut for Handle<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let reference: &'static mut T = unsafe { &mut *self.ptr.get() };
        reference
    }
}

impl<T: 'static> Handle<T> {
    fn untyped(&self) -> UntypedHandle {
        UntypedHandle {
            ptr: unsafe { std::mem::transmute(self.ptr) },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UntypedHandle {
    ptr: *const (),
}

impl UntypedHandle {
    fn typed<T: 'static>(&self) -> Handle<T> {
        Handle {
            ptr: unsafe { std::mem::transmute(self.ptr) },
        }
    }
}

impl<T: ModuleT> DependenciesT for Handle<T> {
    fn type_ids() -> Vec<ModuleId> {
        vec![ModuleId::of::<T>()]
    }

    fn from_untyped_handles(ptrs: &[UntypedHandle]) -> Self {
        assert!(ptrs.len() == 1);
        ptrs[0].typed()
    }
}

impl<A: DependenciesT, B: DependenciesT> DependenciesT for (A, B) {
    fn type_ids() -> Vec<ModuleId> {
        let mut e = vec![];
        e.extend(A::type_ids());
        e.extend(B::type_ids());
        e
    }

    fn from_untyped_handles(ptrs: &[UntypedHandle]) -> Self {
        let a: A;
        let b: B;

        let mut offset = -(A::type_ids().len() as i32);

        {
            let a_len = A::type_ids().len() as i32;
            offset += a_len;
            a = A::from_untyped_handles(&ptrs[offset as usize..(offset + a_len) as usize]);
        }

        {
            let b_len = B::type_ids().len() as i32;
            offset += b_len;
            b = B::from_untyped_handles(&ptrs[offset as usize..(offset + b_len) as usize]);
        }

        (a, b)
    }
}

pub struct AppBuilder {
    module_configs: &'static mut bumpalo::Bump,
    added_modules: HashMap<ModuleId, AddedModule>,
}

pub struct App {
    module_configs: &'static bumpalo::Bump,
    modules: &'static bumpalo::Bump,
    instantiated_modules: HashMap<ModuleId, InstantiatedModule>,
}

impl AppBuilder {
    /// todo! parameterize with some struct that handles inputs and updates.
    pub fn new() -> Self {
        AppBuilder {
            module_configs: Box::leak(Box::new(bumpalo::Bump::new())),
            added_modules: HashMap::new(),
        }
    }

    /// Adds a module to the app. Does NOT instantiate and intitialize it yet.
    pub fn add<M: ModuleT>(mut self) -> Self
    where
        M::Config: Default,
    {
        let config: M::Config = Default::default();
        self.add_with_config::<M>(config)
    }

    /// Adds a module to the app. Does NOT instantiate and intitialize it yet.
    pub fn add_with_config<M: ModuleT>(mut self, config: M::Config) -> Self
    where
        M::Config: Default,
    {
        // allocate the config in the `module_configs` Bump for later use.
        let config: &M::Config = self.module_configs.alloc(config);
        let config_ptr: *const () = config as *const M::Config as *const ();

        // insert an entry for the added module, containing a monomorphized function pointer,
        // that can later be used to instantiate the module.
        let dep_type_ids = M::Dependencies::type_ids();
        let own_type_id = ModuleId::of::<M>();
        let added_module = AddedModule {
            dependencies: dep_type_ids,
            module: own_type_id,
            config_ptr,
            instantiate_module_fn: instantiate_module::<M>,
        };
        self.added_modules.insert(ModuleId::of::<M>(), added_module);
        self
    }

    /// tries to find a
    pub fn build(self) -> anyhow::Result<App> {
        let modules_bump: &'static Bump = Box::leak(Box::new(Bump::new()));
        let mut instantiated_modules: HashMap<ModuleId, InstantiatedModule> = HashMap::new();

        // determine an order of instantiation:
        let order = instantiation_order(&self.added_modules)?;

        // instantiate all the modules.
        for m_id in order.iter() {
            let m = self.added_modules.get(m_id).unwrap();
            // instantiate the module by calling the function (that was monomorphized before, to allow for type punning).
            m.instantiate(&mut instantiated_modules, modules_bump);
        }

        // call the initialize function on all modules:
        for m_id in order.iter() {
            let m = instantiated_modules.get(m_id).unwrap();
            // instantiate the module by calling the function (that was monomorphized before, to allow for type punning).
            m.initialize();
        }

        Ok(App {
            module_configs: self.module_configs,
            modules: modules_bump,
            instantiated_modules,
        })
    }
}

// basically finds an order of traversing the directed acyclic graph of dependencies, such that things that come first in the order, do not need any modules that come later in the order as dependencies.
// currently cannot detect infinite cycles. (todo!())
fn instantiation_order(modules: &HashMap<ModuleId, AddedModule>) -> anyhow::Result<Vec<ModuleId>> {
    /// fills `order` in a depth first way,
    fn depth_first_fill_order(
        m_id: &ModuleId,
        modules: &HashMap<ModuleId, AddedModule>,
        visitied: &mut HashSet<ModuleId>,
        order: &mut Vec<ModuleId>,
        dependency_chain: &Vec<ModuleId>,
    ) -> anyhow::Result<()> {
        if !visitied.contains(m_id) {
            visitied.insert(*m_id);

            if let Some(m) = modules.get(m_id) {
                let mut chain = dependency_chain.clone();
                chain.push(*m_id);
                for d_id in m.dependencies.iter() {
                    depth_first_fill_order(d_id, modules, visitied, order, &chain)?;
                }
            } else {
                let mut dep_chain_string = String::new();

                for d in dependency_chain.iter() {
                    dep_chain_string.push_str(&d.to_string());
                    dep_chain_string.push_str(" -> ");
                }

                return Err(anyhow!(
                    "Module {m_id} not found. It is needed as a dependency of ({dep_chain_string}{m_id})"
                ));
            }

            order.push(*m_id);
        }

        Ok(())
    }

    let mut visitied: HashSet<ModuleId> = HashSet::new();
    let mut order: Vec<ModuleId> = vec![];

    for m_id in modules.keys() {
        depth_first_fill_order(m_id, modules, &mut visitied, &mut order, &vec![])?;
    }

    Ok(order)
}

// struct DependencyGraph {
//     nodes: HashMap,
//     entry_points: HashM
// }

// struct Node {
//     module: AddedModule,
//     dependencies: Vec<TypeId>,
// }

struct AddedModule {
    dependencies: Vec<ModuleId>,
    module: ModuleId,
    config_ptr: *const (), // points to the stored config in the static Bump.
    instantiate_module_fn: fn(
        &AddedModule,
        instantiated_modules: &mut HashMap<ModuleId, InstantiatedModule>,
        modules_bump: &'static Bump,
    ) -> (),
}

impl AddedModule {
    fn instantiate(
        &self,
        instantiated_modules: &mut HashMap<ModuleId, InstantiatedModule>,
        modules_bump: &'static Bump,
    ) {
        (self.instantiate_module_fn)(self, instantiated_modules, modules_bump);
    }
}

struct InstantiatedModule {
    handle: UntypedHandle,
    initialize_module_fn: fn(&InstantiatedModule) -> (),
}

impl InstantiatedModule {
    /// Calls the type punned initialization function pointer for this module.
    fn initialize(&self) {
        (self.initialize_module_fn)(self);
    }
}

/// create the module in the `modules_bump` and adds it to the instantiated_modules
fn instantiate_module<M: ModuleT>(
    added_module: &AddedModule,
    instantiated_modules: &mut HashMap<ModuleId, InstantiatedModule>,
    modules_bump: &'static Bump,
) {
    let mut dep_handles: Vec<UntypedHandle> = vec![];
    for ty_id in added_module.dependencies.iter() {
        if let Some(m) = instantiated_modules.get(ty_id) {
            dep_handles.push(m.handle)
        } else {
            panic!("Cannot instantiate module {} because dependency not in instantiated_module_handles", std::any::type_name::<M>());
        }
    }

    let deps = M::Dependencies::from_untyped_handles(&dep_handles);
    let config: &M::Config = unsafe { &*(added_module.config_ptr as *const M::Config) };
    let module = M::new(config.clone(), deps);

    let module_ref = modules_bump.alloc(UnsafeCell::new(module));
    let handle = Handle::<M> { ptr: module_ref };
    instantiated_modules.insert(
        ModuleId::of::<M>(),
        InstantiatedModule {
            handle: handle.untyped(),
            initialize_module_fn: initialize_module::<M>,
        },
    );
}

/// Happens after all modules have been initialized. This is optional (trait fn often left empty) and not all modules use it.
fn initialize_module<M: ModuleT>(instantiated_module: &InstantiatedModule) {
    let handle: Handle<M> = instantiated_module.handle.typed();
    M::intialize(handle);
}

#[cfg(test)]
mod test {
    use super::{instantiation_order, AppBuilder, Handle, ModuleT};

    // /////////////////////////////////////////////////////////////////////////////
    // Some test structs that implement the Module trait.
    // /////////////////////////////////////////////////////////////////////////////

    struct RendererSettings;

    impl ModuleT for RendererSettings {
        type Config = ();
        type Dependencies = ();
        fn new(config: Self::Config, deps: Self::Dependencies) -> Self {
            println!("New RendererSettings created");
            RendererSettings
        }

        fn intialize(handle: Handle<Self>) {
            println!("RendererSettings Initialized");
        }
    }

    struct GraphicsContext;

    impl ModuleT for GraphicsContext {
        type Config = ();
        type Dependencies = ();
        fn new(config: Self::Config, deps: Self::Dependencies) -> Self {
            println!("New GraphicsContext created");
            GraphicsContext
        }

        fn intialize(handle: Handle<Self>) {
            println!("GraphicsContext Initialized");
        }
    }

    struct Renderer {
        ctx: Handle<GraphicsContext>,
        settings: Handle<RendererSettings>,
    }

    impl ModuleT for Renderer {
        type Config = ();
        type Dependencies = (Handle<RendererSettings>, Handle<GraphicsContext>);
        fn new(config: Self::Config, (settings, ctx): Self::Dependencies) -> Self {
            println!("New Renderer created");
            Renderer { settings, ctx }
        }

        fn intialize(handle: Handle<Self>) {
            println!("Renderer Initialized");
        }
    }

    struct LineRenderer {
        renderer: Handle<Renderer>,
    }

    impl ModuleT for LineRenderer {
        type Config = ();
        type Dependencies = Handle<Renderer>;
        fn new(config: Self::Config, renderer: Self::Dependencies) -> Self {
            println!("New LineRenderer created");
            LineRenderer { renderer }
        }

        fn intialize(handle: Handle<Self>) {
            println!("LineRenderer Initialized");
        }
    }

    fn app_builder() -> AppBuilder {
        let app = AppBuilder::new()
            .add::<LineRenderer>()
            .add::<GraphicsContext>()
            .add::<Renderer>()
            .add::<RendererSettings>();
        app
    }

    #[test]
    fn dependency_order() {
        let app1 = AppBuilder::new()
            .add::<LineRenderer>()
            .add::<GraphicsContext>()
            .add::<Renderer>()
            .add::<RendererSettings>();

        let app2 = AppBuilder::new()
            .add::<GraphicsContext>()
            .add::<Renderer>()
            .add::<RendererSettings>();

        let app3 = AppBuilder::new()
            .add::<LineRenderer>()
            .add::<GraphicsContext>()
            .add::<RendererSettings>();

        let app4 = AppBuilder::new()
            .add::<LineRenderer>()
            .add::<Renderer>()
            .add::<RendererSettings>();

        assert!(instantiation_order(&app1.added_modules).is_ok());
        assert!(instantiation_order(&app2.added_modules).is_ok());
        assert!(instantiation_order(&app3.added_modules).is_err());
        assert!(instantiation_order(&app4.added_modules).is_err());
    }

    #[test]
    fn instantiation() {
        let app1 = AppBuilder::new()
            .add::<LineRenderer>()
            .add::<GraphicsContext>()
            .add::<Renderer>()
            .add::<RendererSettings>();
        app1.build();
    }
}
