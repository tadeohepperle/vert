use std::{
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::anyhow;
use bumpalo::Bump;

pub mod child_module;
pub mod dependencies;
pub mod function_handle;
pub mod handle;
pub mod module;
pub mod plugin;

pub use dependencies::Dependencies;
pub use function_handle::{FunctionHandle, RefFunctionHandle};
pub use handle::{Handle, UntypedHandle};
pub use module::{MainModule, Module, ModuleId};
pub use plugin::Plugin;

pub struct AllModules {
    _inner: Arc<HashMap<ModuleId, UntypedHandle>>,
}

impl AllModules {
    fn get<M: Module>(&self) -> Option<Handle<M>> {
        self._inner
            .get(&ModuleId::of::<M>())
            .map(|e| e.typed::<M>())
    }
}

pub struct App {
    /// Bump Allocator in which all Modules are allocated.
    _modules: &'static bumpalo::Bump,
    /// handles to all modules by their module id
    all_modules: AllModules,
    main_module: ModuleId,
}

impl App {
    pub fn build() -> AppBuilder {
        AppBuilder::new()
    }

    pub fn all_modules(&self) {}

    // pub fn add_dynamic_module(&self)
}

/// To run an application, add any number of modules and exactly one MainModule to the AppBuilder, then call `AppBuilder::run()`;
///
/// The order in which you add modules and the main module schould not matter at all.
pub struct AppBuilder {
    module_configs: bumpalo::Bump,
    added_modules: HashMap<ModuleId, AddedModule>,
    main_module: Option<AddedMainModule>,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    pub fn new() -> Self {
        AppBuilder {
            module_configs: bumpalo::Bump::new(),
            added_modules: HashMap::new(),
            main_module: None,
        }
    }

    pub fn add_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
        plugin.add(self);
        self
    }

    /// Adds a module to the app. Does NOT instantiate and intitialize it yet.
    pub fn add_main_module<M: MainModule>(&mut self) -> &mut Self
    where
        M::Config: Default,
    {
        let config: M::Config = Default::default();
        self.add_main_module_with_config::<M>(config)
    }

    /// Adds a module to the app. Does NOT instantiate and intitialize it yet.
    pub fn add_main_module_with_config<M: MainModule>(&mut self, config: M::Config) -> &mut Self {
        let main_module = AddedMainModule {
            module_id: ModuleId::of::<M>(),
            run_main_module_fn: run_main_module::<M>,
        };
        if let Some(main_before) = &self.main_module {
            if main_before.module_id != main_module.module_id {
                panic!("Main Module {} cannot be added, because Main Module {} was already registered before.", main_module.module_id, main_before.module_id);
            }

            let conf_ptr_before = self
                .added_modules
                .get(&main_module.module_id)
                .unwrap()
                .config_ptr;
            let config_before: &M::Config = unsafe { &*(conf_ptr_before as *const M::Config) };
            if *config_before != config {
                panic!("Main Module {} cannot be added, because it was added with a different config before.\nBefore: {:?}\n Current: {:?}", main_module.module_id, config_before, &config );
            }
        }
        self.main_module = Some(main_module);
        self.add_with_config::<M>(config)
    }

    /// Adds a module to the app. Does NOT instantiate and intitialize it yet.
    pub fn add<M: Module>(&mut self) -> &mut Self
    where
        M::Config: Default,
    {
        let config: M::Config = Default::default();
        self.add_with_config::<M>(config)
    }

    /// Adds a module to the app. Does NOT instantiate and intitialize it yet.
    pub fn add_with_config<M: Module>(&mut self, config: M::Config) -> &mut Self {
        // allocate the config in the `module_configs` Bump for later use.
        let config: &M::Config = self.module_configs.alloc(config);
        let config_ptr: *const () = config as *const M::Config as *const ();

        // insert an entry for the added module, containing a monomorphized function pointer,
        // that can later be used to instantiate the module.
        let dep_type_ids = M::Dependencies::type_ids();
        let module_id = ModuleId::of::<M>();
        let added_module = AddedModule {
            dependencies: dep_type_ids,
            module_id,
            config_ptr,
            instantiate_module_fn: instantiate_module::<M>,
        };

        if let Some(module_before) = self.added_modules.get(&module_id) {
            // check if the config is the same here: (if yes, the two can be considered the same module, all good)
            let config_before = unsafe { &*(module_before.config_ptr as *const M::Config) };
            if config_before != config {
                panic!("Module {module_id} cannot be added twice, with different configs: {config_before:?} and {config:?}");
            }
        }

        self.added_modules.insert(module_id, added_module);
        self
    }

    /// tries to builds a valid dependency graph between all modules (no cycles) and instantiates them.
    /// A `MainModule` is provided as a type parameter. This Module is also just a regular module and is added to the AppBuilder as well.
    /// It specifies the `main` function, that should be run after all modules are created.
    ///
    /// The `MainModule` can be a dependency of other modules. For example the MainModule could expose
    /// scheduling functions that can be used by other modules in their `initialize` functions,
    /// to register their own handles in an update or render loop.  
    ///
    /// On the other hand, the `MainModule` can also have other modules as dependencies:
    /// E.g. if the app, assumes there is a window, a GraphicsContext can be created first,
    /// and be used as a dependency of the main module.
    ///
    /// ## Quick overview of App lifecycle:
    ///
    /// - Instantiate all modules in a valid order: pass dependencies to the modules that need them
    /// - Initialize all modules: Here each Module has the chance to do something with a handle to itself. Useful for registering the own handle in other modules, e.g. as a RenderPass
    /// - Run the `main()` function of the `MainModule`.
    pub fn run(self) -> anyhow::Result<()> {
        let Some(added_main_module) = self.main_module else {
            return Err(anyhow!("No Main Module registered in the AppBuilder!"));
        };

        let modules_bump: &'static Bump = Box::leak(Box::new(Bump::new()));
        let mut instantiated_modules: HashMap<ModuleId, InstantiatedModule> = HashMap::new();

        // determine an order of instantiation:
        let order = instantiation_order(&self.added_modules)?;

        // instantiate all the modules.
        for m_id in order.iter() {
            let m = self.added_modules.get(m_id).unwrap();
            // instantiate the module by calling the function (that was monomorphized before, to allow for type punning).
            m.instantiate(&mut instantiated_modules, modules_bump)?;
        }

        // call the initialize function on all modules:
        for m_id in order.iter() {
            let m = instantiated_modules.get(m_id).unwrap();
            // instantiate the module by calling the function (that was monomorphized before, to allow for type punning).
            m.initialize()?;
        }

        let all_modules: HashMap<ModuleId, UntypedHandle> = instantiated_modules
            .into_iter()
            .map(|(k, v)| (k, v.handle))
            .collect();

        // note: configs allocated in module_configs leaks, maybe it should be deallocated here. See bumpalo::boxed.
        let app = App {
            _modules: modules_bump,
            all_modules: AllModules {
                _inner: Arc::new(all_modules),
            },
            main_module: added_main_module.module_id,
        };

        // todo!() dealloc modules and configs bumps
        // Also: cross fingers that no references to them are around after main module main.

        (added_main_module.run_main_module_fn)(&app)
    }
}

/// basically finds an order of traversing the directed acyclic graph of dependencies, such that things that come first in the order, do not need any modules that come later in the order as dependencies.
/// currently cannot detect infinite cycles. (todo!())
///
/// Returns an error if no instantiation order that satisfies the dependency chain can be found.
fn instantiation_order(modules: &HashMap<ModuleId, AddedModule>) -> anyhow::Result<Vec<ModuleId>> {
    fn dependency_chain_string(m_id: &ModuleId, dependency_chain: &Vec<ModuleId>) -> String {
        let mut dep_chain_string = String::new();

        for d in dependency_chain.iter() {
            dep_chain_string.push_str(&d.to_string());
            dep_chain_string.push_str(" -> ");
        }

        format!("{dep_chain_string}{m_id}")
    }

    /// fills `order` in a depth first way,
    fn depth_first_fill_order(
        m_id: &ModuleId,
        modules: &HashMap<ModuleId, AddedModule>,
        visitied: &mut HashSet<ModuleId>,
        visited_in_this_run: &mut HashSet<ModuleId>,
        order: &mut Vec<ModuleId>,
        dependency_chain: &Vec<ModuleId>,
    ) -> anyhow::Result<()> {
        if !visitied.contains(m_id) {
            visitied.insert(*m_id);

            if visited_in_this_run.contains(m_id) {
                dbg!(&visited_in_this_run);
                let dep_chain_string = dependency_chain_string(m_id, dependency_chain);
                return Err(anyhow!("Recursive dependency chain: {dep_chain_string}"));
            }
            visited_in_this_run.insert(*m_id);

            if let Some(m) = modules.get(m_id) {
                let mut chain = dependency_chain.clone();
                chain.push(*m_id);
                for d_id in m.dependencies.iter() {
                    depth_first_fill_order(
                        d_id,
                        modules,
                        visitied,
                        visited_in_this_run,
                        order,
                        &chain,
                    )?;
                }
            } else {
                let dep_chain_string = dependency_chain_string(m_id, dependency_chain);
                return Err(anyhow!(
                    "Module {m_id} not found. Needed in: {dep_chain_string}"
                ));
            }

            order.push(*m_id);
        }

        Ok(())
    }

    let mut visitied: HashSet<ModuleId> = HashSet::new();
    let mut order: Vec<ModuleId> = vec![];

    for m_id in modules.keys() {
        depth_first_fill_order(
            m_id,
            modules,
            &mut visitied,
            &mut HashSet::new(),
            &mut order,
            &vec![],
        )?;
    }

    Ok(order)
}

#[allow(dead_code)]
struct AddedModule {
    dependencies: Vec<ModuleId>,
    module_id: ModuleId,
    config_ptr: *const (), // points to the stored config in the static Bump.
    /// This function instantiates the module, adding it to the instantiated modueles hashmap (also inserting the initialization function there)
    instantiate_module_fn: fn(
        &AddedModule,
        instantiated_modules: &mut HashMap<ModuleId, InstantiatedModule>,
        modules_bump: &'static Bump,
    ) -> anyhow::Result<()>,
}

impl AddedModule {
    fn instantiate(
        &self,
        instantiated_modules: &mut HashMap<ModuleId, InstantiatedModule>,
        modules_bump: &'static Bump,
    ) -> anyhow::Result<()> {
        (self.instantiate_module_fn)(self, instantiated_modules, modules_bump)
    }
}

struct AddedMainModule {
    module_id: ModuleId,
    /// monomorphized function pointer for
    run_main_module_fn: fn(&App) -> anyhow::Result<()>,
}

struct InstantiatedModule {
    handle: UntypedHandle,
    initialize_module_fn: fn(&InstantiatedModule) -> anyhow::Result<()>,
}

impl InstantiatedModule {
    /// Calls the type punned initialization function pointer for this module.
    fn initialize(&self) -> anyhow::Result<()> {
        (self.initialize_module_fn)(self)
    }
}

/// create the module in the `modules_bump` and adds it to the instantiated_modules
fn instantiate_module<M: Module>(
    added_module: &AddedModule,
    instantiated_modules: &mut HashMap<ModuleId, InstantiatedModule>,
    modules_bump: &'static Bump,
) -> anyhow::Result<()> {
    let mut dep_handles: Vec<UntypedHandle> = vec![];
    for ty_id in added_module.dependencies.iter() {
        if let Some(m) = instantiated_modules.get(ty_id) {
            dep_handles.push(m.handle)
        } else {
            panic!("Cannot instantiate module {} because dependency not in instantiated_module_handles", ModuleId::of::<M>());
        }
    }

    let deps = M::Dependencies::from_untyped_handles(&dep_handles);
    let config: &M::Config = unsafe { &*(added_module.config_ptr as *const M::Config) };
    let module = M::new(config.clone(), deps)?;

    let module_ref = modules_bump.alloc(UnsafeCell::new(module));
    let handle = Handle::<M> { ptr: module_ref };
    instantiated_modules.insert(
        ModuleId::of::<M>(),
        InstantiatedModule {
            handle: handle.untyped(),
            initialize_module_fn: initialize_module::<M>,
        },
    );
    Ok(())
}

/// Happens after all modules have been initialized. This is optional (trait fn often left empty) and not all modules use it.
fn initialize_module<M: Module>(instantiated_module: &InstantiatedModule) -> anyhow::Result<()> {
    let handle: Handle<M> = instantiated_module.handle.typed();
    M::intialize(handle)
}

fn run_main_module<M: MainModule>(app: &App) -> anyhow::Result<()> {
    assert_eq!(ModuleId::of::<M>(), app.main_module);
    let mut main_module_handle = app
        .all_modules
        .get::<M>()
        .ok_or_else(|| anyhow!("Main Module {} not found in App", app.main_module))?;
    main_module_handle.main(app)
}

#[cfg(test)]
mod test {

    use super::{instantiation_order, AppBuilder, Handle, MainModule, Module};

    // /////////////////////////////////////////////////////////////////////////////
    // Some test structs that implement the Module trait.
    // /////////////////////////////////////////////////////////////////////////////

    struct RendererSettings;

    impl Module for RendererSettings {
        type Config = ();
        type Dependencies = ();
        fn new(_config: Self::Config, _deps: Self::Dependencies) -> anyhow::Result<Self> {
            println!("New RendererSettings created");
            Ok(RendererSettings)
        }

        fn intialize(_handle: Handle<Self>) -> anyhow::Result<()> {
            println!("RendererSettings Initialized");
            Ok(())
        }
    }

    struct GraphicsContext;

    impl Module for GraphicsContext {
        type Config = ();
        type Dependencies = ();
        fn new(_config: Self::Config, _deps: Self::Dependencies) -> anyhow::Result<Self> {
            println!("New GraphicsContext created");
            Ok(GraphicsContext)
        }

        fn intialize(_handle: Handle<Self>) -> anyhow::Result<()> {
            println!("GraphicsContext Initialized");
            Ok(())
        }
    }

    #[allow(dead_code)]
    struct Renderer {
        ctx: Handle<GraphicsContext>,
        settings: Handle<RendererSettings>,
    }

    impl Module for Renderer {
        type Config = ();
        type Dependencies = (Handle<RendererSettings>, Handle<GraphicsContext>);
        fn new(_config: Self::Config, (settings, ctx): Self::Dependencies) -> anyhow::Result<Self> {
            println!("New Renderer created");
            Ok(Renderer { settings, ctx })
        }

        fn intialize(_handle: Handle<Self>) -> anyhow::Result<()> {
            println!("Renderer Initialized");
            Ok(())
        }
    }

    struct C;
    impl Module for C {
        type Config = ();
        type Dependencies = Handle<A>;
        fn new(_config: Self::Config, _deps: Self::Dependencies) -> anyhow::Result<Self> {
            Ok(C)
        }
    }

    struct A;
    impl Module for A {
        type Config = ();
        type Dependencies = Handle<B>;
        fn new(_config: Self::Config, _deps: Self::Dependencies) -> anyhow::Result<Self> {
            Ok(A)
        }
    }

    struct B;
    impl Module for B {
        type Config = ();
        type Dependencies = Handle<A>;
        fn new(_config: Self::Config, _deps: Self::Dependencies) -> anyhow::Result<Self> {
            Ok(B)
        }
    }

    #[allow(dead_code)]
    struct LineRenderer {
        renderer: Handle<Renderer>,
    }

    impl Module for LineRenderer {
        type Config = ();
        type Dependencies = Handle<Renderer>;
        fn new(_config: Self::Config, renderer: Self::Dependencies) -> anyhow::Result<Self> {
            println!("New LineRenderer created");
            Ok(LineRenderer { renderer })
        }

        fn intialize(_handle: Handle<Self>) -> anyhow::Result<()> {
            println!("LineRenderer Initialized");
            Ok(())
        }
    }

    struct MainMod {}

    impl Module for MainMod {
        type Config = ();

        type Dependencies = ();

        fn new(_config: Self::Config, _deps: Self::Dependencies) -> anyhow::Result<Self> {
            println!("New MainMod created.");
            Ok(MainMod {})
        }
    }

    impl MainModule for MainMod {
        fn main(&mut self, _app: &super::App) -> anyhow::Result<()> {
            println!("Running Main");
            Ok(())
        }
    }

    #[test]
    fn dependency_order() {
        let mut app1 = AppBuilder::new();
        app1.add::<LineRenderer>()
            .add::<GraphicsContext>()
            .add::<Renderer>()
            .add::<RendererSettings>();

        let mut app2 = AppBuilder::new();
        app2.add::<GraphicsContext>()
            .add::<Renderer>()
            .add::<RendererSettings>();

        let mut app3 = AppBuilder::new();
        app3.add::<LineRenderer>()
            .add::<GraphicsContext>()
            .add::<RendererSettings>();

        let mut app4 = AppBuilder::new();
        app4.add::<LineRenderer>()
            .add::<Renderer>()
            .add::<RendererSettings>();

        // recursive chain not possible:
        let mut apprec = AppBuilder::new();
        apprec.add::<A>().add::<B>();
        let mut apprec2 = AppBuilder::new();
        apprec2.add::<C>().add::<A>().add::<B>();

        assert!(instantiation_order(&app1.added_modules).is_ok());
        assert!(instantiation_order(&app2.added_modules).is_ok());
        assert!(instantiation_order(&app3.added_modules).is_err());
        assert!(instantiation_order(&app4.added_modules).is_err());
        assert!(instantiation_order(&apprec.added_modules).is_err());
        assert!(instantiation_order(&apprec2.added_modules).is_err());
    }

    #[test]
    fn instantiation() {
        let mut app1 = AppBuilder::new();
        app1.add::<LineRenderer>()
            .add::<GraphicsContext>()
            .add::<Renderer>()
            .add::<RendererSettings>()
            .add_main_module::<MainMod>();

        app1.run().unwrap();
    }
}
