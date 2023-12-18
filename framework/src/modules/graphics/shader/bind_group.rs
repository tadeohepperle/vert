use smallvec::{smallvec, SmallVec};
pub trait IntoBindGroupLayouts {
    fn bind_group_layouts() -> SmallVec<[&'static wgpu::BindGroupLayout; 2]>;
}

impl IntoBindGroupLayouts for () {
    fn bind_group_layouts() -> SmallVec<[&'static wgpu::BindGroupLayout; 2]> {
        smallvec![]
    }
}

impl<T: StaticBindGroup> IntoBindGroupLayouts for T {
    fn bind_group_layouts() -> SmallVec<[&'static wgpu::BindGroupLayout; 2]> {
        smallvec![Self::bind_group_layout()]
    }
}

pub trait StaticBindGroup {
    /// # Panics
    /// Make sure the static bind group is initialized before
    fn bind_group_layout() -> &'static wgpu::BindGroupLayout;

    /// # Panics
    /// Make sure the static bind group is initialized before
    fn bind_group() -> &'static wgpu::BindGroup;
}
