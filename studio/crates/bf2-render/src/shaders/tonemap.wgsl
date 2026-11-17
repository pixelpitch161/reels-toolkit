// The tone map: resolved HDR colour -> exposure -> ACES fitted -> grade -> sRGB encode -> Rgba8Unorm.
// One full-screen triangle; each fragment reads its own texel, so no sampler and no filtering.

struct PostU {
    exposure: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(0) var hdr: texture_2d<f32>;
@group(0) @binding(1) var<uniform> u: PostU;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    // (0,0), (2,0), (0,2) in uv: a triangle that covers the whole screen.
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

// Narkowicz 2015, "ACES Filmic Tone Mapping Curve". Keep in step with `post::aces` on the CPU.
fn aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

// The grade hook: display-referred linear in, the same out. E3 puts the per-theme grade (LUT) here.
fn grade(c: vec3<f32>) -> vec3<f32> {
    return c;
}

fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let c = textureLoad(hdr, vec2<i32>(p.xy), 0).rgb * u.exposure;
    return vec4<f32>(srgb_encode(grade(aces(max(c, vec3<f32>(0.0))))), 1.0);
}
