// The opaque pass: Toon, Emissive and Unlit. (Prepended with common.wgsl.)
//
// Toon is cel shading: each light's contribution (surface angle x fall-off x cone x shadow) is cut into `bands`
// hard steps before it is coloured, so pools of light and shadow edges read as flat bands. The specular is a
// hard-edged highlight scaled by the vertex `spec`, and the rim is a hard band where the surface turns away from
// the eye.

const SHININESS: f32 = 48.0;

// `x` (0..1) cut into `bands` steps: 0 stays 0, anything lit gets at least one band. No bands (0) is smooth
// shading, for the set (canvas, floor, stands): cel bands are for the models.
fn band(x: f32, bands: f32) -> f32 {
    if bands <= 0.0 {
        return saturate(x);
    }
    return ceil(saturate(x) * bands - 0.001) / bands;
}

// A hard specular step for one light.
fn spec_step(n: vec3<f32>, l: vec3<f32>, v: vec3<f32>) -> f32 {
    let h = normalize(l + v);
    return step(0.5, pow(max(dot(n, h), 0.0), SHININESS));
}

// The pixel spike's light cut: the toon bands (or PX_STEPS on the smooth set) with the ordered dither.
fn lband(x: f32, bands: f32, cell: vec2<i32>) -> f32 {
    if PX_MODE == 0u {
        return band(x, bands);
    }
    return px_steps(x, select(PX_STEPS, bands, bands > 0.0), cell);
}

fn toon(in0: VOut, i: InstanceG, maps: Maps, px: PxAt) -> vec3<f32> {
    var in = in0;
    in.world = px.world;
    in.nrm = px.nrm;
    let n = perturb(normalize(in.nrm), in.tan, maps.normal, i.mat_b.z);
    let v = normalize(frame.eye.xyz - in.world);
    let bands = f32(i.kind.y);
    let base = in.rgb.rgb * maps.albedo;
    var diffuse = frame.ambient.rgb;
    var spec = vec3<f32>(0.0);
    let reach = spot_reach(in.world, n);
    if reach > 0.0 {
        let l = normalize(frame.spot_pos.xyz - in.world);
        diffuse += frame.spot_col.rgb * lband(max(dot(n, l), 0.0) * reach, bands, px.cell);
        spec += frame.spot_col.rgb * spec_step(n, l, v) * step(0.5, reach);
    }
    for (var k = 0u; k < frame.info.x; k++) {
        let lg = lights[k];
        let r = point_reach(lg, in.world);
        if r <= 0.0 {
            continue;
        }
        let l = normalize(lg.pos_range.xyz - in.world);
        diffuse += lg.colour.rgb * lband(max(dot(n, l), 0.0) * r, bands, px.cell);
        spec += lg.colour.rgb * spec_step(n, l, v) * step(0.5, r);
    }
    let rim_amount = pow(1.0 - saturate(dot(n, v)), max(i.mat_a.w, 0.001));
    let rim = i.mat_a.rgb * step(0.5, rim_amount);
    return base * diffuse + spec * in.spec + rim;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let maps = read_maps(in);
    let px = px_at(in);
    let i = instances[in.inst];
    if (in.flags & FLAG_EMIT) != 0u {
        return vec4<f32>(in.rgb.rgb, 1.0);
    }
    switch i.kind.x {
        case KIND_EMISSIVE: {
            return vec4<f32>(in.rgb.rgb * i.mat_a.x, 1.0);
        }
        case KIND_UNLIT: {
            return vec4<f32>(in.rgb.rgb, 1.0);
        }
        default: {
            return vec4<f32>(toon(in, i, maps, px), 1.0);
        }
    }
}
