A nice idea that I did not use in the end:

// // /////////////////////////////////////////////////////////////////////////////
// // Super Simple Trait Reflection
// // /////////////////////////////////////////////////////////////////////////////

use smallvec::{smallvec, SmallVec};
use std::any::TypeId;

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

impl Implementor for () {
    unsafe fn dyn_traits() -> &'static [VTablePtrWithMeta] {
        &[]
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Multi Traits
// /////////////////////////////////////////////////////////////////////////////

pub type VTablePtr = *const ();

#[repr(C)]
pub struct VTable {
    pub data: *const (),
    pub ptr: VTablePtr,
}

#[derive(Debug, Clone, Copy)]
pub struct VTablePtrWithMeta {
    /// The second half of a trait object.
    pub ptr: VTablePtr,
    /// type id of dyn MyTrait.
    pub dyn_trait_id: TypeId,
    /// type name of the dyn trait: e.g. "dyn vert::trait_reflection::types::Render"
    pub dyn_trait_name: &'static str,
}
unsafe impl Sync for VTablePtrWithMeta {}
unsafe impl Send for VTablePtrWithMeta {}

pub fn vtable_pointer<C: Implementor, T: DynTrait + ?Sized>() -> Option<VTablePtrWithMeta> {
    let dyn_trait_id = T::id();
    unsafe {
        C::dyn_traits()
            .iter()
            .find(|e| e.dyn_trait_id == dyn_trait_id)
            .cloned()
    }
}

pub trait MultipleReflectedTraits {
    unsafe fn vtable_pointers<C: Implementor>() -> SmallVec<[(TypeId, Option<VTablePtrWithMeta>); 4]>;
}

impl DynTrait for () {}

impl<T: DynTrait> MultipleReflectedTraits for T {
    unsafe fn vtable_pointers<C: Implementor>() -> SmallVec<[(TypeId, Option<VTablePtrWithMeta>); 4]>
    {
        let dyn_trait_id = Self::id();
        let dyn_trait_name = Self::name();
        let c_vtable_ptr_with_meta = <C as Implementor>::dyn_traits().iter().find_map(|v| {
            if dyn_trait_id == v.dyn_trait_id {
                assert_eq!(dyn_trait_name, v.dyn_trait_name);
                Some(*v)
            } else {
                None
            }
        });
        smallvec![(dyn_trait_id, c_vtable_ptr_with_meta)]
    }
}

macro_rules! multi_implements_impl_for_tuples {
    ($a:ident,$($x:ident),+) => {
        impl<$a: MultipleReflectedTraits, $($x : MultipleReflectedTraits,)+> MultipleReflectedTraits for ($a, $($x,)+){
            unsafe fn vtable_pointers<Comp: Implementor>() -> SmallVec<[(TypeId, Option<VTablePtrWithMeta>); 4]> {
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
/// ```rust,no_run,ignore
/// use vert_core::prelude::*;
/// trait Render { }
/// reflect!(Render)
/// ```
/// which expands to:
/// ```rust,no_run,ignore
/// trait Render { }
/// impl DynTrait for dyn Render {}
/// ```
///
/// ### Use on structs, specifying traits:
/// ```rust,no_run,ignore
/// use vert_core::prelude::*;
///
/// trait Render { }
/// struct Circle;
/// reflect!(Circle: Render)
/// ```
/// which expands to:
/// ```rust,no_run,ignore
///
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
///                 dyn_trait_id: TypeId::of::<dyn Render>(),
///                 dyn_trait_name: std::any::type_name::<dyn Render>(),
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
    ($component:ident : $($trait:ident),* ) => {
        impl Implementor for $component {
            unsafe fn dyn_traits() -> &'static [VTablePtrWithMeta]{
                use std::sync::OnceLock;
                static ONCE: OnceLock<Box<[VTablePtrWithMeta]>> = OnceLock::new();
                ONCE.get_or_init(||{
                    #[allow(invalid_value)]
                    let uninit: $component =
                        unsafe { std::mem::MaybeUninit::<$component>::uninit().assume_init() };
                    let impls = vec![
                        $(
                            {
                                let trait_obj: &dyn $trait = &uninit as &dyn $trait;
                                // if std::mem::size_of::<&dyn $trait>() != std::mem::size_of::<usize>() * 2 {
                                //     panic!("Error in Implementor::dyn_traits, invalid fat pointer...")
                                // }
                                let vtable = &trait_obj as *const _ as *const VTable;
                                VTablePtrWithMeta {
                                    ptr: unsafe { (*vtable).ptr },
                                    dyn_trait_id: std::any::TypeId::of::<dyn $trait>(),
                                    dyn_trait_name: std::any::type_name::<dyn $trait>(),
                                }
                            }
                        ),*
                    ];
                    std::mem::forget(uninit);
                    impls.into()
                })
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
