// Inspired by: https://www.shadertoy.com/view/fsdyzB
struct ScreenSize {
    width: f32,
    height: f32,
    aspect: f32,
}

@group(0) @binding(0)
var<uniform> screen: ScreenSize;

struct Instance {
    @location(0) pos: vec4<f32>, // pos aabb for the glyph
    @location(1) color: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) border_color: vec4<f32>,
    // border_thickness, border_softness, _unused, _unused
    @location(4) others: vec4<f32>,
}

// we calculate the vertices here in the shader instead of passing a vertex buffer
struct Vertex {
    pos: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) offset: vec2<f32>, // offset from center
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_radius: vec4<f32>,
    @location(4) border_color: vec4<f32>,
     // border_thickness, _unused, _unused, _unused
    @location(5) others: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: Instance,
) -> VertexOutput {
    let vertex = rect_vertex(vertex_index, instance.pos);
    let device_pos = vec2<f32>((vertex.pos.x / screen.width) * 2.0 - 1.0, 1.0 - (vertex.pos.y / screen.height) * 2.0) ;
    let center = instance.pos.xy + instance.pos.zw * 0.5;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(device_pos, 0.0, 1.0);
    out.offset = vertex.pos - center;
    out.size = instance.pos.zw;

    out.color = instance.color;
    out.border_radius = instance.border_radius;
    out.border_color = instance.border_color;
    out.others = instance.others;
    return out;
}
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sdf = rounded_box_sdf(in.offset, in.size, in.border_radius);
    let color: vec4<f32> = mix(in.color, in.border_color, smoothstep(0.0, 1.0, ((sdf + in.others[0]) / in.others[1]) ));

    let alpha = min(color.a, smoothstep(1.0, 0.0, sdf + 0.5)); // the + 0.5 makes the edge a bit smoother
    // return vec4(in.color.rgb, alpha);
    return vec4(color.rgb, alpha);
}

// given some bounding box aabb [f32;4] being min x, min y, max x, max y,
// extracts the x,y position [f32;2] for the given index in a counter clockwise quad:
// 0 ------ 1
// | .      |
// |   .    |
// |     .  |
// 3 ------ 2  
fn rect_vertex(idx: u32, pos: vec4<f32>) -> Vertex {
    var out: Vertex;
    switch idx {
      case 0u, 4u: {
            out.pos = vec2<f32>(pos.x, pos.y); // min x, min y 
        }
      case 1u: {
            out.pos = vec2<f32>(pos.x, pos.w); // min x, max y 
        }
      case 2u, 5u: {
            out.pos = vec2<f32>(pos.z, pos.w); // max x, max y
        }
      case 3u, default: {
            out.pos = vec2<f32>(pos.z, pos.y); // max x, min y 
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