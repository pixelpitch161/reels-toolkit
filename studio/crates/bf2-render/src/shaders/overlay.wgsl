// The UI overlay: the premultiplied 8-bit UI matte (360 x 640) over the tone-mapped picture, after the tone map, so
// the UI's bytes are final sRGB values and never bloom. Each output pixel reads the matte texel `pixel / scale`
// (nearest, exact integer scale: the pixel font stays crisp); the pipeline blends One, OneMinusSrcAlpha.

struct OverlayU {
    scale: vec2<u32>,
    _pad: vec2<u32>,
}

@group(0) @binding(0) var ui: texture_2d<f32>;
@group(0) @binding(1) var<uniform> u: OverlayU;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let q = vec2<u32>(p.xy) / u.scale;
    return textureLoad(ui, vec2<i32>(q), 0);
}
