// Source: Adapted from Bevy Source code, which in turn is from COD GDC talk.

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
    let sample = sample_input_13_tap(vs.uv);
    return vec4(sample, 1.0);
}

@fragment
fn threshold_downsample(vs: VertexOutput) -> @location(0) vec4<f32> {
    let sample = sample_input_13_tap(vs.uv);
    return vec4(soft_threshold(sample.xyz), 1.0);
}

@fragment
fn upsample(vs: VertexOutput) -> @location(0) vec4<f32> {
    let sample = sample_input_3x3_tent(vs.uv);
    return vec4(sample,1.0);
}


// // [COD] slide 162
fn sample_input_3x3_tent(uv: vec2<f32>) -> vec3<f32> {
    // Radius. Empirically chosen by and tweaked from the LearnOpenGL article.
    let x = 0.004 / screen.aspect;
    let y = 0.004;

    let a = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x - x, uv.y + y)).rgb;
    let b = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x, uv.y + y)).rgb;
    let c = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x + x, uv.y + y)).rgb;

    let d = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x - x, uv.y)).rgb;
    let e = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x, uv.y)).rgb;
    let f = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x + x, uv.y)).rgb;

    let g = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x - x, uv.y - y)).rgb;
    let h = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x, uv.y - y)).rgb;
    let i = textureSample(hdr_image, hdr_sampler, vec2<f32>(uv.x + x, uv.y - y)).rgb;

    var sample = e * 0.25;
    sample += (b + d + f + h) * 0.125;
    sample += (a + c + g + i) * 0.0625;

    return sample;
}

/// Todo! look at bevy code where this comes from and 
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


// [COD] slide 153
fn sample_input_13_tap(uv: vec2<f32>) -> vec3<f32> {
    let a = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(-2, 2)).rgb;
    let b = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(0, 2)).rgb;
    let c = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(2, 2)).rgb;
    let d = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(-2, 0)).rgb;
    let e = textureSample(hdr_image, hdr_sampler, uv).rgb;
    let f = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(2, 0)).rgb;
    let g = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(-2, -2)).rgb;
    let h = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(0, -2)).rgb;
    let i = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(2, -2)).rgb;
    let j = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(-1, 1)).rgb;
    let k = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(1, 1)).rgb;
    let l = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(-1, -1)).rgb;
    let m = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(1, -1)).rgb;

    var sample = (a + c + g + i) * 0.03125;
    sample += (b + d + f + h) * 0.0625;
    sample += (e + j + k + l + m) * 0.125;
    return sample;
}



// fn sample_input_13_tap_initial(uv: vec2<f32>) -> vec3<f32> {
//     let a = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(-2, 2)).rgb;
//     let b = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(0, 2)).rgb;
//     let c = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(2, 2)).rgb;
//     let d = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(-2, 0)).rgb;
//     let e = textureSample(hdr_image, hdr_sampler, uv).rgb;
//     let f = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(2, 0)).rgb;
//     let g = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(-2, -2)).rgb;
//     let h = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(0, -2)).rgb;
//     let i = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(2, -2)).rgb;
//     let j = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(-1, 1)).rgb;
//     let k = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(1, 1)).rgb;
//     let l = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(-1, -1)).rgb;
//     let m = textureSample(hdr_image, hdr_sampler, uv, vec2<i32>(1, -1)).rgb;

//     // [COD] slide 168
//     //
//     // The first downsample pass reads from the rendered frame which may exhibit
//     // 'fireflies' (individual very bright pixels) that should not cause the bloom effect.
//     //
//     // The first downsample uses a firefly-reduction method proposed by Brian Karis
//     // which takes a weighted-average of the samples to limit their luma range to [0, 1].
//     // This implementation matches the LearnOpenGL article [PBB].
//     var group0 = (a + b + d + e) * (0.125f / 4.0f);
//     var group1 = (b + c + e + f) * (0.125f / 4.0f);
//     var group2 = (d + e + g + h) * (0.125f / 4.0f);
//     var group3 = (e + f + h + i) * (0.125f / 4.0f);
//     var group4 = (j + k + l + m) * (0.5f / 4.0f);
//     // group0 *= karis_average(group0);
//     // group1 *= karis_average(group1);
//     // group2 *= karis_average(group2);
//     // group3 *= karis_average(group3);
//     // group4 *= karis_average(group4);
//     return group0 + group1 + group2 + group3 + group4;
// }


// fn karis_average(color: vec3<f32>) -> f32 {
//     // Luminance calculated by gamma-correcting linear RGB to non-linear sRGB using pow(color, 1.0 / 2.2)
//     // and then calculating luminance based on Rec. 709 color primaries.
//     let luma = tonemapping_luminance(rgb_to_srgb_simple(color)) / 4.0;
//     return 1.0 / (1.0 + luma);
// }