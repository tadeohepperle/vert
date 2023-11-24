#![feature(ptr_alignment_type)]
#![feature(min_specialization)]

pub mod app;
pub mod arena;
pub mod component;
pub mod events;
pub mod system;
pub mod trait_reflection;
pub mod world;

pub mod prelude {
    pub use crate::component::Component;
    pub use crate::system::System;
    pub use crate::trait_reflection::{
        Implements, MultipleReflectedTraits, ReflectedTrait, ReflectedTraitInv,
    };
    pub use vert_macros::reflect;
}
