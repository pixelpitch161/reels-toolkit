// The tone map: resolved HDR colour -> exposure -> the curve (ACES fitted and the sRGB encode, or the current
// engine's straight light and 2.2 gamma) -> grade -> the value steps and Bayer dither -> Rgba8Unorm.
// One full-screen triangle, no sampler and no filtering. With the pixel pass on (`pixel` above 1), each fragment
// finds its art pixel, resolves the HDR texels the art pixel covers hard-edged (`resolve`), and grades and
// dithers at the art pixel, so every output pixel of one art pixel is the same colour (`post::Post::pixel`).

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
// saturation about the mean, the vignette and the top scrim. With the value steps on (`step` above 0) the vignette
// is banded into fifths and the scrim into sixths, each dithered by the Bayer matrix at the art pixel `q`, as the
// current engine's `vignette` and `top_scrim` tables are (`post::vignette_cpu`, `post::scrim_cpu`).
fn grade(c0: vec3<f32>, p: vec2<f32>, size: vec2<f32>, q: vec2<u32>) -> vec3<f32> {
    let v = saturate(c0);
    let s = v + u.contrast * (v * v * (3.0 - 2.0 * v) - v);
    let g = (s.r + s.g + s.b) / 3.0;
    let r = length(vec2<f32>((p.x - 0.5 * size.x) / (0.62 * size.x), (p.y - 0.47 * size.y) / (0.62 * size.y)));
    var dv = saturate(r - 0.55) * u.vignette;
    var ds = saturate(1.0 - (p.y - size.y * 100.0 / 640.0) / (size.y * 90.0 / 640.0)) * u.scrim;
    if u.step > 0.0 {
        let b = bayer(q);
        dv = floor(dv * 5.0 + b * 0.999) / 5.0;
        ds = floor(ds * 6.0 + b * 0.999) / 6.0;
    }
    let vig = 1.0 - min(dv, 0.5);
    let scrim = 1.0 - ds;
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

fn bayer(q: vec2<u32>) -> f32 {
    return f32(BAYER[(q.y % 4u) * 4u + q.x % 4u]) / 16.0;
}

// `post::dither_cpu`: floor(v / step + bayer) * step on 0..255, the Bayer matrix on the art grid.
fn dither(v: vec3<f32>, q: vec2<u32>) -> vec3<f32> {
    if u.step <= 0.0 {
        return v;
    }
    return clamp(floor(v * 255.0 / u.step + bayer(q)) * u.step, vec3<f32>(0.0), vec3<f32>(255.0)) / 255.0;
}

// Brightness for the resolve's comparisons: the square root of the luminance, near the encoded value.
fn lum(c: vec3<f32>) -> f32 {
    return sqrt(max(dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.0));
}

// Below this spread of brightness inside an art pixel it is a flat or gently shaded patch: the mean.
const FLAT: f32 = 0.06;
// The widest window the resolve reads: the art pixel and a ring of one texel round it (art pixels up to 6 wide).
const WIN: i32 = 8;

// One art pixel of the HDR picture, hard-edged like the current engine's aliased drawing. A flat patch is the mean of its texels. Where it holds an edge, its texels split into a
// bright and a dark side about the middle brightness, and the art pixel takes the mean of one side only, so no
// in-between colour is left: the side that covers more of it, unless the other side is a thin line or dot (no
// 2 x 2 square of that side anywhere in the art pixel and the ring round it), which takes the whole art pixel so
// ropes, cracks, sparks and ink lines stay whole.
fn resolve(q: vec2<f32>, s: f32, size: vec2<i32>) -> vec3<f32> {
    let lo = vec2<i32>(floor(q * s));
    let hi = min(vec2<i32>(ceil((q + 1.0) * s)), size) - vec2<i32>(1);
    if all(lo == hi) {
        return textureLoad(hdr, lo, 0).rgb;
    }
    if any(hi - lo + vec2<i32>(3) > vec2<i32>(WIN)) {
        // wider than the window holds (art pixels over 6 wide): the centre texel
        return textureLoad(hdr, vec2<i32>(floor((q + 0.5) * s)), 0).rgb;
    }
    // the window: the art pixel and its ring, clamped to the picture
    let w0 = max(lo - vec2<i32>(1), vec2<i32>(0));
    let w1 = min(hi + vec2<i32>(1), size - vec2<i32>(1));
    var l: array<f32, 64>;
    var sum = vec3<f32>(0.0);
    var lmin = 1e9;
    var lmax = -1e9;
    for (var y = w0.y; y <= w1.y; y++) {
        for (var x = w0.x; x <= w1.x; x++) {
            let c = textureLoad(hdr, vec2<i32>(x, y), 0).rgb;
            let v = lum(c);
            l[(y - w0.y) * WIN + (x - w0.x)] = v;
            if x >= lo.x && x <= hi.x && y >= lo.y && y <= hi.y {
                sum += c;
                lmin = min(lmin, v);
                lmax = max(lmax, v);
            }
        }
    }
    let n = f32((hi.x - lo.x + 1) * (hi.y - lo.y + 1));
    if lmax - lmin < FLAT {
        return sum / n;
    }
    let mid = 0.5 * (lmin + lmax);
    // each side's count and colour inside the art pixel
    var nb = 0.0;
    var sb = vec3<f32>(0.0);
    var sd = vec3<f32>(0.0);
    for (var y = lo.y; y <= hi.y; y++) {
        for (var x = lo.x; x <= hi.x; x++) {
            let c = textureLoad(hdr, vec2<i32>(x, y), 0).rgb;
            if l[(y - w0.y) * WIN + (x - w0.x)] > mid {
                nb += 1.0;
                sb += c;
            } else {
                sd += c;
            }
        }
    }
    let nd = n - nb;
    // a 2 x 2 square of one side in the window means that side is a region, not a line
    var wide_b = false;
    var wide_d = false;
    for (var y = w0.y; y < w1.y; y++) {
        for (var x = w0.x; x < w1.x; x++) {
            let i = (y - w0.y) * WIN + (x - w0.x);
            let a = l[i] > mid;
            let b = l[i + 1] > mid;
            let c = l[i + WIN] > mid;
            let d = l[i + WIN + 1] > mid;
            wide_b = wide_b || (a && b && c && d);
            wide_d = wide_d || !(a || b || c || d);
        }
    }
    let bright = sb / max(nb, 1.0);
    let dark = sd / max(nd, 1.0);
    if nb <= nd && !wide_b {
        return bright;
    }
    if nd <= nb && !wide_d {
        return dark;
    }
    if nb > nd {
        return bright;
    }
    if nd > nb {
        return dark;
    }
    let ci = vec2<i32>(floor((q + 0.5) * s)) - w0;
    return select(dark, bright, l[ci.y * WIN + ci.x] > mid);
}

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(hdr));
    // output pixels per art pixel: `pixel` across a 1080-wide frame, never under one
    let s = max(u.pixel * size.x / 1080.0, 1.0);
    // the pixel's index over s, as the UI layer's nearest fit counts it (`job2::fit`), so the HUD shares the grid
    let q = floor(floor(p.xy) / s);
    let centre = (q + 0.5) * s;
    let c = resolve(q, s, vec2<i32>(size)) * u.exposure;
    let g = grade(curve(max(c, vec3<f32>(0.0))), centre, size, vec2<u32>(q));
    return vec4<f32>(dither(g, vec2<u32>(q)), 1.0);
}
