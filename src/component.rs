use std::any::TypeId;

use crate::{trait_reflection::Implementor, world::World};

pub trait Component: Sized + 'static + Implementor {
    fn id() -> TypeId {
        TypeId::of::<Self>()
    }
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
    type ComponentResource: Sized + 'static + Implementor = ();
}

pub trait ComponentForState<W>: Component<ComponentResource: NewFromMut<W>> {}

impl<T, W> ComponentForState<W> for T where T: Component<ComponentResource: NewFromMut<W>> {}

impl<T> Component for T where T: Sized + 'static + Implementor {}

/// todo!() change the parameters of this functions to anything implementing FromWorld.
pub trait NewFromMut<W> {
    fn new_from_mut(state: &mut W) -> Self;
}

impl<W> NewFromMut<W> for () {
    fn new_from_mut(state: &mut W) -> Self {}
}
