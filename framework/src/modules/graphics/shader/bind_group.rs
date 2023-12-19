// use smallvec::{smallvec, SmallVec};
// pub trait IntoBindGroupLayouts {
//     fn bind_group_layouts() -> SmallVec<[&'static wgpu::BindGroupLayout; 2]>;
// }

// impl IntoBindGroupLayouts for () {
//     fn bind_group_layouts() -> SmallVec<[&'static wgpu::BindGroupLayout; 2]> {
//         smallvec![]
//     }
// }

// impl<T: StaticBindGroup> IntoBindGroupLayouts for T {
//     fn bind_group_layouts() -> SmallVec<[&'static wgpu::BindGroupLayout; 2]> {
//         smallvec![Self::bind_group_layout()]
//     }
// }

use smallvec::{smallvec, SmallVec};
use wgpu::BindGroupEntry;

pub trait StaticBindGroup {
    /// # Panics
    /// Make sure the static bind group is initialized before
    fn bind_group_layout() -> &'static wgpu::BindGroupLayout;

    /// # Panics
    /// Make sure the static bind group is initialized before
    fn bind_group() -> &'static wgpu::BindGroup;
}

pub trait BindGroupT {
    const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static>;
    fn bind_group_entries<'a>(&'a self) -> SmallVec<[BindGroupEntry<'a>; 2]>;
}

pub trait MultiBindGroupT {
    const BIND_GROUP_LAYOUT_DESCRIPTORS: &'static [&'static wgpu::BindGroupLayoutDescriptor<
        'static,
    >];
}

impl<T> MultiBindGroupT for T
where
    T: BindGroupT,
{
    const BIND_GROUP_LAYOUT_DESCRIPTORS: &'static [&'static wgpu::BindGroupLayoutDescriptor<
        'static,
    >] = &[&T::BIND_GROUP_LAYOUT_DESCRIPTOR];
}

macro_rules! multi_bind_group_t {
    ($($a:ident),+) => {
        impl<$($a),+> MultiBindGroupT for ($($a),+)
        where
            $($a : BindGroupT),+
        {
            const BIND_GROUP_LAYOUT_DESCRIPTORS: &'static [&'static wgpu::BindGroupLayoutDescriptor<'static>] = &[$(&$a::BIND_GROUP_LAYOUT_DESCRIPTOR),+];
        }
    };
}

multi_bind_group_t!(A, B);
multi_bind_group_t!(A, B, C);
multi_bind_group_t!(A, B, C, D);
multi_bind_group_t!(A, B, C, D, E);
