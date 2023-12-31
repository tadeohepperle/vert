use super::AppBuilder;

pub trait Plugin {
    fn add(&self, app: &mut AppBuilder);
}
