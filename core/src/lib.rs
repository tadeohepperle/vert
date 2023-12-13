pub mod arenas;
pub mod component;
pub mod trait_reflection;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::reflect;
    pub use crate::trait_reflection::{
        DynTrait, Implementor, MultipleReflectedTraits, VTable, VTablePtrWithMeta,
    };
    pub use smallvec::*;
}
