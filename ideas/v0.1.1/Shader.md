# vert_framework::modules::graphics::shader::color_mesh::ColorMeshShader
Shader Source File. Code in this segment annotated with `rs,wgsl`, `rs,wgsl,vertex` or `rs,wgsl,fragment` will be assembled into a the vert_framework::modules::graphics::shader::color_mesh::ColorMeshShader Shader. 

## Bind Groups
```rs,wgsl,ignore
@group(0) @binding(0)
var<uniform> camera : Camera;

struct Camera {
    view_pos : vec4<f32>,
    view_proj : mat4x4<f32>,
}


``` 

## Vertex, Instance and VertexOutput   
```rs,wgsl,ignore
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
- Inputs: `vertex_index: u32`, `instance_index: u32`, `vertex: Vertex`, `instance: Instance`
- Output: VertexOutput

```rs,wgsl,vertex
// Code here will be inserted into the vertex shader.

var output: VertexOutput;
// set output..
return output;
```

## Fragment Shader
- Inputs: `position: vec4<f32>`, `front_facing: bool`, `frag_depth: f32`, `sample_index: u32`, `sample_mask: u32`, `in: VertexOutput`
- Output: vec4<f32> (The fragment color)

```rs,wgsl,fragment
// Code here will be inserted into the fragment shader.

return vec4(1.0,0.0,0.0,1.0);
```

## Other Code
```rs,wgsl
// Here you can define other structs functions that the parse will pick up on.
```
