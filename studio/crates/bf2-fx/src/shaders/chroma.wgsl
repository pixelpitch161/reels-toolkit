// Chromatic aberration: red pushed out from the centre, blue pulled in, growing toward the edges.

const GLOW_BAYER = array<u32, 16>(0u, 8u, 2u, 10u, 12u, 4u, 14u, 6u, 3u, 11u, 1u, 9u, 15u, 7u, 13u, 5u);

// The pixel look: the glow's light banded into thirds with the current engine's Bayer dither at the art pixel
// (its glows are `floor(m * 3 + BAYER * 0.9) / 3` of a 0..1 mask). HDR glow is first brought to 0..1 as
// `m / (1 + m)`, banded there and brought back, so a halo is a few dithered rings, not a smooth haze; the
// white-hot top band (past two thirds) keeps its light.
fn glow_steps(g: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let art = bitcast<f32>(su.frame.z);
    if art <= 1.0 {
        return g;
    }
    let q = vec2<u32>(floor(floor(uv * su.size.xy) / art));
    let b = f32(GLOW_BAYER[(q.y % 4u) * 4u + q.x % 4u]) / 16.0;
    let m = max(max(g.r, g.g), g.b);
    if m <= 0.0 {
        return g;
    }
    let t = floor(m / (1.0 + m) * 3.0 + b * 0.9) / 3.0;
    if t >= 1.0 {
        return g;
    }
    return g * (t / (1.0 - t)) / m;
}

fn scene_at(uv: vec2<f32>) -> vec3<f32> {
    let bloom = textureSampleLevel(src2, samp, uv, 0.0).rgb;
    let dirt = 1.0 + su.bloom.w * lens_dirt(uv);
    return sane(textureSampleLevel(src, samp, uv, 0.0).rgb) + glow_steps(sane(bloom) * su.bloom.x * dirt, uv);
}

fn chroma_sample(uv: vec2<f32>) -> vec3<f32> {
    let amount = su.chroma.z;
    if amount <= 0.0 {
        return scene_at(uv);
    }
    let d = uv - su.chroma.xy;
    let k = 0.012 * amount * length(d) * 2.0;
    let r = scene_at(uv + d * k).r;
    let g = scene_at(uv).g;
    let b = scene_at(uv - d * k).b;
    return vec3<f32>(r, g, b);
}
