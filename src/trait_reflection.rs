// // /////////////////////////////////////////////////////////////////////////////
// // Super Simple Trait Reflection
// // /////////////////////////////////////////////////////////////////////////////

use std::any::TypeId;

use smallvec::{smallvec, SmallVec};

/// should only be implemented for `dyn MyTrait`
pub trait DynTrait: 'static {
    fn id() -> TypeId {
        TypeId::of::<Self>()
    }
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub trait Implementor: Sized + 'static {
    unsafe fn dyn_traits() -> &'static [VTablePtrWithMeta];
}

// /////////////////////////////////////////////////////////////////////////////
// Multi Traits
// /////////////////////////////////////////////////////////////////////////////

type VTablePtr = *const ();

#[repr(C)]
pub struct VTable {
    data: *const (),
    ptr: VTablePtr,
}

#[derive(Debug, Clone, Copy)]
pub struct VTablePtrWithMeta {
    /// The second half of a trait object.
    pub ptr: VTablePtr,
    /// type id of dyn MyTrait.
    pub dyn_trait_type_id: TypeId,
    /// type name of the dyn trait: e.g. "dyn vert::trait_reflection::types::Render"
    pub dyn_trait_type_name: &'static str,
}

pub fn vtable_pointer<C: Implementor, T: DynTrait + ?Sized>() -> Option<VTablePtrWithMeta> {
    let dyn_trait_type_id = T::id();
    unsafe {
        C::dyn_traits()
            .iter()
            .find(|e| e.dyn_trait_type_id == dyn_trait_type_id)
            .cloned()
    }
}

pub trait MultipleReflectedTraits {
    unsafe fn vtable_pointers<C: Implementor>() -> SmallVec<[(TypeId, Option<VTablePtrWithMeta>); 1]>;
}

impl DynTrait for () {}

impl<T: DynTrait> MultipleReflectedTraits for T {
    unsafe fn vtable_pointers<C: Implementor>() -> SmallVec<[(TypeId, Option<VTablePtrWithMeta>); 1]>
    {
        let dyn_trait_type_id = Self::id();
        let dyn_trait_type_name = Self::name();
        let c_vtable_ptr_with_meta = <C as Implementor>::dyn_traits().iter().find_map(|v| {
            if dyn_trait_type_id == v.dyn_trait_type_id {
                assert_eq!(dyn_trait_type_name, v.dyn_trait_type_name);
                Some(*v)
            } else {
                None
            }
        });
        smallvec![(dyn_trait_type_id, c_vtable_ptr_with_meta)]
    }
}

macro_rules! multi_implements_impl_for_tuples {
    ($a:ident,$($x:ident),+) => {
        impl<$a: MultipleReflectedTraits, $($x : MultipleReflectedTraits,)+> MultipleReflectedTraits for ($a, $($x,)+){
            unsafe fn vtable_pointers<Comp: Implementor>() -> SmallVec<[(TypeId, Option<VTablePtrWithMeta>); 1]> {
                let mut a = $a::vtable_pointers::<Comp>();
                $(
                    let o = $x::vtable_pointers::<Comp>();
                    a.extend(o);
                )+
                a
            }
        }
    };
}

multi_implements_impl_for_tuples!(A, B);
multi_implements_impl_for_tuples!(A, B, C);
multi_implements_impl_for_tuples!(A, B, C, D);
multi_implements_impl_for_tuples!(A, B, C, D, E);
multi_implements_impl_for_tuples!(A, B, C, D, E, F);
multi_implements_impl_for_tuples!(A, B, C, D, E, F, G);
multi_implements_impl_for_tuples!(A, B, C, D, E, F, G, H);
multi_implements_impl_for_tuples!(A, B, C, D, E, F, G, H, I);
multi_implements_impl_for_tuples!(A, B, C, D, E, F, G, H, I, J);

// /////////////////////////////////////////////////////////////////////////////
// Macros!
// /////////////////////////////////////////////////////////////////////////////

/// This macro can be applied to traits or specifying a struct and some traits it implements:
/// ### Use on traits:
/// ```rust,norun
/// trait Render { }
/// reflect!(Render)
/// ```
/// which expands to:
/// ```rust,norun
/// trait Render { }
/// impl DynTrait for dyn Render {}
/// ```
///
/// ### Use on structs, specifying traits:
/// ```rust,norun
/// trait Render { }
/// struct Circle;
/// reflect!(Circle: Render)
/// ```
/// which expands to:
/// ```rust,norun
/// impl Implementor for Circle {
///     unsafe fn dyn_traits() -> &'static [VTablePtrWithMeta] {
///         const UNINIT: Circle =
///             unsafe { std::mem::MaybeUninit::<Circle>::uninit().assume_init() };
///         const IMPLS: &'static [VTablePtrWithMeta] = &[{
///             const RENDER: &'static dyn Render = &UNINIT as &'static dyn Render;
///             if std::mem::size_of::<&dyn Render>() != std::mem::size_of::<usize>() * 2 {
///                 panic!("Error in Implementor::dyn_traits, invalid fat pointer")
///             }
///             let vtable = &RENDER as *const _ as *const VTable;
///             VTablePtrWithMeta {
///                 ptr: unsafe { (*vtable).ptr },
///                 dyn_trait_type_id: TypeId::of::<dyn Render>(),
///                 dyn_trait_type_name: std::any::type_name::<dyn Render>(),
///             }
///         }];
///         IMPLS
///     }
/// }
/// ```
#[macro_export]
macro_rules! reflect {
    ($trait:ident) => {
        impl DynTrait for dyn $trait {}
    };
    ($component:ident : $($trait:ident),+ ) => {
        impl Implementor for $component {
            unsafe fn dyn_traits() -> &'static [VTablePtrWithMeta] {
                const UNINIT: $component =
                    unsafe { std::mem::MaybeUninit::<$component>::uninit().assume_init() };
                const IMPLS: &'static [VTablePtrWithMeta] = &[
                    $(
                        {
                            const TRAIT_OBJ: &'static dyn $trait = &UNINIT as &'static dyn $trait;
                            if std::mem::size_of::<&dyn $trait>() != std::mem::size_of::<usize>() * 2 {
                                panic!("Error in Implementor::dyn_traits, invalid fat pointer...")
                            }
                            let vtable = &TRAIT_OBJ as *const _ as *const VTable;
                            VTablePtrWithMeta {
                                ptr: unsafe { (*vtable).ptr },
                                dyn_trait_type_id: std::any::TypeId::of::<dyn $trait>(),
                                dyn_trait_type_name: std::any::type_name::<dyn $trait>(),
                            }
                        }
                    ),+
                    ];
                IMPLS
            }
        }
    };
}

// // /////////////////////////////////////////////////////////////////////////////
// // Some example structs/traits
// // /////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::{prelude::*, trait_reflection::vtable_pointer};

    #[test]
    fn test_macros() {
        struct Circle;
        struct Rect;
        struct Point;

        pub trait Render {}
        reflect!(Render);

        pub trait Log {}
        reflect!(Log);

        pub trait Update {}
        reflect!(Update);

        impl Render for Circle {}

        impl Render for Rect {}
        impl Log for Rect {}

        impl Render for Point {}
        impl Log for Point {}
        impl Update for Point {}

        reflect!(Circle: Render);
        reflect!(Rect: Render, Log);
        reflect!(Point: Render, Log, Update);

        // /////////////////////////////////////////////////////////////////////////////
        // Examples of trait implementation.
        // Xi are traits, the shapes are structs.
        //
        //          Render   Log    Update
        // Circle   x
        // Rect     x        x
        // Point    x        x      x
        //
        // /////////////////////////////////////////////////////////////////////////////

        assert!(vtable_pointer::<Circle, dyn Render>().is_some());
        assert!(vtable_pointer::<Circle, dyn Log>().is_none());
        assert!(vtable_pointer::<Circle, dyn Update>().is_none());

        assert!(vtable_pointer::<Rect, dyn Render>().is_some());
        assert!(vtable_pointer::<Rect, dyn Log>().is_some());
        assert!(vtable_pointer::<Rect, dyn Update>().is_none());

        assert!(vtable_pointer::<Point, dyn Render>().is_some());
        assert!(vtable_pointer::<Point, dyn Log>().is_some());
        assert!(vtable_pointer::<Point, dyn Update>().is_some());
    }
}
