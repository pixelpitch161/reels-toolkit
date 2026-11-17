// Mesh effects: shock rings and domes, emissive HDR, additive, depth tested. (Prepended with hash.wgsl.)

struct Cam {
    view_proj: mat4x4<f32>,
    // w: output pixels per art pixel (1: no pixel look).
    eye: vec4<f32>,
}

struct Inst {
    // xyz: centre (render space); w: radius now.
    centre: vec4<f32>,
    // rgb: HDR colour times the envelope; w: a ring's thickness.
    colour: vec4<f32>,
    // xyz: a ring's plane normal; w: a dome's fresnel.
    axis: vec4<f32>,
    // x: life share; y: seed (as a float); z: kind (0 ring, 1 dome).
    extra: vec4<f32>,
}

@group(0) @binding(0) var<uniform> cam: Cam;
@group(0) @binding(1) var<storage, read> insts: array<Inst>;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec3<f32>,
    // Ring: x across 0 (inner) .. 1 (outer), y the angle. Dome: xyz the normal.
    @location(1) a: vec3<f32>,
    @location(2) world: vec3<f32>,
    @location(3) @interpolate(flat) inst: u32,
}

// A ring vertex: (cos, sin) of its angle and which edge (0 inner, 1 outer).
@vertex
fn vs_ring(@location(0) v: vec3<f32>, @builtin(instance_index) ii: u32) -> VOut {
    let i = insts[ii];
    let n = normalize(i.axis.xyz);
    let helper = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(n.x) > 0.9);
    let u = normalize(cross(n, helper));
    let w = cross(n, u);
    let r = max(i.centre.w + (v.z - 0.5) * i.colour.w, 0.0);
    let p = i.centre.xyz + (u * v.x + w * v.y) * r;
    var o: VOut;
    o.clip = cam.view_proj * vec4<f32>(p, 1.0);
    o.colour = i.colour.rgb;
    o.a = vec3<f32>(v.z, atan2(v.y, v.x), 0.0);
    o.world = p;
    o.inst = ii;
    return o;
}

@fragment
fn fs_ring(in: VOut) -> @location(0) vec4<f32> {
    let i = insts[in.inst];
    let s = in.a.x;
    // A shock front: bright at the outer edge, a soft wake inside, broken up along the circle.
    let edge = exp(-pow((1.0 - s) / 0.18, 2.0));
    let wake = 0.35 * smoothstep(0.0, 0.8, s) * (1.0 - smoothstep(0.9, 1.0, s));
    let ends = smoothstep(0.0, 0.15, s) * (1.0 - smoothstep(0.96, 1.0, s));
    let seed = u32(i.extra.y);
    let br = 0.7 + 0.3 * vnoise(vec2<f32>(in.a.y * 5.0 + 20.0, i.extra.x * 4.0), seed);
    if cam.eye.w > 1.0 {
        // the pixel look: the front a hard band, the wake in dithered thirds
        let k = select(0.0, 1.0, s > 0.8) + art_steps(wake, 3.0, in.clip.xy, cam.eye.w);
        return vec4<f32>(in.colour * k * select(0.0, 1.0, ends > 0.5), 0.0);
    }
    return vec4<f32>(in.colour * (edge + wake) * ends * br, 0.0);
}

@vertex
fn vs_dome(@location(0) pos: vec3<f32>, @location(1) nrm: vec3<f32>, @builtin(instance_index) ii: u32) -> VOut {
    let i = insts[ii];
    let p = i.centre.xyz + pos * i.centre.w;
    var o: VOut;
    o.clip = cam.view_proj * vec4<f32>(p, 1.0);
    o.colour = i.colour.rgb;
    o.a = nrm;
    o.world = p;
    o.inst = ii;
    return o;
}

@fragment
fn fs_dome(in: VOut) -> @location(0) vec4<f32> {
    let i = insts[in.inst];
    let n = normalize(in.a);
    let v = normalize(cam.eye.xyz - in.world);
    let rim = pow(1.0 - abs(dot(n, v)), 2.5);
    let seed = u32(i.extra.y);
    // Energy bands drifting over the shell.
    let bands = 0.75 + 0.25 * vnoise(vec2<f32>(atan2(n.z, n.x) * 3.0, n.y * 6.0 - i.extra.x * 8.0), seed);
    var k = mix(1.0, rim, clamp(i.axis.w, 0.0, 1.0)) * bands;
    if cam.eye.w > 1.0 {
        // the pixel look: the shell's light in dithered thirds
        k = art_steps(k, 3.0, in.clip.xy, cam.eye.w);
    }
    // Both sides add, so each counts half.
    return vec4<f32>(in.colour * k * 0.5, 0.0);
}
