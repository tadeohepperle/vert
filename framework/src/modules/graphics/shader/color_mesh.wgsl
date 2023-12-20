fn vertex() {
    let model_matrix = mat4x4<f32>(
        instance.col1,
        instance.col2,
        instance.col3,
        instance.translation,
    );
    let world_position = vec4<f32>(vertex.pos, 1.0);
    
    var out: VertexOutput;
    out.clip_position = camera.view_proj * model_matrix * world_position;
    return out;
}

fn fragment() {
    return vec4(1.0,1.0,0.0,1.0);
}

