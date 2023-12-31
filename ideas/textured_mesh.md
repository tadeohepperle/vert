# Color Mesh Shader

## Bind Groups

```rs
@group(0) @binding(0)
var<uniform> camera : Camera;
struct Camera {
    view_pos : vec4<f32>,
    view_proj : mat4x4<f32>,
}
```

## Vertex, Instance, Vertex Output

```rs,wgsl
struct Vertex {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
}
struct Instance {
    @location(2) col1: vec4<f32>,
    @location(3) col2: vec4<f32>,
    @location(4) col3: vec4<f32>,
    @location(5) translation: vec4<f32>,
}
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}
```

## Vertex Shader

has access to the following arguments:

- `vertex: `

```rs,vertex
let model_matrix = mat4x4<f32>(
    instance.col1,
    instance.col2,
    instance.col3,
    instance.translation,
);
let world_position = vec4<f32>(vertex.pos, 1.0);
var out: VertexOutput;
out.clip_position = camera.view_proj * model_matrix * world_position;
out.color = vertex.color * vec4(1.0,0.3,0.3,1.0);
return out;
```

```rs,vertex
@group(0) @binding(0)
var<uniform> camera : Camera;

struct Camera {
    view_pos : vec4<f32>,
    view_proj : mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}


struct Vertex {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
}


struct Instance {
    @location(2) col1: vec4<f32>,
    @location(3) col2: vec4<f32>,
    @location(4) col3: vec4<f32>,
    @location(5) translation: vec4<f32>,
}


@vertex
fn vs_main(vertex: Vertex, instance: Instance) -> VertexOutput {

            let model_matrix = mat4x4<f32>(
                instance.col1,
                instance.col2,
                instance.col3,
                instance.translation,
            );
            let world_position = vec4<f32>(vertex.pos, 1.0);
            var out: VertexOutput;
            out.clip_position = camera.view_proj * model_matrix * world_position;
            out.color = vertex.color * vec4(1.0,0.3,0.3,1.0);
            return out;

}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>  {
    return in.color;
}
```
