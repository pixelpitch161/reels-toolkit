// Decals: procedural marks on the canvas (flat quads just above it) and on the ropes (sleeves around them),
// premultiplied over the scene so they darken, with emissive parts (a scorch's hot rim, glowing cracks) added on
// top. (Prepended with hash.wgsl.)

const SCORCH: u32 = 0u;
const CRACK: u32 = 1u;
const SPLAT: u32 = 2u;
const GLOW: u32 = 3u;
const EXIT_EMBER: u32 = 5u;

struct Cam {
    view_proj: mat4x4<f32>,
    // x: output pixels per art pixel (1: no pixel look).
    art: vec4<f32>,
}

struct Dec {
    // xyz: centre (render space); w: radius.
    centre: vec4<f32>,
    // rgb: HDR colour; w: kind.
    colour: vec4<f32>,
    // xyz: a rope sleeve's axis (unit); w: the sleeve's radius.
    axis: vec4<f32>,
    // x: life share; y: seed; z: exit; w: envelope.
    extra: vec4<f32>,
}

@group(0) @binding(0) var<uniform> cam: Cam;
@group(0) @binding(1) var<storage, read> decs: array<Dec>;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    // -1..1 across the decal.
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) inst: u32,
}

@vertex
fn vs_canvas(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VOut {
    let d = decs[ii];
    let c = vec2<f32>(f32(vi & 1u) * 2.0 - 1.0, f32(vi >> 1u) * 2.0 - 1.0);
    let p = d.centre.xyz + vec3<f32>(c.x, 0.0, c.y) * d.centre.w;
    var o: VOut;
    o.clip = cam.view_proj * vec4<f32>(p, 1.0);
    o.uv = c;
    o.inst = ii;
    return o;
}

// A unit cylinder (radius 1, y from -1 to 1) wrapped round a rope.
@vertex
fn vs_rope(@location(0) pos: vec3<f32>, @builtin(instance_index) ii: u32) -> VOut {
    let d = decs[ii];
    let ax = d.axis.xyz;
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let side = normalize(cross(ax, up));
    let p = d.centre.xyz + ax * (pos.y * d.centre.w) + (side * pos.x + up * pos.z) * d.axis.w;
    var o: VOut;
    o.clip = cam.view_proj * vec4<f32>(p, 1.0);
    // Along the rope, and round it (a mark wraps the front half most).
    o.uv = vec2<f32>(pos.y, atan2(pos.z, pos.x) / 3.14159265);
    o.inst = ii;
    return o;
}

// Cellular edges: distance to the nearest border between hashed cells (0 on a crack).
fn cell_edge(p: vec2<f32>, seed: u32) -> f32 {
    let i = floor(p);
    var f1 = 9.0;
    var f2 = 9.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let c = i + vec2<f32>(f32(x), f32(y));
            let ci = vec2<u32>(vec2<i32>(c) + vec2<i32>(4096));
            let o = vec2<f32>(h3(seed, ci.x * 7919u + ci.y, 1u), h3(seed, ci.x * 7919u + ci.y, 2u));
            let d = length(c + o - p);
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    return f2 - f1;
}

// The pixel look: the same marks with hard edges, flat tones, a hard hot rim, and the fade dithered on the art
// grid, as the current engine's canvas marks are drawn.
fn fs_pixel(in: VOut) -> vec4<f32> {
    let d = decs[in.inst];
    let kind = u32(d.colour.w);
    let seed = u32(d.extra.y);
    let u = d.extra.x;
    let art = cam.art.x;
    let p = in.clip.xy;
    let uv = in.uv;
    let r = length(uv);
    let ragged = r + (fbm(uv * 3.0, seed) - 0.5) * 0.35;
    var alpha = 0.0;
    var body = vec3<f32>(0.0);
    var emit = vec3<f32>(0.0);
    var heat = 1.0 - smooth01(u * 4.0);
    if u32(d.extra.z) == EXIT_EMBER {
        heat = max(heat, 0.25 * d.extra.w);
    }
    let hot = mix(vec3<f32>(0.6, 0.05, 0.0), vec3<f32>(4.0, 1.6, 0.4), heat * heat) * heat;
    switch kind {
        case SCORCH: {
            let mask = select(0.0, 1.0, ragged < 0.75);
            alpha = mask * (0.6 + 0.3 * art_steps(fbm(uv * 6.0 + 3.0, seed + 7u), 2.0, p, art));
            body = d.colour.rgb * alpha;
            emit = hot * select(0.0, 1.5, abs(ragged - 0.72) < 0.07);
        }
        case CRACK: {
            let e = cell_edge(uv * 3.2, seed);
            let ang = atan2(uv.y, uv.x);
            let spokes = abs(fract(ang / 6.2831853 * 7.0 + h3(seed, 1u, 3u) + (fbm(uv * 4.0, seed + 3u) - 0.5) * 0.25) - 0.5);
            let line = select(0.0, 1.0, e < 0.05 || (spokes < 0.025 && r < 0.75));
            let mask = select(0.0, 1.0, ragged < 0.8);
            alpha = line * mask * 0.9 + mask * 0.15;
            body = d.colour.rgb * alpha;
            emit = hot * line * mask;
        }
        case SPLAT: {
            let n = fbm(uv * 2.5, seed);
            alpha = select(0.0, 0.9, ragged + (0.5 - n) * 0.6 < 0.52);
            body = d.colour.rgb * alpha;
        }
        default: {
            emit = d.colour.rgb * art_steps(exp(-r * r * 3.0) * select(0.0, 1.0, r < 0.9), 3.0, p, art);
        }
    }
    let on = art_on(d.extra.w, p, art);
    return vec4<f32>((body + emit) * on, alpha * on);
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    if cam.art.x > 1.0 {
        return fs_pixel(in);
    }
    let d = decs[in.inst];
    let kind = u32(d.colour.w);
    let seed = u32(d.extra.y);
    let u = d.extra.x;
    let env = d.extra.w;
    let uv = in.uv;
    let r = length(uv);
    let ragged = r + (fbm(uv * 3.0, seed) - 0.5) * 0.35;
    var alpha = 0.0;
    var body = vec3<f32>(0.0);
    var emit = vec3<f32>(0.0);
    // A scorch's rim and glowing cracks start hot and cool.
    var heat = 1.0 - smooth01(u * 4.0);
    if u32(d.extra.z) == EXIT_EMBER {
        heat = max(heat, 0.25 * env);
    }
    let hot = mix(vec3<f32>(0.6, 0.05, 0.0), vec3<f32>(4.0, 1.6, 0.4), heat * heat) * heat;
    switch kind {
        case SCORCH: {
            let mask = 1.0 - smoothstep(0.55, 0.95, ragged);
            alpha = mask * (0.72 + 0.28 * fbm(uv * 6.0 + 3.0, seed + 7u));
            body = d.colour.rgb * alpha;
            let rim = exp(-pow((ragged - 0.72) / 0.12, 2.0));
            emit = hot * rim * 1.5;
        }
        case CRACK: {
            // Cells for the web, plus a few long radial cracks.
            let e = cell_edge(uv * 3.2, seed);
            let ang = atan2(uv.y, uv.x);
            let spokes = abs(fract(ang / 6.2831853 * 7.0 + h3(seed, 1u, 3u) + (fbm(uv * 4.0, seed + 3u) - 0.5) * 0.25) - 0.5);
            let line = max(1.0 - smoothstep(0.0, 0.07, e), (1.0 - smoothstep(0.0, 0.035, spokes)) * (1.0 - smoothstep(0.2, 1.0, r)));
            let mask = 1.0 - smoothstep(0.6, 1.0, ragged);
            alpha = line * mask * 0.9 + mask * 0.15;
            body = d.colour.rgb * alpha;
            emit = hot * line * mask;
        }
        case SPLAT: {
            let n = fbm(uv * 2.5, seed);
            let mask = 1.0 - smoothstep(0.45, 0.6, ragged + (0.5 - n) * 0.6);
            alpha = mask * 0.9;
            body = d.colour.rgb * alpha;
        }
        default: {
            // Glow: light on the surface, nothing covered.
            let g = exp(-r * r * 3.0) * (1.0 - smoothstep(0.8, 1.0, r));
            emit = d.colour.rgb * g;
        }
    }
    return vec4<f32>((body + emit) * env, alpha * env);
}
