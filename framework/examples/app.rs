use vert_framework::{app::App, state::StateT};

pub struct MyState {}

impl StateT for MyState {
    async fn initialize(modules: &vert_framework::modules::Modules) -> anyhow::Result<Self> {
        Ok(MyState {})
    }
}

fn main() {
    App::<MyState>::run().unwrap();
}
