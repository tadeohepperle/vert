use wgpu::Device;

pub mod camera;
pub mod screen_size;
pub mod static_texture;
pub mod time;

pub trait StaticBindGroup {
    /// # Panics
    /// Make sure the static bind group is initialized before
    fn bind_group_layout() -> &'static wgpu::BindGroupLayout;

    /// # Panics
    /// Make sure the static bind group is initialized before
    fn bind_group() -> &'static wgpu::BindGroup;
}

pub trait ToBindGroup {
    /// # Panics
    /// Make sure the static bind group is initialized before
    fn layout_descriptor() -> wgpu::BindGroupLayoutDescriptor<'static>;

    /// # Panics
    /// Make sure the static bind group is initialized before
    fn to_bind_group(&self, device: &Device, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup;
}
