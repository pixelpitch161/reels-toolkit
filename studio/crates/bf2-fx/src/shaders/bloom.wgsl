// Bloom: a soft threshold, then a dual-filter pyramid (Marius Bjørge's 5-tap down, 8-tap up) of six levels.

@fragment
fn fs_prefilter(in: FsIn) -> @location(0) vec4<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(src));
    var c = vec3<f32>(0.0);
    // A 4-tap box with each tap's weight cut by its brightness (Karis), so lone hot pixels don't flicker.
    var wsum = 0.0;
    for (var i = 0u; i < 4u; i++) {
        let o = vec2<f32>(f32(i & 1u) - 0.5, f32(i >> 1u) - 0.5) * t;
        let s = min(sane(textureSampleLevel(src, samp, in.uv + o, 0.0).rgb), vec3<f32>(256.0));
        let w = 1.0 / (1.0 + luma(s));
        c += s * w;
        wsum += w;
    }
    c /= wsum;
    let b = max(max(c.r, c.g), c.b);
    let thr = su.bloom.y;
    let knee = max(su.bloom.z, 1e-4);
    let soft = clamp(b - thr + knee, 0.0, 2.0 * knee);
    let k = max(soft * soft / (4.0 * knee), b - thr) / max(b, 1e-4);
    return vec4<f32>(c * k, 1.0);
}

@fragment
fn fs_down(in: FsIn) -> @location(0) vec4<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(src));
    var c = textureSampleLevel(src, samp, in.uv, 0.0).rgb * 4.0;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(-t.x, -t.y), 0.0).rgb;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(t.x, -t.y), 0.0).rgb;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(-t.x, t.y), 0.0).rgb;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(t.x, t.y), 0.0).rgb;
    return vec4<f32>(c / 8.0, 1.0);
}

@fragment
fn fs_up(in: FsIn) -> @location(0) vec4<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(src));
    var c = vec3<f32>(0.0);
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(-2.0 * t.x, 0.0), 0.0).rgb;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(2.0 * t.x, 0.0), 0.0).rgb;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(0.0, -2.0 * t.y), 0.0).rgb;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(0.0, 2.0 * t.y), 0.0).rgb;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(-t.x, -t.y), 0.0).rgb * 2.0;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(t.x, -t.y), 0.0).rgb * 2.0;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(-t.x, t.y), 0.0).rgb * 2.0;
    c += textureSampleLevel(src, samp, in.uv + vec2<f32>(t.x, t.y), 0.0).rgb * 2.0;
    // Added (by blending) onto the finer level's own downsample.
    return vec4<f32>(c / 12.0, 1.0);
}

// Lens dirt: smudges and a scatter of soft specks, which the bloom lights up.
fn lens_dirt(uv: vec2<f32>) -> f32 {
    let p = uv * vec2<f32>(su.size.x * su.size.w, 1.0);
    var d = 0.35 * smooth01(fbm(p * 2.5, 977u) * 1.6 - 0.4);
    let cell = p * 7.0;
    let id = floor(cell);
    let ci = vec2<u32>(vec2<i32>(id) + vec2<i32>(4096));
    let h = h3(4242u, ci.x * 131u + ci.y, 0u);
    if h > 0.72 {
        let c = id + vec2<f32>(h3(4242u, ci.x * 131u + ci.y, 1u), h3(4242u, ci.x * 131u + ci.y, 2u));
        let r = length(cell - c);
        d += 0.8 * (1.0 - smoothstep(0.05, 0.35 * h, r));
    }
    return d;
}
