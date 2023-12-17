struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) clip_position: vec4<f32>,
};


struct ScreenSpace {
    width: f32,
    height: f32,
    aspect: f32,
}

@group(0) @binding(0)
var<uniform> screen: ScreenSpace;

@group(1)
@binding(0)
var hdr_image: texture_2d<f32>;

@group(1)
@binding(1)
var hdr_sampler: sampler;

@fragment
fn downsample(vs: VertexOutput) -> @location(0) vec4<f32> {
    let sample1 = textureSample(hdr_image, hdr_sampler, vs.uv);
    let sample2 = textureSample(hdr_image, hdr_sampler, vec2(vs.uv.x, vs.uv.y + 0.01));
    let sample3 = textureSample(hdr_image, hdr_sampler, vec2(vs.uv.x, vs.uv.y + 0.01));
    return (sample1 + sample2 + sample3) * 0.33333;
}


@fragment
fn threshold_downsample(vs: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(hdr_image, hdr_sampler, vs.uv);
    return vec4(soft_threshold(sample.xyz), sample.a);
}


@fragment
fn upsample(vs: VertexOutput) -> @location(0) vec4<f32> {
    let sample1 = textureSample(hdr_image, hdr_sampler, vs.uv);
    let sample2 = textureSample(hdr_image, hdr_sampler, vec2(vs.uv.x, vs.uv.y + 0.01));
    let sample3 = textureSample(hdr_image, hdr_sampler, vec2(vs.uv.x, vs.uv.y + 0.01));
    return (sample1 + sample2 + sample3) * 0.33333;
}




// // [COD] slide 162
// fn sample_input_3x3_tent(uv: vec2<f32>) -> vec3<f32> {
//     // Radius. Empirically chosen by and tweaked from the LearnOpenGL article.
//     let x = 0.004 / uniforms.aspect;
//     let y = 0.004;

//     let a = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x - x, uv.y + y)).rgb;
//     let b = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x, uv.y + y)).rgb;
//     let c = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x + x, uv.y + y)).rgb;

//     let d = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x - x, uv.y)).rgb;
//     let e = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x, uv.y)).rgb;
//     let f = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x + x, uv.y)).rgb;

//     let g = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x - x, uv.y - y)).rgb;
//     let h = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x, uv.y - y)).rgb;
//     let i = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x + x, uv.y - y)).rgb;

//     var sample = e * 0.25;
//     sample += (b + d + f + h) * 0.125;
//     sample += (a + c + g + i) * 0.0625;

//     return sample;
// }


const x: f32 = 0.5;
const y: f32 = 1.0;
const z: f32 = 1.0;
const w: f32 = 1.0;

fn soft_threshold(color: vec3<f32>) -> vec3<f32> {
    let brightness = max(color.r, max(color.g, color.b));
    var softness = brightness - y;
    softness = clamp(softness, 0.0, z);
    softness = softness * softness * w;
    var contribution = max(brightness - x, softness);
    contribution /= max(brightness, 0.00001); // Prevent division by 0
    return color * contribution;
}