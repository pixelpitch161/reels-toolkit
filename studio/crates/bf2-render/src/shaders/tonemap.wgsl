// The tone map: resolved HDR colour -> exposure -> the curve (ACES fitted and the sRGB encode, or the current
// engine's straight light and 2.2 gamma) -> grade -> Rgba8Unorm.
// One full-screen triangle; each fragment reads its own texel, so no sampler and no filtering.

struct PostU {
    exposure: f32,
    contrast: f32,
    saturation: f32,
    vignette: f32,
    scrim: f32,
    knee: f32,
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

// The current engine's grade on the encoded picture (`post::grade_cpu` is its CPU reference): the S-curve, the
// saturation about the mean, the vignette and the top scrim.
fn grade(c0: vec3<f32>, p: vec2<f32>, size: vec2<f32>) -> vec3<f32> {
    let v = saturate(c0);
    let s = v + u.contrast * (v * v * (3.0 - 2.0 * v) - v);
    let g = (s.r + s.g + s.b) / 3.0;
    let r = length(vec2<f32>((p.x - 0.5 * size.x) / (0.62 * size.x), (p.y - 0.47 * size.y) / (0.62 * size.y)));
    let vig = 1.0 - min(saturate(r - 0.55) * u.vignette, 0.5);
    let scrim = 1.0 - saturate(1.0 - (p.y - size.y * 100.0 / 640.0) / (size.y * 90.0 / 640.0)) * u.scrim;
    return saturate(saturate(vec3<f32>(g) + (s - vec3<f32>(g)) * u.saturation) * vig * scrim);
}

fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

// `post::curve_cpu`: ACES and the sRGB encode, or (knee above 0) straight light with a shoulder and 2.2 gamma.
fn curve(x: vec3<f32>) -> vec3<f32> {
    if u.knee <= 0.0 {
        return srgb_encode(aces(x));
    }
    let k = u.knee;
    let s = select(x, k + (1.0 - k) * (1.0 - exp(-(x - k) / (1.0 - k))), x > vec3<f32>(k));
    return pow(s, vec3<f32>(1.0 / 2.2));
}

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let c = textureLoad(hdr, vec2<i32>(p.xy), 0).rgb * u.exposure;
    let size = vec2<f32>(textureDimensions(hdr));
    return vec4<f32>(grade(curve(max(c, vec3<f32>(0.0))), p.xy, size), 1.0);
}
