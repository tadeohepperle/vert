use smallvec::SmallVec;
use wgpu::{naga::TypeInner, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry};

pub trait StaticBindGroup {
    /// # Panics
    /// Make sure the static bind group is initialized before
    fn bind_group_layout() -> &'static wgpu::BindGroupLayout;

    /// # Panics
    /// Make sure the static bind group is initialized before
    fn bind_group() -> &'static wgpu::BindGroup;
}

pub struct BindGroupEntryDef {
    pub name: &'static str,
    pub visibility: wgpu::ShaderStages,
    pub ty: wgpu::BindingType,
    // iff the ty is BindingType::Buffer::Uniform <==> struct_fields should be Some(..)
    pub struct_fields: Option<&'static [(&'static str, TypeInner)]>,
}

pub struct BindGroupDef {
    pub name: &'static str,
    pub entries: &'static [BindGroupEntryDef],
}

pub trait BindGroupT {
    const BIND_GROUP_DEF: BindGroupDef;
    fn bind_group_entries<'a>(&'a self) -> SmallVec<[BindGroupEntry<'a>; 2]>;

    fn create_bind_group_layout(device: &wgpu::Device) -> BindGroupLayout {
        let mut entries: Vec<BindGroupLayoutEntry> = vec![];
        for (i, e) in Self::BIND_GROUP_DEF.entries.iter().enumerate() {
            entries.push(BindGroupLayoutEntry {
                binding: i as u32,
                visibility: e.visibility,
                ty: e.ty,
                count: None,
            })
        }
        let desc = wgpu::BindGroupLayoutDescriptor {
            label: Some(Self::BIND_GROUP_DEF.name),
            entries: &entries,
        };
        device.create_bind_group_layout(&desc)
    }
}

pub trait MultiBindGroupT {
    const BIND_GROUP_DEFS: &'static [&'static BindGroupDef];

    fn create_bind_group_layouts(device: &wgpu::Device) -> Vec<BindGroupLayout>;
}

impl<T> MultiBindGroupT for T
where
    T: BindGroupT,
{
    const BIND_GROUP_DEFS: &'static [&'static BindGroupDef] = &[&T::BIND_GROUP_DEF];

    fn create_bind_group_layouts(device: &wgpu::Device) -> Vec<BindGroupLayout> {
        let mut layouts: Vec<BindGroupLayout> = vec![];
        layouts.push(T::create_bind_group_layout(device));
        layouts
    }
}

macro_rules! multi_bind_group_t {
    ($($a:ident),+) => {
        impl<$($a),+> MultiBindGroupT for ($($a),+)
        where
            $($a : BindGroupT),+
        {
            const BIND_GROUP_DEFS: &'static [&'static BindGroupDef] = &[$(&$a::BIND_GROUP_DEF),+];

            fn create_bind_group_layouts(device: &wgpu::Device) -> Vec<BindGroupLayout> {
                let mut layouts: Vec<BindGroupLayout> = vec![];
                $(layouts.push($a::create_bind_group_layout(device));)+
                layouts
            }

        }
    };
}

multi_bind_group_t!(A, B);
multi_bind_group_t!(A, B, C);
multi_bind_group_t!(A, B, C, D);
multi_bind_group_t!(A, B, C, D, E);
