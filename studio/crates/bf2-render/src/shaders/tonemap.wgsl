// The tone map: resolved HDR colour -> exposure -> the curve (ACES fitted and the sRGB encode, or the current
// engine's straight light and 2.2 gamma) -> grade -> the value steps and Bayer dither -> Rgba8Unorm.
// One full-screen triangle, no sampler and no filtering. With the pixel pass on (`pixel` above 1), each fragment
// finds its art pixel and reads the texel at that art pixel's centre, and grades and dithers at the art pixel, so
// every output pixel of one art pixel is the same colour (`post::Post::pixel`, `post::dither_cpu`).

struct PostU {
    exposure: f32,
    contrast: f32,
    saturation: f32,
    vignette: f32,
    scrim: f32,
    knee: f32,
    pixel: f32,
    step: f32,
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

// `BAYER` of the current engine (`bf_picture::config::BAYER`), in sixteenths, rows top to bottom.
const BAYER = array<u32, 16>(0u, 8u, 2u, 10u, 12u, 4u, 14u, 6u, 3u, 11u, 1u, 9u, 15u, 7u, 13u, 5u);

// `post::dither_cpu`: floor(v / step + bayer) * step on 0..255, the Bayer matrix on the art grid.
fn dither(v: vec3<f32>, q: vec2<u32>) -> vec3<f32> {
    if u.step <= 0.0 {
        return v;
    }
    let b = f32(BAYER[(q.y % 4u) * 4u + q.x % 4u]) / 16.0;
    return clamp(floor(v * 255.0 / u.step + b) * u.step, vec3<f32>(0.0), vec3<f32>(255.0)) / 255.0;
}

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(hdr));
    // output pixels per art pixel: `pixel` across a 1080-wide frame, never under one
    let s = max(u.pixel * size.x / 1080.0, 1.0);
    // the pixel's index over s, as the UI layer's nearest fit counts it (`job2::fit`), so the HUD shares the grid
    let q = floor(floor(p.xy) / s);
    let centre = (q + 0.5) * s;
    let texel = min(floor(centre), size - 1.0);
    let c = textureLoad(hdr, vec2<i32>(texel), 0).rgb * u.exposure;
    let g = grade(curve(max(c, vec3<f32>(0.0))), centre, size);
    return vec4<f32>(dither(g, vec2<u32>(q)), 1.0);
}
