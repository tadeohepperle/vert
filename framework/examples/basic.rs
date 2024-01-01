use vert_framework::{
    modules::{DefaultDependencies, DefaultModules, Schedule},
    utils::Timing,
    AppBuilder, Module,
};

fn main() {
    let mut app = AppBuilder::new();
    app.add_plugin(DefaultModules);
    app.add::<MyApp>();
    app.run().unwrap();
}

struct MyApp {
    deps: DefaultDependencies,
}

impl Module for MyApp {
    type Config = ();

    type Dependencies = DefaultDependencies;

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        Ok(MyApp { deps })
    }

    fn intialize(handle: vert_framework::Handle<Self>) -> anyhow::Result<()> {
        let scheduler = handle.deps.scheduler.get_mut();
        scheduler.register(handle, Schedule::Update, Timing::DEFAULT, Self::update);
        Ok(())
    }
}

impl MyApp {
    fn update(&mut self) {
        if self
            .deps
            .input
            .keys()
            .just_pressed(winit::keyboard::KeyCode::Space)
        {
            let fps = self.deps.time.fps();
            println!("Fps: {fps}");
        }
        if self
            .deps
            .input
            .keys()
            .just_pressed(winit::keyboard::KeyCode::Escape)
        {
            self.deps.scheduler.request_exit("Escape");
        }

        let egui = self.deps.egui;
        let mut egui_ctx = egui.context();
        egui::Window::new("Hellow World").show(&mut egui_ctx, |ui| {
            ui.label("Wow!");
        });
    }
}
