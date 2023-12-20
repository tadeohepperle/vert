
// ////////////////////////////////////////////////////////////////
// # Bind Groups




// ////////////////////////////////////////////////////////////////
// # Vertex Format
// ```
// struct VertexOutput {
//     @builtin(position) clip_position: vec4<f32>,
//     @location(0) color: vec4<f32>,
// }
// struct Vertex {
//     @location(0) pos: vec3<f32>,
//     @location(1) color: vec4<f32>,
// }
// struct Instance {
//     @location(2) col1: vec4<f32>,
//     @location(3) col2: vec4<f32>,
//     @location(4) col3: vec4<f32>,
//     @location(5) translation: vec4<f32>,
// }
// ``` 
// 
// has access to `vertex: Vertex`, `instance: Instance`, `vertex_index: u32`, `instance_index: u32`
// ////////////////////////////////////////////////////////////////
// # Vertex Shader
//
// input: `vertex: Vertex`, `instance: Instance`, `vertex_index: u32`, `instance_index: u32`
// output: VertexOutput
// ////////////////////////////////////////////////////////////////


// ////////////////////////////////////////////////////////////////
// # Fragment Shader
//
// has access to `vertex_index: u32`, `instance_index: u32`
// - input: `vertex: Vertex`, `instance: Instance`, `vertex_index: u32`, `instance_index: u32`
// - output: vec4<f32>
// ////////////////////////////////////////////////////////////////









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


struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}


struct InstanceInput{
    transform: Transform,
    color: vec4<f32>,
}


struct Transform {
    @location(5) col1: vec4<f32>,
    @location(6) col2: vec4<f32>,
    @location(7) col3: vec4<f32>,
    @location(8) translation: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};


@vertex
fn vs_main(
    vertex: VertexInput,
    transform: Transform,
) -> VertexOutput {
    
    let model_matrix = mat4x4<f32>(
        transform.col1,
        transform.col2,
        transform.col3,
        transform.translation,
    );

    let world_position = vec4<f32>(vertex.position, 1.0, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model_matrix * world_position;
    out.uv = vertex.uv;
    return out;
}
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.uv);
    if color.a < 0.1 {
        discard;
    }
    return color;
}