#![feature(ptr_alignment_type)]
#![feature(const_type_id)]
#![feature(const_type_name)]
#![feature(associated_type_defaults)]
#![feature(associated_type_bounds)]

pub mod app;
pub mod arena;
pub mod component;
pub mod events;
pub mod system;
pub mod trait_reflection;
pub mod world;

pub mod prelude {
    pub use crate::reflect;
    pub use crate::system::System;
    pub use crate::trait_reflection::{
        DynTrait, Implementor, MultipleReflectedTraits, VTable, VTablePtrWithMeta,
    };
    pub use smallvec::*;
}
