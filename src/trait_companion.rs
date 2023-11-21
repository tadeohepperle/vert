// //! This example shows a technique of dynamically getting V-tables for traits.
// //! These Vtables can be retrieved once and stored for later dynamic dispatch.

// use std::{
//     any::{type_name, TypeId},
//     marker::PhantomData,
// };

// use smallvec::{smallvec, SmallVec};

// use crate::component::Component;

// pub type VTablePointer = *const usize;

// #[derive(Debug, Clone, Copy)]
// pub struct VTablePointerWithMetadata {
//     /// The second half of a trait object.
//     pub ptr: VTablePointer,
//     /// type id of the traits companion struct.
//     pub ty_id: TypeId,
//     /// type name of the traits companion struct.
//     pub ty_name: &'static str,
// }
// const PTR_SIZE: usize = std::mem::size_of::<usize>();

// /// is meant to be implemented for dyn Traitname
// pub trait Reflectable<Dyn>: 'static {
//     unsafe fn vtable_pointer<C: Component>() -> Option<VTablePointerWithMetadata>
//     where
//         Self: Sized,
//     {
//         let uninit_dyn: Option<&'static Self> = <C as Implements<Self>>::uninit_trait_obj();
//         match uninit_dyn {
//             Some(trait_object) => {
//                 // this is a fat pointer: (data + vtable)
//                 assert_eq!(std::mem::size_of::<&Dyn>(), PTR_SIZE * 2);
//                 // lets get a pointer to just the vtable:
//                 let data_pointer: VTablePointer = &trait_object as *const _ as *const usize;
//                 let ptr: VTablePointer = data_pointer.add(1);
//                 let ptr_with_metadata = VTablePointerWithMetadata {
//                     ty_id: TypeId::of::<Self>(),
//                     ty_name: type_name::<Self>(),
//                     ptr,
//                 };
//                 Some(ptr_with_metadata)
//             }
//             None => None,
//         }
//     }
// }

// /// the T here should be an unsized dyn TraitXYZ
// pub struct Trait<T: ?Sized>(PhantomData<T>);

// impl<T: ?Sized> Trait<T> {

// }

// trait Implements<T, X: Reflectable<T>> {
//     unsafe fn uninit_trait_obj() -> Option<&'static X>;
// }

// impl<T, X: Reflectable<T>, C: Component> Implements<X> for C {
//     default unsafe fn uninit_trait_obj() -> Option<&'static X> {
//         None
//     }
// }

// pub trait MultiTraitCompanion {
//     unsafe fn vtable_pointers<C: Component>(
//     ) -> SmallVec<[(TypeId, Option<VTablePointerWithMetadata>); 1]>;
// }

// // impl for unit type:

// impl Reflectable for () {
//     unsafe fn vtable_pointer<C: Component>() -> Option<VTablePointerWithMetadata>
//     where
//         Self: Sized,
//     {
//         None
//     }

//     type DynUnsized = ();
// }

// // impl for single trait:
// impl<X: Reflectable> MultiTraitCompanion for X {
//     unsafe fn vtable_pointers<C: Component>(
//     ) -> SmallVec<[(TypeId, Option<VTablePointerWithMetadata>); 1]> {
//         smallvec![(TypeId::of::<Self>(), Self::vtable_pointer::<C>())]
//     }
// }

// /// This macro generates impls for tuples of MultiImplements.
// ///
// /// e.g.:
// /// ```rust,norun
// /// impl<A: MultiTraitCompanion, B: MultiTraitCompanion> MultiTraitCompanion for (A, B) {
// ///     unsafe fn vtable_pointers<C: Component>() -> SmallVec<[(TypeId, Option<VTablePointerWithMetadata>); 1]> {
// ///         let mut a = A::vtable_pointers::<C>();
// ///         let b = B::vtable_pointers::<C>();
// ///         a.extend(b);
// ///         a
// ///     }
// /// }
// /// ```
// macro_rules! multi_implements_impl_for_tuples {
//     ($a:ident,$($x:ident),+) => {
//         impl<$a: MultiTraitCompanion, $($x : MultiTraitCompanion,)+> MultiTraitCompanion for ($a, $($x,)+){
//             unsafe fn vtable_pointers<Comp: Component>() -> SmallVec<[(TypeId, Option<VTablePointerWithMetadata>); 1]> {
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

// #[cfg(test)]
// mod tests {

//     use super::{Implements, Reflectable, Trait};
//     use crate::component::Component;
//     use std::mem::MaybeUninit;

//     // /////////////////////////////////////////////////////////////////////////////
//     // Examples of trait implementation.
//     // Xi are traits, the shapes are structs.
//     //
//     //          X1    X2    X3
//     // Circle   x
//     // Rect     x     x
//     // Point    x     x     x
//     //
//     // /////////////////////////////////////////////////////////////////////////////

//     struct Circle;
//     struct Rect;
//     struct Point;

//     impl Component for Circle {}
//     impl Component for Rect {}
//     impl Component for Point {}

//     trait X1 {}
//     impl Reflectable for Trait<dyn X1> {
//         type DynUnsized = dyn X1;
//     }

//     trait X2 {}
//     impl Reflectable for dyn X2 {
//         type DynUnsized = dyn X2;
//     }

//     trait X3 {}
//     impl Reflectable for dyn X3 {
//         type DynUnsized = dyn X3;
//     }

//     impl X1 for Circle {}
//     impl Implements<dyn X1> for Circle {
//         unsafe fn uninit_trait_obj() -> Option<&'static dyn X1> {
//             const CIRCLE: Circle = unsafe { MaybeUninit::<Circle>::uninit().assume_init() };
//             Some(&CIRCLE as &'static dyn X1)
//         }
//     }

//     impl X1 for Rect {}
//     impl Implements<dyn X1> for Rect {
//         unsafe fn uninit_trait_obj() -> Option<&'static dyn X1> {
//             const RECT: Rect = unsafe { MaybeUninit::<Rect>::uninit().assume_init() };
//             Some(&RECT as &'static dyn X1)
//         }
//     }

//     impl X2 for Rect {}
//     impl Implements<dyn X2> for Rect {
//         unsafe fn uninit_trait_obj() -> Option<&'static dyn X2> {
//             const RECT: Rect = unsafe { MaybeUninit::<Rect>::uninit().assume_init() };
//             Some(&RECT as &'static dyn X2)
//         }
//     }

//     impl X1 for Point {}
//     impl Implements<dyn X1> for Point {
//         unsafe fn uninit_trait_obj() -> Option<&'static dyn X1> {
//             const POINT: Point = unsafe { MaybeUninit::<Point>::uninit().assume_init() };
//             Some(&POINT as &'static dyn X1)
//         }
//     }

//     impl X2 for Point {}
//     impl Implements<dyn X2> for Point {
//         unsafe fn uninit_trait_obj() -> Option<&'static dyn X2> {
//             const POINT: Point = unsafe { MaybeUninit::<Point>::uninit().assume_init() };
//             Some(&POINT as &'static dyn X2)
//         }
//     }

//     impl X3 for Point {}
//     impl Implements<dyn X3> for Point {
//         unsafe fn uninit_trait_obj() -> Option<&'static dyn X3> {
//             const POINT: Point = unsafe { MaybeUninit::<Point>::uninit().assume_init() };
//             Some(&POINT as &'static dyn X3)
//         }
//     }

//     fn can_get_vtable<C: Component, X: Reflectable>() -> bool {
//         unsafe { <C as Implements<X>>::uninit_trait_obj().is_some() }
//     }

//     #[test]
//     fn test() {
//         assert!(can_get_vtable::<Circle, dyn X1>());
//         assert!(!can_get_vtable::<Circle, dyn X2>());
//         assert!(!can_get_vtable::<Circle, dyn X3>());

//         assert!(can_get_vtable::<Rect, dyn X1>());
//         assert!(can_get_vtable::<Rect, dyn X2>());
//         assert!(!can_get_vtable::<Rect, dyn X3>());

//         assert!(can_get_vtable::<Point, dyn X1>());
//         assert!(can_get_vtable::<Point, dyn X2>());
//         assert!(can_get_vtable::<Point, dyn X3>());
//     }
// }
