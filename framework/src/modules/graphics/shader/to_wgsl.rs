use std::{any, borrow::Cow, fmt::Write, path::PathBuf};

use crate::modules::graphics::shader::{bind_group::MultiBindGroupT, vertex::VertexT};
use anyhow::anyhow;
use heck::ToUpperCamelCase;
use indoc::{formatdoc, indoc};
use wgpu::naga::{ScalarKind, TypeInner, VectorSize};

use super::{bind_group::BindGroupDef, vertex::VertexAttribute, ShaderT};

pub fn generate_wgsl_skeleton<S: ShaderT>(path: &str) {
    let vertex = indoc! {"
            // vertex shader code here.
            var out: VertexOutput;
            // ...
            return out;
        "};

    let fragment = indoc! {"
            // fragment shader code here.
        "};

    let other = indoc! {"
            // You can also include other code.
        "};

    let wgsl_string = generate_wgsl::<S>(vertex, fragment, other);
    std::fs::write(path, wgsl_string).expect("Could not write wgsl file");
}

pub fn generate_wgsl<S: ShaderT>(
    vertex_inner_code: &str,
    fragment_inner_code: &str,
    other_code: &str,
) -> String {
    let bind_groups = bind_groups_to_wgsl(<S::BindGroups as MultiBindGroupT>::BIND_GROUP_DEFS);

    let (vertex_output_struct_def, vertex_struct_def, instance_struct_def) =
        vertex_instance_output_struct_defs::<S>();

    let vertex_function_args: String = {
        let mut args: Vec<String> = vec![];

        let builtins = find_usage_of_builtin_inputs(vertex_inner_code, WGPU_VERTEX_BUILTIN_INPUTS);
        for (name, ty) in builtins {
            args.push(format!("@builtin({name}) {name}: {ty}"));
        }
        if vertex_struct_def.is_some() {
            args.push("vertex: Vertex".into());
        }
        if instance_struct_def.is_some() {
            args.push("instance: Instance".into());
        }

        args.join(", ")
    };

    let fragment_function_args: String = {
        let mut args: Vec<String> = vec![];

        let builtins =
            find_usage_of_builtin_inputs(fragment_inner_code, WGPU_FRAGMENT_BUILTIN_INPUTS);
        for (name, ty) in builtins {
            args.push(format!("@builtin({name}) {name}: {ty}"));
        }
        args.push("in: VertexOutput".into());
        args.join(", ")
    };

    let vertex_struct_def = vertex_struct_def.unwrap_or_default();
    let instance_struct_def = instance_struct_def.unwrap_or_default();

    formatdoc! {"
        {bind_groups}

        {vertex_output_struct_def}

        {vertex_struct_def}

        {instance_struct_def}

        @vertex 
        fn vs_main({vertex_function_args}) -> VertexOutput {{
            {vertex_inner_code}
        }}

        @fragment 
        fn fs_main({fragment_function_args}) -> @location(0) vec4<f32>  {{
            {fragment_inner_code}
        }}
    
        {other_code}
    "}
}

/// generates struct_defs for (VertexOutput, Vertex, Instance)
fn vertex_instance_output_struct_defs<S: ShaderT>() -> (String, Option<String>, Option<String>) {
    let vertex_output_struct_def = vertex_attributes_to_wgsl_struct(
        <S::VertexOutput as VertexT>::ATTRIBUTES,
        0,
        "VertexOutput",
    );

    let vertex_attributes = <S::Vertex as VertexT>::ATTRIBUTES;
    let vertex_struct_def: Option<String> = if vertex_attributes.is_empty() {
        None
    } else {
        Some(vertex_attributes_to_wgsl_struct(
            vertex_attributes,
            0,
            "Vertex",
        ))
    };

    let instance_attributes = <S::Instance as VertexT>::ATTRIBUTES;
    let instance_struct_def = if instance_attributes.is_empty() {
        None
    } else {
        Some(vertex_attributes_to_wgsl_struct(
            instance_attributes,
            vertex_attributes.len(),
            "Instance",
        ))
    };

    (
        vertex_output_struct_def,
        vertex_struct_def,
        instance_struct_def,
    )
}

fn vertex_attributes_to_wgsl_struct(
    vertex_attributes: &[VertexAttribute],
    location_offset: usize,
    struct_name: &str,
) -> String {
    let mut s: String = format!("struct {struct_name} {{\n");
    if struct_name == "VertexOutput" {
        _ = writeln!(s, "    @builtin(position) clip_position: vec4<f32>,");
    }

    for (i, attr) in vertex_attributes.iter().enumerate() {
        let i = i + location_offset;
        let attr_name = attr.ident;
        let attr_type = vertex_format_to_wgsl(attr.format);
        _ = writeln!(s, "    @location({i}) {attr_name}: {attr_type},");
    }

    s.push_str("}\n");
    s
}

fn bind_groups_to_wgsl(bind_groups: &[&BindGroupDef]) -> String {
    let mut s: String = String::new();

    for (i, def) in bind_groups.iter().enumerate() {
        for (j, entry) in def.entries.iter().enumerate() {
            if matches!(entry.ty, wgpu::BindingType::Buffer { ty, .. })
                && entry.struct_fields.is_none()
            {
                panic!("For Buffer bindings, you need to define struct_fields in the BindGroupDef!")
            }
            let entry_name = entry.name;
            let entry_name_upper = entry_name.to_upper_camel_case();
            let (prefix, ty) = match entry.ty {
                wgpu::BindingType::Buffer {
                    ty,
                    has_dynamic_offset,
                    min_binding_size,
                } => match ty {
                    wgpu::BufferBindingType::Uniform => ("var<uniform>", entry_name_upper.as_str()),
                    wgpu::BufferBindingType::Storage { read_only } => {
                        panic!(
                            "todo!(Storage Buffers not yet implemented as a binding group definition)"
                        )
                    }
                },
                wgpu::BindingType::Sampler(_) => ("var", "sampler"),
                wgpu::BindingType::Texture {
                    sample_type,
                    view_dimension,
                    multisampled,
                } => {
                    let sample_ty = match sample_type {
                        wgpu::TextureSampleType::Float { filterable } => "texture_2d<f32>",
                        wgpu::TextureSampleType::Depth => "texture_depth_2d",
                        wgpu::TextureSampleType::Sint => "texture_2d<i32>",
                        wgpu::TextureSampleType::Uint => "texture_2d<u32>",
                    };
                    ("var", sample_ty)
                }
                wgpu::BindingType::StorageTexture {
                    access,
                    format,
                    view_dimension,
                } => panic!(
                    "todo!(StorageTexture is not yet implemented as a binding group definition)"
                ),
            };

            _ = writeln!(s, "@group({i}) @binding({j})");
            _ = writeln!(s, "{prefix} {entry_name} : {ty};\n");

            // eg. something like this
            //
            // @group(0) @binding(0)
            // var<uniform> camera: Camera;
            // @group(1) @binding(0)
            // var t_diffuse: texture_2d<f32>;
            // @group(1) @binding(1)
            // var s_diffuse: sampler;

            // add struct definition for uniforms:

            if let Some(struct_fields) = entry.struct_fields {
                // eg. something like:

                // struct Camera {
                //     view_pos: vec4<f32>,
                //     view_proj: mat4x4<f32>,
                // }

                _ = writeln!(s, "struct {ty} {{");
                for (field_name, field_ty) in struct_fields {
                    let field_ty = type_inner_to_wgsl(field_ty);
                    _ = writeln!(s, "    {field_name} : {field_ty},");
                }

                _ = writeln!(s, "}}\n");
            }
        }
    }

    s
}

pub fn find_usage_of_builtin_inputs(
    code: &str,
    builtins: &'static [(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    let mut found: Vec<(&'static str, &'static str)> = vec![];
    for (name, ty) in builtins {
        // super naive, make better later...
        if code.contains(name) {
            found.push((*name, *ty));
        }
    }
    found
}

/// See https://www.w3.org/TR/WGSL/#built-in-values
const WGPU_VERTEX_BUILTIN_INPUTS: &[(&str, &str)] =
    &[("vertex_index", "u32"), ("instance_index", "u32")];

/// See https://www.w3.org/TR/WGSL/#built-in-values
const WGPU_FRAGMENT_BUILTIN_INPUTS: &[(&str, &str)] = &[
    ("position", "vec4<f32>"),
    ("front_facing", "bool"),
    ("frag_depth", "f32"),
    ("sample_index", "u32"),
    ("sample_mask", "u32"),
];

pub fn type_inner_to_wgsl(type_inner: &TypeInner) -> String {
    match type_inner {
        TypeInner::Scalar { kind, width } => scalar_to_wgsl(kind).into(),
        TypeInner::Vector { size, kind, width } => {
            format!("vec{}<{}>", *size as u32, scalar_to_wgsl(kind))
        }
        TypeInner::Matrix {
            columns,
            rows,
            width,
        } => format!("mat{}x{}<f32>", *columns as u32, *rows as u32,),
        _ => panic!("todo!() type {type_inner:?} not supported as field of a uniform, yet."),
    }
}

pub fn scalar_to_wgsl(scalar: &ScalarKind) -> &'static str {
    match scalar {
        ScalarKind::Sint => "i32",
        ScalarKind::Uint => "u32",
        ScalarKind::Float => "f32",
        ScalarKind::Bool => "bool", // ???
    }
}

pub const fn vertex_format_to_wgsl(vertex_format: wgpu::VertexFormat) -> &'static str {
    const u32: &str = "u32";
    const vec2u32: &str = "vec2<u32>";
    const vec3u32: &str = "vec3<u32>";
    const vec4u32: &str = "vec4<u32>";

    const i32: &str = "i32";
    const vec2i32: &str = "vec2<i32>";
    const vec3i32: &str = "vec3<i32>";
    const vec4i32: &str = "vec4<i32>";

    const f32: &str = "f32";
    const vec2f32: &str = "vec2<f32>";
    const vec3f32: &str = "vec3<f32>";
    const vec4f32: &str = "vec4<f32>";

    match vertex_format {
        wgpu::VertexFormat::Uint8x2 => vec2u32,
        wgpu::VertexFormat::Uint8x4 => vec4u32,
        wgpu::VertexFormat::Sint8x2 => vec2i32,
        wgpu::VertexFormat::Sint8x4 => vec4i32,
        wgpu::VertexFormat::Unorm8x2 => vec2f32,
        wgpu::VertexFormat::Unorm8x4 => vec4f32,
        wgpu::VertexFormat::Snorm8x2 => vec2f32,
        wgpu::VertexFormat::Snorm8x4 => vec4f32,
        wgpu::VertexFormat::Uint16x2 => vec2u32,
        wgpu::VertexFormat::Uint16x4 => vec4u32,
        wgpu::VertexFormat::Sint16x2 => vec2i32,
        wgpu::VertexFormat::Sint16x4 => vec4i32,
        wgpu::VertexFormat::Unorm16x2 => vec2f32,
        wgpu::VertexFormat::Unorm16x4 => vec4f32,
        wgpu::VertexFormat::Snorm16x2 => vec2f32,
        wgpu::VertexFormat::Snorm16x4 => vec4f32,
        wgpu::VertexFormat::Float16x2 => vec2f32,
        wgpu::VertexFormat::Float16x4 => vec4f32,
        wgpu::VertexFormat::Float32 => f32,
        wgpu::VertexFormat::Float32x2 => vec2f32,
        wgpu::VertexFormat::Float32x3 => vec3f32,
        wgpu::VertexFormat::Float32x4 => vec4f32,
        wgpu::VertexFormat::Uint32 => u32,
        wgpu::VertexFormat::Uint32x2 => vec2u32,
        wgpu::VertexFormat::Uint32x3 => vec3u32,
        wgpu::VertexFormat::Uint32x4 => vec4u32,
        wgpu::VertexFormat::Sint32 => i32,
        wgpu::VertexFormat::Sint32x2 => vec2i32,
        wgpu::VertexFormat::Sint32x3 => vec3i32,
        wgpu::VertexFormat::Sint32x4 => vec4i32,
        wgpu::VertexFormat::Float64 => f32,
        wgpu::VertexFormat::Float64x2 => vec2f32,
        wgpu::VertexFormat::Float64x3 => vec3f32,
        wgpu::VertexFormat::Float64x4 => vec4f32,
    }
}
