// The glass pass: transparent shells, drawn back to front with premultiplied alpha, depth tested but not
// written. Each instance draws its far side (back faces) first, then its near side. (Prepended with
// common.wgsl.)
//
// Face on, glass is a faint tint; toward the silhouette, Schlick's fresnel makes it more opaque and reflective,
// which outlines the ball. Highlights from the spot and the point lights are added on top (premultiplied, so they
// brighten whatever is behind).

const GLASS_SHININESS: f32 = 160.0;
// sqrt(1 - 0.91^2): where a sphere's outer 9 % (the current engine's 2 px rim on a ball) begins.
const RIM_COS: f32 = 0.415;

@fragment
fn fs_glass(in: VOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let px = px_at(in);
    let maps = read_maps(in);
    let i = instances[in.inst];
    // Additive (`mat_b.w` 1): its colour added to what is behind, nothing covered.
    if i.mat_b.w > 0.5 {
        return vec4<f32>(in.rgb.rgb * i.mat_a.x * in.rgb.a, 0.0);
    }
    let tint = i.mat_a.rgb * in.rgb.rgb;
    let opacity = i.mat_a.w;
    let fresnel_k = i.mat_b.x;
    var in2 = in;
    in2.world = px.world;
    in2.nrm = px.nrm;
    var n = perturb(normalize(in2.nrm), in.tan, maps.normal, i.mat_b.z);
    let v = normalize(frame.eye.xyz - in2.world);
    if !front && i.mat_c.w >= 0.0 {
        return back_fill(in2, i, n, v, px.cell);
    }
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
        light += frame.spot_col.rgb * gq(max(dot(n, l), 0.0) * reach, px.cell);
        let h = normalize(l + v);
        spec += frame.spot_col.rgb * gs(pow(max(dot(n, h), 0.0), GLASS_SHININESS) * reach);
    }
    for (var k = 0u; k < frame.info.x; k++) {
        let lg = lights[k];
        let r = point_reach(lg, in.world);
        if r <= 0.0 {
            continue;
        }
        let l = normalize(lg.pos_range.xyz - in.world);
        light += lg.colour.rgb * gq(max(dot(n, l), 0.0) * r, px.cell);
        let h = normalize(l + v);
        spec += lg.colour.rgb * gs(pow(max(dot(n, h), 0.0), GLASS_SHININESS) * r);
    }
    // The tint glows a little on its own, so glass never reads as a hole in a dark arena.
    let body = tint * (0.5 + light);
    return vec4<f32>(body * alpha + spec * side, alpha);
}

// The far side as a fill: its own tint and opacity, lit as the near side at the same place on the screen would be
// (the outward normal mirrored toward the camera), the spot's light cut into four bands as the current engine's
// ball fill is.
fn back_fill(in: VOut, i: InstanceG, n_out: vec3<f32>, v: vec3<f32>, cell: vec2<i32>) -> vec4<f32> {
    // The outer 9 % of the disc the ball covers on the screen (|n.v| under 0.42 on a sphere) is the current
    // engine's rim band: the glass's tint, solid.
    if abs(dot(n_out, v)) < RIM_COS {
        return vec4<f32>(i.mat_a.rgb, 1.0);
    }
    let n = normalize(n_out - 2.0 * dot(n_out, v) * v);
    var light = frame.ambient.rgb;
    let reach = spot_reach(in.world, n);
    if reach > 0.0 {
        let l = normalize(frame.spot_pos.xyz - in.world);
        if PX_MODE == 0u {
            light += frame.spot_col.rgb * ceil(saturate(max(dot(n, l), 0.0) * reach) * 4.0 - 0.001) / 4.0;
        } else {
            light += frame.spot_col.rgb * px_steps(max(dot(n, l), 0.0) * reach, 4.0, cell);
        }
    }
    for (var k = 0u; k < frame.info.x; k++) {
        let lg = lights[k];
        let r = point_reach(lg, in.world);
        if r <= 0.0 {
            continue;
        }
        let l = normalize(lg.pos_range.xyz - in.world);
        light += lg.colour.rgb * gq(max(dot(n, l), 0.0) * r, cell);
    }
    let alpha = saturate(i.mat_c.w * in.rgb.a);
    return vec4<f32>(i.mat_c.rgb * light * alpha, alpha);
}

// The pixel spike: glass light in four dithered steps, its highlight a hard dot.
fn gq(x: f32, cell: vec2<i32>) -> f32 {
    if PX_MODE == 0u {
        return x;
    }
    return px_steps(x, 4.0, cell);
}

fn gs(x: f32) -> f32 {
    if PX_MODE == 0u {
        return x;
    }
    return step(0.5, x) * 1.5;
}
