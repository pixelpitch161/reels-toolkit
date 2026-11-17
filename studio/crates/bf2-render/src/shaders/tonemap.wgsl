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
    beam_n: u32,
    beam_top: f32,
    beam_k: f32,
    _pad3: f32,
    _pad4: f32,
    _pad5: f32,
    // the beam's outline on screen, two corners to a vec4
    beam: array<vec4<f32>, 18>,
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

// `BAYER_FULL` at an art pixel: the current engine's 4 x 4 ordered dither, 0..15 sixteenths.
fn bayer(p: vec2<f32>) -> f32 {
    var m = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    let c = vec2<u32>(p);
    return m[(c.y & 3u) * 4u + (c.x & 3u)] / 16.0;
}

fn beam_pt(i: u32) -> vec2<f32> {
    let v = u.beam[i / 2u];
    return select(v.xy, v.zw, (i & 1u) == 1u);
}

// The spot's beam (`render`'s "soft wedge of light from the can down to the pool"): inside the outline, a haze of
// one tenth dithered by how far down the frame it is, `floor(fall k 10 + bayer 0.9) / 10`, tinted
// [255, 246, 230] x 0.55 on the 0..255 picture.
fn beam(p: vec2<f32>, size: vec2<f32>) -> vec3<f32> {
    let n = u.beam_n;
    if n < 3u {
        return vec3<f32>(0.0);
    }
    var inside = true;
    for (var i = 0u; i < n; i++) {
        let a = beam_pt(i);
        let b = beam_pt((i + 1u) % n);
        let c = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        if c < 0.0 {
            inside = false;
        }
    }
    if !inside {
        return vec3<f32>(0.0);
    }
    let fall = clamp(1.0 - (p.y - u.beam_top) / (size.y * 520.0 / 640.0), 0.25, 1.0);
    let b = floor(fall * u.beam_k * 10.0 + bayer(p) * 0.9) / 10.0;
    return b * vec3<f32>(255.0, 246.0, 230.0) * 0.55 / 255.0;
}

// The current engine's grade on the encoded picture (`post::grade_cpu` is its CPU reference): the S-curve, the
// saturation about the mean, the vignette and the top scrim.
fn grade(c0: vec3<f32>, p: vec2<f32>, size: vec2<f32>) -> vec3<f32> {
    let v = saturate(c0);
    let s = v + u.contrast * (v * v * (3.0 - 2.0 * v) - v);
    let g = (s.r + s.g + s.b) / 3.0;
    let r = length(vec2<f32>((p.x - 0.5 * size.x) / (0.62 * size.x), (p.y - 0.47 * size.y) / (0.62 * size.y)));
    // banded into fifths and sixths and dithered, as the current engine's `vignette` and `top_scrim` tables are
    let d = bayer(p) * 0.999;
    let vig = 1.0 - min(floor(saturate(r - 0.55) * u.vignette * 5.0 + d) / 5.0, 0.5);
    let scrim = 1.0 - floor(saturate(1.0 - (p.y - size.y * 100.0 / 640.0) / (size.y * 90.0 / 640.0)) * u.scrim * 6.0 + d) / 6.0;
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
    // the beam goes on before the grade, as in the current engine (before its bloom and grade)
    let lit = saturate(curve(max(c, vec3<f32>(0.0))) + beam(p.xy, size));
    return vec4<f32>(grade(lit, p.xy, size), 1.0);
}
