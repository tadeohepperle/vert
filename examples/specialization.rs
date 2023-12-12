// //! This example shows a technique of dynamically getting V-tables for traits.
// //! These Vtables can be retrieved once and stored for later dynamic dispatch.

// #![feature(min_specialization)]

// use std::{any::TypeId, mem::MaybeUninit};

// use smallvec::{smallvec, SmallVec};

// trait Component {}

// const PTR_SIZE: usize = std::mem::size_of::<usize>();
// trait Collect: 'static {
//     type Dyn: ?Sized + 'static;

//     unsafe fn vtable_pointer<C: Component>() -> Option<*const usize>
//     where
//         Self: Sized,
//     {
//         let uninit_dyn: Option<&'static Self::Dyn> = <C as Implements<Self>>::uninit_trait_obj();

//         match uninit_dyn {
//             Some(trait_object) => {
//                 // this is a fat pointer: (data + vtable)
//                 assert_eq!(std::mem::size_of::<&Self::Dyn>(), PTR_SIZE * 2);
//                 // lets get a pointer to just the vtable:
//                 let data_pointer: *const usize = &trait_object as *const _ as *const usize;
//                 let vtable_pointer = data_pointer.add(1);
//                 Some(vtable_pointer)
//             }
//             None => None,
//         }
//     }
// }

// trait Implements<X: Collect> {
//     unsafe fn uninit_trait_obj() -> Option<&'static X::Dyn>;
// }

// impl<X: Collect, C: Component> Implements<X> for C {
//     default unsafe fn uninit_trait_obj() -> Option<&'static <X as Collect>::Dyn> {
//         None
//     }
// }

// // /////////////////////////////////////////////////////////////////////////////
// // Examples
// //          X1    X2    X3
// // Circle   x
// // Rect     x     x
// // Point    x     x     x
// //
// // /////////////////////////////////////////////////////////////////////////////

// struct Circle;
// struct Rect;
// struct Point;

// impl Component for Circle {}
// impl Component for Rect {}
// impl Component for Point {}

// struct CollectX1 {}
// trait X1 {}

// impl Collect for CollectX1 {
//     type Dyn = dyn X1;
// }
// struct CollectX2 {}
// trait X2 {}

// impl Collect for CollectX2 {
//     type Dyn = dyn X2;
// }

// struct CollectX3 {}
// trait X3 {}

// impl Collect for CollectX3 {
//     type Dyn = dyn X3;
// }

// impl X1 for Circle {}
// impl Implements<CollectX1> for Circle {
//     unsafe fn uninit_trait_obj() -> Option<&'static <CollectX1 as Collect>::Dyn> {
//         const CIRCLE: Circle = unsafe { MaybeUninit::<Circle>::uninit().assume_init() };
//         Some(&CIRCLE as &'static <CollectX1 as Collect>::Dyn)
//     }
// }

// impl X1 for Rect {}
// impl Implements<CollectX1> for Rect {
//     unsafe fn uninit_trait_obj() -> Option<&'static <CollectX1 as Collect>::Dyn> {
//         const RECT: Rect = unsafe { MaybeUninit::<Rect>::uninit().assume_init() };
//         Some(&RECT as &'static <CollectX1 as Collect>::Dyn)
//     }
// }

// impl X2 for Rect {}
// impl Implements<CollectX2> for Rect {
//     unsafe fn uninit_trait_obj() -> Option<&'static <CollectX2 as Collect>::Dyn> {
//         const RECT: Rect = unsafe { MaybeUninit::<Rect>::uninit().assume_init() };
//         Some(&RECT as &'static <CollectX2 as Collect>::Dyn)
//     }
// }

// impl X1 for Point {}
// impl Implements<CollectX1> for Point {
//     unsafe fn uninit_trait_obj() -> Option<&'static <CollectX1 as Collect>::Dyn> {
//         const POINT: Point = unsafe { MaybeUninit::<Point>::uninit().assume_init() };
//         Some(&POINT as &'static <CollectX1 as Collect>::Dyn)
//     }
// }

// impl X2 for Point {}
// impl Implements<CollectX2> for Point {
//     unsafe fn uninit_trait_obj() -> Option<&'static <CollectX2 as Collect>::Dyn> {
//         const POINT: Point = unsafe { MaybeUninit::<Point>::uninit().assume_init() };
//         Some(&POINT as &'static <CollectX2 as Collect>::Dyn)
//     }
// }

// impl X3 for Point {}
// impl Implements<CollectX3> for Point {
//     unsafe fn uninit_trait_obj() -> Option<&'static <CollectX3 as Collect>::Dyn> {
//         const POINT: Point = unsafe { MaybeUninit::<Point>::uninit().assume_init() };
//         Some(&POINT as &'static <CollectX3 as Collect>::Dyn)
//     }
// }

// fn can_get_vtable<C: Component, X: Collect>() -> bool {
//     unsafe { <C as Implements<X>>::uninit_trait_obj().is_some() }
// }

// pub fn main() {
//     println!(
//         "Circle can get a vtable for X1: {:?}",
//         can_get_vtable::<Circle, CollectX1>()
//     );
//     println!(
//         "Circle can get a vtable for X2: {:?}",
//         can_get_vtable::<Circle, CollectX2>()
//     );
//     println!(
//         "Circle can get a vtable for X3: {:?}",
//         can_get_vtable::<Circle, CollectX3>()
//     );

//     println!(
//         "Rect can get a vtable for X1: {:?}",
//         can_get_vtable::<Rect, CollectX1>()
//     );
//     println!(
//         "Rect can get a vtable for X2: {:?}",
//         can_get_vtable::<Rect, CollectX2>()
//     );
//     println!(
//         "Rect can get a vtable for X3: {:?}",
//         can_get_vtable::<Rect, CollectX3>()
//     );

//     println!(
//         "Point can get a vtable for X1: {:?}",
//         can_get_vtable::<Point, CollectX1>()
//     );
//     println!(
//         "Point can get a vtable for X2: {:?}",
//         can_get_vtable::<Point, CollectX2>()
//     );
//     println!(
//         "Point can get a vtable for X3: {:?}",
//         can_get_vtable::<Point, CollectX3>()
//     );

//     // let multi_collect()
// }

// trait MultiImplements {
//     unsafe fn vtable_pointers<C: Component>() -> SmallVec<[(TypeId, Option<*const usize>); 1]>;
// }

// // impl for single trait:
// impl<X: Collect> MultiImplements for X {
//     unsafe fn vtable_pointers<C: Component>() -> SmallVec<[(TypeId, Option<*const usize>); 1]> {
//         smallvec![(TypeId::of::<Self>(), Self::vtable_pointer::<C>())]
//     }
// }

// /// This macro generates impls for tuples of MultiImplements.
// ///
// /// e.g.:
// /// ```rust,norun
// /// impl<A: MultiImplements, B: MultiImplements> MultiImplements for (A, B) {
// ///     unsafe fn vtable_pointers<C: Component>() -> SmallVec<[(TypeId, Option<*const usize>); 1]> {
// ///         let mut a = A::vtable_pointers::<C>();
// ///         let b = B::vtable_pointers::<C>();
// ///         a.extend(b);
// ///         a
// ///     }
// /// }
// /// ```
// macro_rules! multi_implements_impl_for_tuples {
//     ($a:ident,$($x:ident),+) => {
//         impl<$a: MultiImplements, $($x : MultiImplements,)+> MultiImplements for ($a, $($x,)+){
//             unsafe fn vtable_pointers<Comp: Component>() -> SmallVec<[(TypeId, Option<*const usize>); 1]> {
//                 let mut a = $a::vtable_pointers::<Comp>();
//                 $(
//                     let o = $x::vtable_pointers::<Comp>();
//                     a.extend(o);
//                 )+
//                 a
//             }
//         }
//     };
// }

// multi_implements_impl_for_tuples!(A, B);
// multi_implements_impl_for_tuples!(A, B, C);
// multi_implements_impl_for_tuples!(A, B, C, D);
// multi_implements_impl_for_tuples!(A, B, C, D, E);
// multi_implements_impl_for_tuples!(A, B, C, D, E, F);
// multi_implements_impl_for_tuples!(A, B, C, D, E, F, G);
// multi_implements_impl_for_tuples!(A, B, C, D, E, F, G, H);
// multi_implements_impl_for_tuples!(A, B, C, D, E, F, G, H, I);
// multi_implements_impl_for_tuples!(A, B, C, D, E, F, G, H, I, J);

pub fn main() {}
