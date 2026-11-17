// Speed lines: thin streaks radiating from a centre, a fresh set each frame, clear of the middle.

fn speed_lines(uv: vec2<f32>) -> f32 {
    let amount = su.lines.z;
    if amount <= 0.0 {
        return 0.0;
    }
    let d = from_centre(uv, su.lines.xy);
    let r = length(d);
    let a = (atan2(d.y, d.x) / 6.2831853 + 0.5) * 220.0;
    let id = u32(floor(a));
    let seed = su.frame.x * 7u + 3u;
    let on = step(0.55, h3(seed, id, 0u));
    let f = fract(a) - 0.5;
    let w = 0.12 + 0.25 * h3(seed, id, 1u);
    let start = 0.18 + 0.25 * h3(seed, id, 2u);
    let line = exp(-(f * f) / (w * w * 0.1)) * smoothstep(start, start + 0.2, r);
    return on * line * amount;
}
