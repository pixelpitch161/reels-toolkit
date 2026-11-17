// Trails: additive HDR strips built on the CPU. (Prepended with hash.wgsl.)
//
// A smear is brightest at the weapon's edge (the tip side) with a hot core line there, and fades toward the base
// and the tail; a ribbon and lightning glow across their middle with a hot core.

struct Cam {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> cam: Cam;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) along: f32,
    @location(2) colour: vec3<f32>,
    @location(3) across: f32,
    @location(4) core: vec3<f32>,
    @location(5) style: f32,
}

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec3<f32>,
    @location(1) core: vec3<f32>,
    @location(2) along: f32,
    @location(3) across: f32,
    @location(4) @interpolate(flat) style: f32,
}

@vertex
fn vs_main(v: VIn) -> VOut {
    var o: VOut;
    o.clip = cam.view_proj * vec4<f32>(v.pos, 1.0);
    o.colour = v.colour;
    o.core = v.core;
    o.along = v.along;
    o.across = v.across;
    o.style = v.style;
    return o;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    // Full at the head, nothing at the tail.
    let tail = 1.0 - smooth01(in.along);
    let fade = tail * tail;
    var body: f32;
    var core: f32;
    if in.style == 1.0 {
        // Smear: a gradient from the base up to a bright edge at the tip.
        let x = in.across;
        body = pow(x, 1.6) * (1.0 - smoothstep(0.97, 1.0, x));
        let e = (1.0 - x) / 0.06;
        core = exp(-e * e) * (1.0 - smooth01(in.along * 2.5));
    } else {
        let x = abs(in.across * 2.0 - 1.0);
        body = exp(-x * x * 2.5) * (1.0 - smoothstep(0.85, 1.0, x));
        let e = x / 0.22;
        core = exp(-e * e);
    }
    return vec4<f32>((in.colour * body + in.core * core) * fade, 0.0);
}
