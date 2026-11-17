// Grade (lift, gamma, gain, saturation), desaturate keeping one hue, vignette, grain, letterbox; and the
// composite pass that runs the chain's last steps in order.

fn grade(c: vec3<f32>) -> vec3<f32> {
    let amount = clamp(su.grade_lift.w, 0.0, 1.0);
    var o = c;
    if amount > 0.0 {
        let g = max(su.grade_gamma.rgb, vec3<f32>(0.05));
        var x = max(c * su.grade_gain.rgb + su.grade_lift.rgb * 0.1, vec3<f32>(0.0));
        x = pow(x, 1.0 / g);
        x = mix(vec3<f32>(luma(x)), x, su.grade_gamma.w);
        o = mix(c, x, amount);
    }
    let da = clamp(su.desat.w, 0.0, 1.0);
    if da > 0.0 {
        let l = luma(o);
        let n = length(o);
        let keep = normalize(max(su.desat.rgb, vec3<f32>(1e-4)));
        let sim = select(0.0, dot(o / max(n, 1e-6), keep), n > 1e-6);
        let kept = smoothstep(0.88, 0.97, sim);
        o = mix(o, vec3<f32>(l), da * (1.0 - kept));
    }
    return o;
}

@fragment
fn fs_composite(in: FsIn) -> @location(0) vec4<f32> {
    let uv = in.uv;
    var c = chroma_sample(uv);
    c += su.lines_col.rgb * speed_lines(uv);
    c = flash_invert(c);
    c = grade(c);
    // Vignette.
    let d = from_centre(uv, vec2<f32>(0.5));
    c *= 1.0 - su.misc.z * smoothstep(0.15, 0.75, dot(d, d) * 2.2);
    // Grain: multiplicative, fresh each frame.
    let px = vec2<u32>(in.pos.xy);
    let n = h3(su.frame.x, px.y * 8192u + px.x, 5u) - 0.5;
    c *= 1.0 + su.misc.w * n * 2.0;
    // Letterbox.
    let bar = 0.11 * clamp(su.misc.y, 0.0, 1.0);
    let edge = su.size.w * 1.5;
    let inside = smoothstep(bar, bar + edge, uv.y) * smoothstep(bar, bar + edge, 1.0 - uv.y);
    c *= select(inside, 1.0, bar <= 0.0);
    return vec4<f32>(sane(c), 1.0);
}
