// Shared by the screen passes: the frame's screen-effect settings, a linear sampler, two inputs and the
// full-screen triangle. Every pass of the chain is stateless: its output depends on its inputs and `su` only.

struct ScreenU {
    // w, h, 1/w, 1/h.
    size: vec4<f32>,
    // rgb: HDR colour; w: amount.
    flash: vec4<f32>,
    // invert, letterbox, vignette, grain.
    misc: vec4<f32>,
    // xy: centre (uv); z: amount.
    radial: vec4<f32>,
    // xy: centre (uv); z: amount.
    chroma: vec4<f32>,
    // xy: centre (uv); z: amount.
    lines: vec4<f32>,
    lines_col: vec4<f32>,
    // rgb lift; w: amount.
    grade_lift: vec4<f32>,
    // rgb gamma; w: saturation.
    grade_gamma: vec4<f32>,
    // rgb gain.
    grade_gain: vec4<f32>,
    // rgb: the hue kept; w: amount.
    desat: vec4<f32>,
    // intensity, threshold, knee, lens dirt.
    bloom: vec4<f32>,
    // Each: xy centre (uv), z radius (picture heights), w strength.
    shock: array<vec4<f32>, 4>,
    // x: frame (grain and speed-line seed); y: shockwaves in use; z: the art pixel size (f32 bits; 1: no pixel
    // look).
    frame: vec4<u32>,
}

@group(0) @binding(0) var<uniform> su: ScreenU;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var src: texture_2d<f32>;
@group(0) @binding(3) var src2: texture_2d<f32>;

struct FsIn {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> FsIn {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: FsIn;
    o.pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return o;
}

// No NaN, no infinity, nothing negative: whatever came before, the chain carries finite light.
fn sane(c: vec3<f32>) -> vec3<f32> {
    let ok = c == c;
    return clamp(select(vec3<f32>(0.0), c, ok), vec3<f32>(0.0), vec3<f32>(60000.0));
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// The aspect-correct offset from `c` to `uv`, in picture heights.
fn from_centre(uv: vec2<f32>, c: vec2<f32>) -> vec2<f32> {
    return (uv - c) * vec2<f32>(su.size.x * su.size.w, 1.0);
}
