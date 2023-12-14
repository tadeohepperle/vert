use glam::vec3;
use vert_framework::{
    app::App,
    flow::Flow,
    modules::{
        assets::fetchable_asset::{AssetSource, ImageAsset, LoadingAsset},
        graphics::elements::color_mesh::SingleColorMesh,
        Modules,
    },
    state::StateT,
};

pub enum MyImg {
    Loading(LoadingAsset<ImageAsset>),
    Loaded(ImageAsset),
    Error(anyhow::Error),
}

pub struct MyState {
    img: MyImg,
}

impl StateT for MyState {
    async fn initialize(modules: &mut Modules) -> anyhow::Result<Self> {
        // let system_id = modules.add_system();

        let source: AssetSource = "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcQ6ttcadfmj7-DuD3JDobVHUXcGFNcreMPMjKm3nq-keA&s".into();
        dbg!(&source);
        let loading_asset = source.fetch_in_background();
        Ok(MyState {
            img: MyImg::Loading(loading_asset),
        })
    }

    fn update(&mut self, modules: &mut Modules) -> Flow {
        let label = match &self.img {
            MyImg::Loading(_) => "Loading".to_string(),
            MyImg::Loaded(img) => format!("Loaded, dims: {:?}", img.rgba.dimensions()),
            MyImg::Error(err) => format!("Error: {err}",),
        };

        egui::Window::new("Asset test").show(&mut modules.egui(), |ui| {
            ui.label(label);
        });

        match &mut self.img {
            MyImg::Loading(l) => {
                match l.get() {
                    Some(Ok(img)) => {
                        println!(
                            "Received image after {} seconds",
                            modules.time().total_secs()
                        );
                        self.img = MyImg::Loaded(img);
                    }
                    Some(Err(err)) => {
                        println!("Received error: {err}");
                        self.img = MyImg::Error(err);
                    }
                    None => {}
                };
            }
            _ => {}
        }

        Flow::Continue
    }

    fn prepare(&mut self, modules: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {}
}

fn main() {
    App::<MyState>::run().unwrap();
}
