use std::any::Any;

pub type ComponentID = u64;

pub trait Component: 'static + Any {
    // type Singleton: Default;
}
