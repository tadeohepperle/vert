// Inspired by: https://www.shadertoy.com/view/fsdyzB
struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct Instance {
    /// rect top left corner and size
    @location(0) pos: vec4<f32>,
    /// rect top left corner and size
    @location(1) uv: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_radius: vec4<f32>,
    
    /// transform
    @location(5) col1: vec4<f32>,
    @location(6) col2: vec4<f32>,
    @location(7) col3: vec4<f32>,
    @location(8) translation: vec4<f32>,
}

// we calculate the vertices here in the shader instead of passing a vertex buffer
struct Vertex {
    pos: vec2<f32>,
    uv: vec2<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    // offset from center
    @location(2) offset: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) border_radius: vec4<f32>,
};

// given some bounding box [f32;4] being min x, min y, max x, max y,
// extracts the x,y position [f32;2] for the given index in a counter clockwise quad:
// 0 ------ 1
// | .      |
// |   .    |
// |     .  |
// 3 ------ 2  
fn rect_vertex(idx: u32, pos: vec4<f32>, uv: vec4<f32>) -> Vertex {
    var out: Vertex;
    switch idx {
      case 0u: {
            out.pos = vec2<f32>(pos.x, pos.y); // min x, min y 
            out.uv = vec2<f32>(uv.x, uv.y);
        }
      case 1u: {
            out.pos = vec2<f32>(pos.x, pos.y + pos.w); // min x, max y 
            out.uv = vec2<f32>(uv.x, uv.y + uv.w);
        }
      case 2u: {
            out.pos = vec2<f32>(pos.x + pos.z, pos.y + pos.w); // max x, max y
            out.uv = vec2<f32>(uv.x + uv.z, uv.y + uv.w);
        }
      case 3u, default: {
            out.pos = vec2<f32>(pos.x + pos.z, pos.y); // max x, min y 
            out.uv = vec2<f32>(uv.x + uv.z, uv.y);
        }
    }
    return out;
}


@vertex
fn vs_main(
    @builtin(vertex_index) idx: u32,
    instance: Instance,
) -> VertexOutput {

    let vertex = rect_vertex(idx, instance.pos, instance.uv);
    // offset as if it was on a screen that is the xy plane, with 100 pixels per unit.
    let xy_plane_offset = vec2<f32>(vertex.pos.x / 100.0, -vertex.pos.y / 100.0);

    let model_matrix = mat4x4<f32>(
        instance.col1,
        instance.col2,
        instance.col3,
        instance.translation,
    );
    let world_position = vec4<f32>(xy_plane_offset, 0.0, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model_matrix * world_position;

    out.border_radius = instance.border_radius;
    out.size = instance.pos.zw;
    let center = instance.pos.xy + instance.pos.zw * 0.5;
    out.offset = vertex.pos - center;
    out.color = instance.color;
    out.uv = vertex.uv;
    return out;
}
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {


    let image_color = textureSample(t_diffuse, s_diffuse, in.uv);
    let color = mix(image_color.rgb, image_color.rgb * in.color.rgb, in.color.a);

 

    // return mix(image_color, vec4<f32>(1.0,0.0,0.0,1.0), 0.5);

    /// the borders are counterclockwise: topleft, topright, bottomright, bottomleft
    let sdf = rounded_box_sdf(in.offset, in.size, in.border_radius);
    let opacity = min(image_color.a, smoothstep(1.0, 0.0, sdf + 0.5)); // the + 0.5 makes the edge a bit smoother
    return vec4(color, opacity);
}


fn rounded_box_sdf(offset: vec2<f32>, size: vec2<f32>, radius: vec4<f32>) -> f32 {
    let r = select(radius.xw, radius.yz, offset.x > 0.0);
    let r2 = select(r.x, r.y, offset.y > 0.0);


    let q: vec2<f32> = abs(offset) - size / 2.0 + vec2<f32>(r2);
    let q2: f32 = min(max(q.x, q.y), 0.0);

    let l = length(max(q, vec2(0.0)));
    return q2 + l - r2;
    // return length(max(abs(center)-size+vec2<f32>(r2),vec2<f32>(0.0)))-r2;
}


// float roundedBoxSDF(vec2 CenterPosition, vec2 Size, vec4 Radius)
// {
//     Radius.xy =   (CenterPosition.x > 0.0) ? Radius.xy : Radius.zw;
//     Radius.x  = (CenterPosition.y > 0.0) ? Radius.x  : Radius.y;
    
//     vec2 q = abs(CenterPosition)-Size+Radius.x;
//     return min(max(q.x,q.y),0.0) + length(max(q,0.0)) - Radius.x;
// }

// float roundedBoxSDF(vec2 CenterPosition, vec2 Size, float Radius) {
//     return 
// }

// fn roundedBoxSDF(center: vec2<f32>, size: vec2<f32>, radius: vec4<f32>) -> f32 {
//     Radius.xy = (CenterPosition.x > 0.0) ? Radius.xy : Radius.zw;
//     Radius.x  = (CenterPosition.y > 0.0) ? Radius.x  : Radius.y;
    
//     vec2 q = abs(CenterPosition)-Size+Radius.x;
//     return min(max(q.x,q.y),0.0) + length(max(q,0.0)) - Radius.x;
// }
