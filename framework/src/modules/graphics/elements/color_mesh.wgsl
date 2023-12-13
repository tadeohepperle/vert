struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct Transform {
    @location(5) col1: vec4<f32>,
    @location(6) col2: vec4<f32>,
    @location(7) col3: vec4<f32>,
    @location(8) translation: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
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

    let world_position = vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model_matrix * world_position;
    out.color = vertex.color;
    return out;
}
 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}