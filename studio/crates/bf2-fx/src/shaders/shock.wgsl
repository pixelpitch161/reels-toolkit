// Shockwave refraction: up to four expanding rings that push the picture outward where they pass.

fn shock_offset(uv: vec2<f32>) -> vec2<f32> {
    var off = vec2<f32>(0.0);
    for (var i = 0u; i < min(su.frame.y, 4u); i++) {
        let s = su.shock[i];
        let d = from_centre(uv, s.xy);
        let r = length(d);
        let width = 0.035 + 0.15 * s.z;
        let x = (r - s.z) / width;
        // A refraction lobe: out ahead of the front, in behind it; exactly 0 past 2.5 widths, so the picture away
        // from the front is untouched (HDR edges would show even a hundredth of a pixel).
        let k = s.w * x * exp(-x * x) * (1.0 - smoothstep(2.0, 2.5, abs(x)));
        if r > 1e-4 {
            off += (d / r) * k * vec2<f32>(su.size.y * su.size.z, 1.0);
        }
    }
    return off;
}
