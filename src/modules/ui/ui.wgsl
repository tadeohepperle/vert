// Inspired by: https://www.shadertoy.com/view/fsdyzB
struct ScreenSize {
    width: f32,
    height: f32,
    aspect: f32,
}

@group(0) @binding(0)
var<uniform> screen: ScreenSize;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct RectInstance {
    @location(0) aabb: vec4<f32>, // pos aabb for the glyph
    @location(1) color: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) border_color: vec4<f32>,
    // border_thickness, border_softness, _unused, _unused
    @location(4) others: vec4<f32>,
}

struct TexturedRectInstance {
    @location(0) aabb: vec4<f32>, // pos aabb for the glyph
    @location(1) color: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) border_color: vec4<f32>,
    // border_thickness, border_softness, _unused, _unused
    @location(4) others: vec4<f32>,
    // for the texture
    @location(5) uv: vec4<f32>,
}

struct GlyphInstance {
    @location(0) pos: vec4<f32>, // pos aabb for the glyph
    @location(1) color: vec4<f32>,
    @location(2) uv: vec4<f32>,    // uv aabb in the texture atlas
}

// we calculate the vertices here in the shader instead of passing a vertex buffer
struct PosVertex {
    pos: vec2<f32>,
}

struct PosUvVertex {
    pos: vec2<f32>,
    uv: vec2<f32>
}

struct RectVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) offset: vec2<f32>, // offset from center
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_radius: vec4<f32>,
    @location(4) border_color: vec4<f32>,
     // border_thickness, border_softness, _unused, _unused
    @location(5) others: vec4<f32>,
};


struct TexturedRectVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) offset: vec2<f32>, // offset from center
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_radius: vec4<f32>,
    @location(4) border_color: vec4<f32>,
     // border_thickness, border_softness, _unused, _unused
    @location(5) others: vec4<f32>,
    @location(6) uv: vec2<f32>,
};


struct GlyphVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) offset: vec2<f32>, // offset from center
    @location(3) size: vec2<f32>,
};

@vertex
fn rect_vs(
    @builtin(vertex_index) vertex_index: u32,
    instance: RectInstance,
) -> RectVertexOutput {
    let vertex = pos_vertex(vertex_index, instance.aabb);
    let device_pos = vec2<f32>((vertex.pos.x / screen.width) * 2.0 - 1.0, 1.0 - (vertex.pos.y / screen.height) * 2.0) ;
    let center = (instance.aabb.xy + instance.aabb.zw) * 0.5;

    var out: RectVertexOutput;
    out.clip_position = vec4<f32>(device_pos, 0.0, 1.0);
    out.offset = vertex.pos - center;
    out.size = instance.aabb.zw - instance.aabb.xy;

    out.color = instance.color;
    out.border_radius = instance.border_radius;
    out.border_color = instance.border_color;
    out.others = instance.others;
    return out;
}
 
@fragment
fn rect_fs(in: RectVertexOutput) -> @location(0) vec4<f32> {
    let sdf = rounded_box_sdf(in.offset, in.size, in.border_radius);
    let color: vec4<f32> = mix(in.color, in.border_color, smoothstep(0.0, 1.0, ((sdf + in.others[0]) / in.others[1]) ));

    let alpha = min(color.a, smoothstep(1.0, 0.0, sdf + 0.5)); // the + 0.5 makes the edge a bit smoother
    // return vec4(in.color.rgb, alpha);
    return vec4(color.rgb, alpha);
}


@vertex
fn textured_rect_vs(
    @builtin(vertex_index) vertex_index: u32,
    instance: TexturedRectInstance,
) -> TexturedRectVertexOutput {
    let vertex = pos_uv_vertex(vertex_index, instance.aabb, instance.uv);
    let device_pos = vec2<f32>((vertex.pos.x / screen.width) * 2.0 - 1.0, 1.0 - (vertex.pos.y / screen.height) * 2.0) ;
    let center = (instance.aabb.xy + instance.aabb.zw) * 0.5;

    var out: TexturedRectVertexOutput;
    out.clip_position = vec4<f32>(device_pos, 0.0, 1.0);
    out.offset = vertex.pos - center;
    out.size = instance.aabb.zw - instance.aabb.xy;

    out.color = instance.color;
    out.border_radius = instance.border_radius;
    out.border_color = instance.border_color;
    out.others = instance.others;
    out.uv = vertex.uv;
    return out;
}

@fragment
fn textured_rect_fs(in: TexturedRectVertexOutput) -> @location(0) vec4<f32> {
    
    // return vec4(0.5, 0.8,0.8,1.0);
    
    let sdf = rounded_box_sdf(in.offset, in.size, in.border_radius);
    let image_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.uv);
    let image_color_tinted: vec3<f32> = mix(image_color.rgb, image_color.rgb * in.color.rgb, in.color.a);
    let image_color_final = vec4(image_color_tinted, image_color.a);

    let color: vec4<f32> = mix(image_color_final, in.border_color, smoothstep(0.0, 1.0, ((sdf + in.others[0]) / in.others[1]) ));
    // todo! add borders and other fancy stuff from above in rect_fs
    return color;
}

@vertex
fn glyph_vs(
    @builtin(vertex_index) vertex_index: u32,
    instance: GlyphInstance,
) -> GlyphVertexOutput {
    let vertex = pos_uv_vertex(vertex_index, instance.pos, instance.uv);
    let device_pos = vec2<f32>((vertex.pos.x / screen.width) * 2.0 - 1.0, 1.0 - (vertex.pos.y / screen.height) * 2.0) ;
    let center = instance.pos.xy + instance.pos.zw * 0.5;

    var out: GlyphVertexOutput;
    out.clip_position = vec4<f32>(device_pos, 0.0, 1.0);
    out.color = instance.color;
    out.uv = vertex.uv; 
    out.offset = vertex.pos - center;
    out.size = instance.pos.zw;
    return out;
}

@fragment
fn glyph_fs(in: GlyphVertexOutput) -> @location(0) vec4<f32> {
    let image_color = textureSample(t_diffuse, s_diffuse, in.uv);
    let color = mix(image_color.rgb, image_color.rgb * in.color.rgb, in.color.a);
    return vec4(color, image_color.a);
}

// given some bounding box aabb [f32;4] being min x, min y, max x, max y,
// extracts the x,y position [f32;2] for the given index in a counter clockwise quad:
// 0 ------ 1
// | .      |
// |   .    |
// |     .  |
// 3 ------ 2  
fn pos_vertex(idx: u32, aabb: vec4<f32>) -> PosVertex {
    var out: PosVertex;
    switch idx {
      case 0u, 4u: {
            out.pos = vec2<f32>(aabb.x, aabb.y); // min x, min y 
        }
      case 1u: {
            out.pos = vec2<f32>(aabb.x, aabb.w); // min x, max y 
        }
      case 2u, 5u: {
            out.pos = vec2<f32>(aabb.z, aabb.w); // max x, max y
        }
      case 3u, default: {
            out.pos = vec2<f32>(aabb.z, aabb.y); // max x, min y 
        }
    }
    return out;
}

// given some bounding box aabb [f32;4] being min x, min y, max x, max y,
// extracts the x,y position [f32;2] for the given index in a counter clockwise quad:
// 0 ------ 1
// | .      |
// |   .    |
// |     .  |
// 3 ------ 2  
fn pos_uv_vertex(idx: u32, pos: vec4<f32>, uv: vec4<f32>) -> PosUvVertex {
    var out: PosUvVertex;
    switch idx {
      case 0u, 4u: {
            out.pos = vec2<f32>(pos.x, pos.y); // min x, min y 
            out.uv = vec2<f32>(uv.x, uv.y);
        }
      case 1u: {
            out.pos = vec2<f32>(pos.x, pos.w); // min x, max y 
            out.uv = vec2<f32>(uv.x, uv.w);
        }
      case 2u, 5u: {
            out.pos = vec2<f32>(pos.z, pos.w); // max x, max y
            out.uv = vec2<f32>(uv.z, uv.w);
        }
      case 3u, default: {
            out.pos = vec2<f32>(pos.z, pos.y); // max x, min y 
            out.uv = vec2<f32>(uv.z, uv.y);
        }
    }
    return out;
}


fn rounded_box_sdf(offset: vec2<f32>, size: vec2<f32>, border_radius: vec4<f32>) -> f32 {
    let r = select(border_radius.xw, border_radius.yz, offset.x > 0.0);
    let r2 = select(r.x, r.y, offset.y > 0.0);

    let q: vec2<f32> = abs(offset) - size / 2.0 + vec2<f32>(r2);
    let q2: f32 = min(max(q.x, q.y), 0.0);

    let l = length(max(q, vec2(0.0)));
    return q2 + l - r2;
}