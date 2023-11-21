use std::{marker::PhantomData, mem::MaybeUninit};

use crate::component::Component;

// /////////////////////////////////////////////////////////////////////////////
// Traits struct that can hold many dyn...
// /////////////////////////////////////////////////////////////////////////////

// #[derive(Debug, Clone)]
// pub struct Traits<
//     A: ?Sized = (),
//     B: ?Sized = (),
//     C: ?Sized = (),
//     D: ?Sized = (),
//     E: ?Sized = (),
//     F: ?Sized = (),
// > {
//     a: PhantomData<A>,
//     b: PhantomData<B>,
//     c: PhantomData<C>,
//     d: PhantomData<D>,
//     e: PhantomData<E>,
//     f: PhantomData<F>,
// }

// impl<A, B, C, D, E, F> Default for Traits<A, B, C, D, E, F>
// where
//     A: ?Sized,
//     B: ?Sized,
//     C: ?Sized,
//     D: ?Sized,
//     E: ?Sized,
//     F: ?Sized,
// {
//     fn default() -> Self {
//         Self {
//             a: Default::default(),
//             b: Default::default(),
//             c: Default::default(),
//             d: Default::default(),
//             e: Default::default(),
//             f: Default::default(),
//         }
//     }
// }

// pub struct Teller<T: ?Sized> {
//     phantom: PhantomData<T>,
// }

// pub fn main() {
//     trait TraitB {}
//     trait TraitC {}
//     let a: Traits<dyn TraitB, dyn TraitC> = Default::default();
//     // let t: Teller<(dyn TraitB, dyn TraitC)> = Teller {
//     //     phantom: PhantomData,
//     // };
// }

// /////////////////////////////////////////////////////////////////////////////
// Dyn struct as a wrapper around any trait.
// Can be put into tuples.
// /////////////////////////////////////////////////////////////////////////////

struct Dyn<T: ?Sized>(PhantomData<T>);

// /////////////////////////////////////////////////////////////////////////////
// Implements Trait
// e.g. used as Implements<dyn T>
// /////////////////////////////////////////////////////////////////////////////

trait Implements<T: ?Sized + TraitCompanion> {
    unsafe fn uninit_trait_obj() -> Option<&'static <T as TraitCompanion>::Dyn>;
}
impl<T: TraitCompanion, C: Component> Implements<T> for C {
    default unsafe fn uninit_trait_obj() -> Option<&'static <T as TraitCompanion>::Dyn> {
        None
    }
}

// impl<T, X: Reflectable<T>, C: Component> Implements<X> for C {
//     default unsafe fn uninit_trait_obj() -> Option<&'static X> {
//         None
//     }
// }

// /////////////////////////////////////////////////////////////////////////////
// Some example structs/traits
// /////////////////////////////////////////////////////////////////////////////

trait TraitCompanion {
    type Dyn: ?Sized + 'static;
}

struct X1S {}
impl TraitCompanion for X1S {
    type Dyn = dyn X1;
}

struct X2S {}
impl TraitCompanion for X2S {
    type Dyn = dyn X2;
}

struct X3S {}
impl TraitCompanion for X3S {
    type Dyn = dyn X3;
}

struct Circle;
struct Rect;
struct Point;

impl Component for Circle {}
impl Component for Rect {}
impl Component for Point {}

trait X1 {}
trait X2 {}
trait X3 {}

impl X1 for Point {}
impl Implements<X1S> for Point {
    unsafe fn uninit_trait_obj() -> Option<&'static dyn X1> {
        const POINT: Point = unsafe { MaybeUninit::<Point>::uninit().assume_init() };
        Some(&POINT as &'static dyn X1)
    }
}

fn can_get_vtable<C: Component, T: TraitCompanion>() -> bool {
    unsafe { <C as Implements<T>>::uninit_trait_obj().is_some() }
}

#[test]
fn main() {
    dbg!(can_get_vtable::<Point, X1S>());
}
