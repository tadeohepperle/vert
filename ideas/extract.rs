// use std::{
//     any::TypeId,
//     cell::UnsafeCell,
//     collections::BTreeMap,
//     sync::{Arc, Mutex},
// };

// /*

// Can we save what has been already extracted in a const generic?
// Lets say that we have a type id for each thing that could be extracted: u128

// What kind of container could hold data about the fact what components have been extracted and what not?
// Requirements for such a container:

// actually each part needs only one bit.
// maybe the extraction id can be computed at runtime?
// We have the problem that actually the

// Maybe there is a Hay trait that is implemented for the world:
// - it can tell us how many needles are there -> N or unlimited.
// Imagine Gamestate fixed number of 7 elements and 30-100 components:
// Then we have a bitset like this:
// |0000000|00000000000000....
// | fixed |    dynamic

// Example: We create a Haystack of the world at the start of the frame.
// Then we give it to every system that wants it. While doing so, we more and more give some things away

// What if we go into the render system,
// where we iterate over all elements of the world implementing a certain trait?
// This should look down all the related components for that.
// Now each of these may need access to the worlds resources as well? or something.

// Okay different example: If we want to spawn a component from some function, what do we need?
// The components componentresource function may

// Example:

// Update system:
//     goes over every struct implementing Update
//     go over first arena:
//         go over first struct:
//             spawn a new component that may also include update. => When?
//             spawning this new component might need some setup code to setup a component resource:
//                 this is a system, taking in a subset of the world.

// We want to be able to take a world:

// */
// // #[derive(Debug, Default)]
// // pub struct Accesses {
// //     inner: Vec<Access>,
// // }

// // #[derive(Debug)]
// // pub struct Access {
// //     needle: TypeId,
// //     ty: AccessType,
// // }

// // #[derive(Debug)]
// // pub enum AccessType {
// //     Mut,
// //     Ref { ref_count: usize },
// // }

// // pub trait Extract<N>: Sized {
// //     fn extract(&self) -> &N;
// // }

// // // struct InExtraction<'hay, Hay> {
// // //     inner: &'hay mut Hay,
// // // }

// // // pub trait Extract<'hay, Needle>: Sized {
// // //     fn extract(hay: &'hay mut InExtraction<'hay, Self>) -> Needle;
// // // }

// pub struct SharedAccum<T> {
//     inner: Arc<Mutex<Accum<T>>>,
// }

// pub struct Accum<T> {
//     inner: T,
//     accesses: BTreeMap<TypeId, Access>,
// }
// impl<T> Accum<T> {
//     pub fn new(t: T) -> Self {
//         Self {
//             inner: t,
//             accesses: Default::default(),
//         }
//     }

//     pub fn extract_ref<'a, N>(&'a self) -> anyhow::Result<Ref<'a, N>>
//     where
//         T: Extract<N>,
//     {
//         // self.accesses = {}
//         // chec

//         todo!()
//     }
// }

// struct Ref<'a, N>(&'a N);

// #[derive(Debug)]
// pub enum Access {
//     Mut,
//     Ref { ref_count: usize },
// }

// trait Extract<N> {
//     fn extract(&self) -> &N;
// }

// pub struct World {
//     a: A,
//     b: B,
//     c: C,
// }

// struct A;
// struct B;
// struct C;

// impl Extract<A> for World {
//     fn extract(&self) -> &A {
//         &self.a
//     }
// }

// impl Extract<B> for World {
//     fn extract(&self) -> &B {
//         &self.b
//     }
// }

// impl Extract<C> for World {
//     fn extract(&self) -> &C {
//         &self.c
//     }
// }

// fn main() {
//     let w = World { a: A, b: B, c: C };
//     let accum = Accum::new(w);
// }

pub fn main() {}
