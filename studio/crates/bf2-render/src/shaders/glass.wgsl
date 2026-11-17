// The glass pass: transparent shells, drawn back to front with premultiplied alpha, depth tested but not
// written. Each instance draws its far side (back faces) first, then its near side. (Prepended with
// common.wgsl.)
//
// Face on, glass is a faint tint; toward the silhouette, Schlick's fresnel makes it more opaque and reflective,
// which outlines the ball. Highlights from the spot and the point lights are added on top (premultiplied, so they
// brighten whatever is behind).

const GLASS_SHININESS: f32 = 160.0;

@fragment
fn fs_glass(in: VOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let i = instances[in.inst];
    let tint = i.mat_a.rgb * in.rgb.rgb;
    let opacity = i.mat_a.w;
    let fresnel_k = i.mat_b.x;
    var n = normalize(in.nrm);
    let v = normalize(frame.eye.xyz - in.world);
    if !front {
        n = -n;
    }
    let cos_v = saturate(abs(dot(n, v)));
    let schlick = pow(1.0 - cos_v, 5.0);
    // The far side shows through the near side, so it counts for less.
    let side = select(0.5, 1.0, front);
    let alpha = saturate((opacity + (1.0 - opacity) * fresnel_k * schlick) * side * in.rgb.a);

    var light = frame.ambient.rgb;
    var spec = vec3<f32>(0.0);
    let reach = spot_reach(in.world, n);
    if reach > 0.0 {
        let l = normalize(frame.spot_pos.xyz - in.world);
        light += frame.spot_col.rgb * max(dot(n, l), 0.0) * reach;
        let h = normalize(l + v);
        spec += frame.spot_col.rgb * pow(max(dot(n, h), 0.0), GLASS_SHININESS) * reach;
    }
    for (var k = 0u; k < frame.info.x; k++) {
        let lg = lights[k];
        let r = point_reach(lg, in.world);
        if r <= 0.0 {
            continue;
        }
        let l = normalize(lg.pos_range.xyz - in.world);
        light += lg.colour.rgb * max(dot(n, l), 0.0) * r;
        let h = normalize(l + v);
        spec += lg.colour.rgb * pow(max(dot(n, h), 0.0), GLASS_SHININESS) * r;
    }
    // The tint glows a little on its own, so glass never reads as a hole in a dark arena.
    let body = tint * (0.5 + light);
    return vec4<f32>(body * alpha + spec * side, alpha);
}
