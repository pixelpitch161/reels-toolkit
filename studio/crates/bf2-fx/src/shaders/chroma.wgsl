// Chromatic aberration: red pushed out from the centre, blue pulled in, growing toward the edges.

fn scene_at(uv: vec2<f32>) -> vec3<f32> {
    let bloom = textureSampleLevel(src2, samp, uv, 0.0).rgb;
    let dirt = 1.0 + su.bloom.w * lens_dirt(uv);
    return sane(textureSampleLevel(src, samp, uv, 0.0).rgb) + sane(bloom) * su.bloom.x * dirt;
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
