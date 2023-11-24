// // /////////////////////////////////////////////////////////////////////////////
// // Super Simple Trait Reflection
// // /////////////////////////////////////////////////////////////////////////////

use std::any::TypeId;

use smallvec::{smallvec, SmallVec};

/// implemented for struct DynMyTrait
pub trait ReflectedTrait: Sized {
    type Dyn: ?Sized + 'static;

    fn dyn_trait_type_id() -> TypeId {
        TypeId::of::<Self::Dyn>()
    }

    fn dyn_trait_type_name() -> &'static str {
        std::any::type_name::<Self::Dyn>()
    }
}

/// implemented for trait object dyn MyTrait
trait ReflectedTraitInv {
    type Struct: Sized;
}

trait Implements<T: ReflectedTrait> {
    unsafe fn uninit_trait_obj() -> Option<&'static <T as ReflectedTrait>::Dyn>;
}

impl<T: ReflectedTrait, C> Implements<T> for C {
    default unsafe fn uninit_trait_obj() -> Option<&'static <T as ReflectedTrait>::Dyn> {
        None
    }
}

type VTablePtr = *const ();

#[repr(C)]
pub struct VTable {
    data: *const (),
    ptr: VTablePtr,
}

pub fn vtable_pointer<C, T: ReflectedTrait>() -> Option<VTablePtr> {
    let trait_obj = unsafe { <C as Implements<T>>::uninit_trait_obj()? };
    assert_eq!(
        std::mem::size_of::<&<T as ReflectedTrait>::Dyn>(),
        std::mem::size_of::<VTable>()
    );
    let ptr_to_vtable: *const VTable = &trait_obj as *const _ as *const VTable;
    let vtable_ptr: VTablePtr = unsafe { (*ptr_to_vtable).ptr };
    Some(vtable_ptr)
}

// /////////////////////////////////////////////////////////////////////////////
// Multi Traits
// /////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy)]
pub struct VTablePtrWithMeta {
    /// The second half of a trait object.
    pub ptr: VTablePtr,
    /// type id of dyn MyTrait.
    pub dyn_trait_type_id: TypeId,
    /// type name of the dyn trait: e.g. "dyn vert::trait_reflection::types::Render"
    pub dyn_trait_type_name: &'static str,
}

pub trait MultipleReflectedTraits {
    unsafe fn vtable_pointers<C>() -> SmallVec<[(TypeId, Option<VTablePtrWithMeta>); 1]>;
}

impl ReflectedTrait for () {
    type Dyn = ();
}

impl<T: ReflectedTrait> MultipleReflectedTraits for T {
    unsafe fn vtable_pointers<C>() -> SmallVec<[(TypeId, Option<VTablePtrWithMeta>); 1]> {
        let dyn_trait_type_id = Self::dyn_trait_type_id();
        let ptr_with_meta = vtable_pointer::<C, T>().map(|ptr| {
            let dyn_trait_type_name = Self::dyn_trait_type_name();
            VTablePtrWithMeta {
                ptr,
                dyn_trait_type_id,
                dyn_trait_type_name,
            }
        });
        smallvec![(dyn_trait_type_id, ptr_with_meta)]
    }
}

macro_rules! multi_implements_impl_for_tuples {
    ($a:ident,$($x:ident),+) => {
        impl<$a: MultipleReflectedTraits, $($x : MultipleReflectedTraits,)+> MultipleReflectedTraits for ($a, $($x,)+){
            unsafe fn vtable_pointers<Comp>() -> SmallVec<[(TypeId, Option<VTablePtrWithMeta>); 1]> {
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

// // /////////////////////////////////////////////////////////////////////////////
// // Some example structs/traits
// // /////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {

    use vert_macros::reflect;

    use crate::{
        component::Component,
        trait_reflection::{vtable_pointer, Implements, ReflectedTrait, ReflectedTraitInv},
    };

    #[test]
    fn main() {
        struct Circle;
        struct Rect;
        struct Point;

        impl Component for Circle {}
        impl Component for Rect {}
        impl Component for Point {}

        trait Render {}
        struct DynRender;
        impl ReflectedTrait for DynRender {
            type Dyn = dyn Render;
        }
        impl ReflectedTraitInv for dyn Render {
            type Struct = DynRender;
        }

        trait Log {}
        struct DynLog;
        impl ReflectedTrait for DynLog {
            type Dyn = dyn Log;
        }
        impl ReflectedTraitInv for dyn Log {
            type Struct = DynLog;
        }

        trait Update {}
        struct DynUpdate;
        impl ReflectedTrait for DynUpdate {
            type Dyn = dyn Update;
        }
        impl ReflectedTraitInv for dyn Update {
            type Struct = DynUpdate;
        }

        impl Render for Circle {}
        impl Implements<DynRender> for Circle {
            unsafe fn uninit_trait_obj() -> Option<&'static dyn Render> {
                const UNINIT: Circle =
                    unsafe { std::mem::MaybeUninit::<Circle>::uninit().assume_init() };
                Some(&UNINIT as &'static dyn Render)
            }
        }

        impl Render for Rect {}
        impl Implements<DynRender> for Rect {
            unsafe fn uninit_trait_obj() -> Option<&'static dyn Render> {
                const UNINIT: Rect =
                    unsafe { std::mem::MaybeUninit::<Rect>::uninit().assume_init() };
                Some(&UNINIT as &'static dyn Render)
            }
        }

        impl Log for Rect {}
        impl Implements<DynLog> for Rect {
            unsafe fn uninit_trait_obj() -> Option<&'static dyn Log> {
                const UNINIT: Rect =
                    unsafe { std::mem::MaybeUninit::<Rect>::uninit().assume_init() };
                Some(&UNINIT as &'static dyn Log)
            }
        }

        impl Render for Point {}
        impl Implements<DynRender> for Point {
            unsafe fn uninit_trait_obj() -> Option<&'static dyn Render> {
                const UNINIT: Point =
                    unsafe { std::mem::MaybeUninit::<Point>::uninit().assume_init() };
                Some(&UNINIT as &'static dyn Render)
            }
        }

        impl Log for Point {}
        impl Implements<DynLog> for Point {
            unsafe fn uninit_trait_obj() -> Option<&'static dyn Log> {
                const UNINIT: Point =
                    unsafe { std::mem::MaybeUninit::<Point>::uninit().assume_init() };
                Some(&UNINIT as &'static dyn Log)
            }
        }

        impl Update for Point {}
        impl Implements<DynUpdate> for Point {
            unsafe fn uninit_trait_obj() -> Option<&'static dyn Update> {
                const UNINIT: Point =
                    unsafe { std::mem::MaybeUninit::<Point>::uninit().assume_init() };
                Some(&UNINIT as &'static dyn Update)
            }
        }

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

        assert!(vtable_pointer::<Circle, DynRender>().is_some());
        assert!(vtable_pointer::<Circle, DynLog>().is_none());
        assert!(vtable_pointer::<Circle, DynUpdate>().is_none());

        assert!(vtable_pointer::<Rect, DynRender>().is_some());
        assert!(vtable_pointer::<Rect, DynLog>().is_some());
        assert!(vtable_pointer::<Rect, DynUpdate>().is_none());

        assert!(vtable_pointer::<Point, DynRender>().is_some());
        assert!(vtable_pointer::<Point, DynLog>().is_some());
        assert!(vtable_pointer::<Point, DynUpdate>().is_some());
    }

    #[test]
    fn test_macros() {
        struct Circle;
        struct Rect;
        struct Point;

        impl Component for Circle {}
        impl Component for Rect {}
        impl Component for Point {}

        trait Render {}
        reflect!(Render);

        trait Log {}
        reflect!(Log);

        trait Update {}
        reflect!(Update);

        impl Render for Circle {}
        reflect!(Render, Circle);

        impl Render for Rect {}
        reflect!(Render, Rect);

        impl Log for Rect {}
        reflect!(Log, Rect);

        impl Render for Point {}
        reflect!(Render, Point);

        impl Log for Point {}
        reflect!(Log, Point);

        impl Update for Point {}
        reflect!(Update, Point);

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

        assert!(vtable_pointer::<Circle, DynRender>().is_some());
        assert!(vtable_pointer::<Circle, DynLog>().is_none());
        assert!(vtable_pointer::<Circle, DynUpdate>().is_none());

        assert!(vtable_pointer::<Rect, DynRender>().is_some());
        assert!(vtable_pointer::<Rect, DynLog>().is_some());
        assert!(vtable_pointer::<Rect, DynUpdate>().is_none());

        assert!(vtable_pointer::<Point, DynRender>().is_some());
        assert!(vtable_pointer::<Point, DynLog>().is_some());
        assert!(vtable_pointer::<Point, DynUpdate>().is_some());
    }
}
