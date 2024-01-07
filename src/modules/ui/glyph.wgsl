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

struct Instance {
    @location(0) pos: vec4<f32>, // pos aabb for the glyph
    @location(1) color: vec4<f32>,
    @location(2) uv: vec4<f32>,    // uv aabb in the texture atlas
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
    @location(2) offset: vec2<f32>, // offset from center
    @location(3) size: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: Instance,
) -> VertexOutput {
    let vertex = rect_vertex(vertex_index, instance.pos, instance.uv);
    let device_pos = vec2<f32>((vertex.pos.x / screen.width) * 2.0 - 1.0, 1.0 - (vertex.pos.y / screen.height) * 2.0) ;
    let center = instance.pos.xy + instance.pos.zw * 0.5;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(device_pos, 0.0, 1.0);
    out.color = instance.color;
    out.uv = vertex.uv; 
    out.offset = vertex.pos - center;
    out.size = instance.pos.zw;
    return out;
}
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
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
fn rect_vertex(idx: u32, pos: vec4<f32>, uv: vec4<f32>) -> Vertex {
    var out: Vertex;
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

