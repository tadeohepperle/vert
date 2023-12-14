struct ScreenSpace {
    width: f32,
    height: f32,
    aspect: f32,
}

@group(0) @binding(0)
var<uniform> screen: ScreenSpace;

struct Instance {
    @location(0) posbb: vec4<f32>,
    @location(1) uvbb: vec4<f32>,
    @location(2) color: vec4<f32>,
}


struct Vertex {
    pos: vec2<f32>,
    uv: vec2<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};



// given some bounding box [f32;4] being min x, min y, max x, max y,
// extracts the x,y position [f32;2] for the given index in a counter clockwise quad:
// 0 ------ 1
// | .      |
// |   .    |
// |     .  |
// 3 ------ 2  
fn bounding_box_vertex(idx: u32, posbb: vec4<f32>, uvbb: vec4<f32>) -> Vertex {
    var out: Vertex;
    switch idx {
      case 0u: {
        out.pos = vec2<f32>(posbb[0], posbb[1]); // min x, min y 
        out.uv = vec2<f32>(uvbb[0], uvbb[1]);
      }
      case 1u: {
        out.pos = vec2<f32>(posbb[0], posbb[3]);// min x, max y 
        out.uv = vec2<f32>(uvbb[0], uvbb[3]);
      }
      case 2u: {
        out.pos = vec2<f32>(posbb[2], posbb[3]); // max x, max y
        out.uv = vec2<f32>(uvbb[2], uvbb[3]);
      }
      case 3u, default: {
        out.pos = vec2<f32>(posbb[2], posbb[1]);// max x, min y 
        out.uv = vec2<f32>(uvbb[2], uvbb[1]);
      }
    }
    return out;
}


@vertex
fn vs_main(
    @builtin(vertex_index) idx: u32,
    instance: Instance,
) -> VertexOutput {
    var out: VertexOutput;

    let vertex = bounding_box_vertex(idx, instance.posbb, instance.uvbb);
    let device_pos = vec2<f32>((vertex.pos.x / screen.width)  - 1.0, 1.0 -((vertex.pos.y / screen.height))) ; // + (screen.width * 0.1) + (screen.height* 0.1)
    // let x = f32(1 - i32(idx)) * 0.5;
    // let y = f32(i32(idx & 1u) * 2 - 1) * 0.5;
    // out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    // out.color = vec4<f32>(x, y, 0.0, 1.0);

    let x = f32(1 - i32(idx)) * 0.5;
    let y = f32(i32(idx & 1u) * 2 - 1) * 0.5;

    out.clip_position = vec4<f32>(device_pos, 0.0, 1.0);
    out.color = vec4<f32>(vertex.uv, 0.0, 1.0);
    return out;
}
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}