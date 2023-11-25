use std::any::TypeId;

use crate::trait_reflection::Implementor;

pub trait Component: Sized + 'static + Implementor {
    fn id() -> TypeId {
        TypeId::of::<Self>()
    }
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl<T> Component for T where T: Sized + 'static + Implementor {}
