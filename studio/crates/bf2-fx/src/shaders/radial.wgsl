// Radial (zoom) blur: 16 taps from the pixel toward the centre.

fn radial_sample(uv: vec2<f32>) -> vec3<f32> {
    let amount = su.radial.z;
    if amount <= 0.0 {
        return sane(textureSampleLevel(src, samp, uv, 0.0).rgb);
    }
    var acc = vec3<f32>(0.0);
    var wsum = 0.0;
    let toward = su.radial.xy - uv;
    for (var i = 0u; i < 16u; i++) {
        // Taps bunched near the pixel (f squared), so the streaks trail off instead of the picture looking zoomed.
        let f = f32(i) / 15.0;
        let w = 1.0 - 0.5 * f;
        acc += sane(textureSampleLevel(src, samp, uv + toward * (f * f * 0.22 * amount), 0.0).rgb) * w;
        wsum += w;
    }
    return acc / wsum;
}

@fragment
fn fs_warp(in: FsIn) -> @location(0) vec4<f32> {
    let uv = in.uv + shock_offset(in.uv);
    return vec4<f32>(radial_sample(uv), 1.0);
}
