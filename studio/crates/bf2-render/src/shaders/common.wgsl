// Shared by every scene shader (shadow, lit, glass): the frame and instance data, the vertex stage and the
// lighting terms. Each pipeline prepends this file to its own; the layouts mirror `renderer.rs`.

const KIND_TOON: u32 = 0u;
const KIND_GLASS: u32 = 1u;
const KIND_EMISSIVE: u32 = 2u;
const KIND_UNLIT: u32 = 3u;

const FLAG_EMIT: u32 = 1u;

struct Frame {
    view_proj: mat4x4<f32>,
    spot_vp: mat4x4<f32>,
    eye: vec4<f32>,
    ambient: vec4<f32>,
    // xyz: position; w: 1 when there is a spot.
    spot_pos: vec4<f32>,
    // xyz: unit direction; w: cos of the cone's half angle.
    spot_dir: vec4<f32>,
    // rgb: HDR colour; w: cos of the inner (full strength) angle.
    spot_col: vec4<f32>,
    // x: point lights; y: 1 when the spot casts a shadow; z: shadow map size in texels.
    info: vec4<u32>,
}

struct LightG {
    // xyz: position; w: range.
    pos_range: vec4<f32>,
    colour: vec4<f32>,
}

struct InstanceG {
    model: mat4x4<f32>,
    nrm0: vec4<f32>,
    nrm1: vec4<f32>,
    nrm2: vec4<f32>,
    colour: vec4<f32>,
    // Toon: rim rgb, rim power. Glass: tint rgb, opacity. Emissive: strength.
    mat_a: vec4<f32>,
    // x: glass fresnel; y: the maps' uv scale; z: the normal map's strength.
    mat_b: vec4<f32>,
    // Glass: the far side's tint and opacity (w < 0: none).
    mat_c: vec4<f32>,
    // x: kind; y: toon bands; z: albedo map slot; w: normal map slot (0: none).
    kind: vec4<u32>,
}

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read> lights: array<LightG>;
@group(0) @binding(2) var shadow_map: texture_depth_2d;
@group(0) @binding(3) var shadow_cmp: sampler_comparison;
@group(0) @binding(4) var albedo_maps: texture_2d_array<f32>;
@group(0) @binding(5) var normal_maps: texture_2d_array<f32>;
@group(0) @binding(6) var map_sampler: sampler;
@group(1) @binding(0) var<storage, read> instances: array<InstanceG>;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) rgb: vec3<f32>,
    @location(3) flags: u32,
    @location(4) spec: f32,
    @location(5) uv: vec2<f32>,
    @location(6) tan: vec4<f32>,
}

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) rgb: vec4<f32>,
    @location(3) spec: f32,
    @location(4) @interpolate(flat) flags: u32,
    @location(5) @interpolate(flat) inst: u32,
    @location(6) uv: vec2<f32>,
    @location(7) tan: vec4<f32>,
}

@vertex
fn vs_main(v: VIn, @builtin(instance_index) inst: u32) -> VOut {
    let i = instances[inst];
    let world = i.model * vec4<f32>(v.pos, 1.0);
    let nm = mat3x3<f32>(i.nrm0.xyz, i.nrm1.xyz, i.nrm2.xyz);
    var o: VOut;
    o.clip = frame.view_proj * world;
    o.world = world.xyz;
    o.nrm = nm * v.nrm;
    o.rgb = vec4<f32>(v.rgb * i.colour.rgb, i.colour.a);
    o.spec = v.spec;
    o.flags = v.flags;
    o.inst = inst;
    o.uv = v.uv;
    // The tangent turns with the model (it lies in the surface), its handedness kept.
    o.tan = vec4<f32>((i.model * vec4<f32>(v.tan.xyz, 0.0)).xyz, v.tan.w);
    return o;
}

// The instance's maps, read in uniform control flow (every fragment samples, the slot picks): the albedo
// (white with no map) and the tangent-space normal (+Z with no map).
struct Maps {
    albedo: vec3<f32>,
    normal: vec3<f32>,
}

fn read_maps(in: VOut) -> Maps {
    let i = instances[in.inst];
    // the albedo map covers the mesh's uvs once; the normal map (a fine finish) repeats `uv_scale` times
    let uv = in.uv * i.mat_b.y;
    let a = textureSample(albedo_maps, map_sampler, in.uv, i32(max(i.kind.z, 1u) - 1u)).rgb;
    let n = textureSample(normal_maps, map_sampler, uv, i32(max(i.kind.w, 1u) - 1u)).xyz * 2.0 - 1.0;
    var m: Maps;
    m.albedo = select(vec3<f32>(1.0), a, i.kind.z > 0u);
    m.normal = select(vec3<f32>(0.0, 0.0, 1.0), n, i.kind.w > 0u);
    return m;
}

// The shading normal: the geometric normal `n` (unit) tilted by the map's tangent-space normal `m`, `k` of the
// way, in the frame of the interpolated tangent.
fn perturb(n: vec3<f32>, tan: vec4<f32>, m: vec3<f32>, k: f32) -> vec3<f32> {
    let t0 = tan.xyz - n * dot(n, tan.xyz);
    if dot(t0, t0) < 1e-10 || k <= 0.0 {
        return n;
    }
    let t = normalize(t0);
    let b = cross(n, t) * tan.w;
    return normalize(t * (m.x * k) + b * (m.y * k) + n * m.z);
}

// The spot's shadow at `world` (1: lit, 0: shadowed): 3 x 3 PCF on hardware 2 x 2 compares, with the sample
// point pushed out along the normal against acne.
fn spot_shadow(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if frame.info.y == 0u {
        return 1.0;
    }
    let texel = 1.0 / f32(frame.info.z);
    let p = frame.spot_vp * vec4<f32>(world + n * 0.03, 1.0);
    if p.w <= 0.0 {
        return 1.0;
    }
    let ndc = p.xyz / p.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || ndc.z > 1.0 {
        return 1.0;
    }
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let o = vec2<f32>(f32(x), f32(y)) * texel;
            lit += textureSampleCompareLevel(shadow_map, shadow_cmp, uv + o, ndc.z);
        }
    }
    return lit / 9.0;
}

// How much of the spot reaches `world` before the surface's angle: the cone with a soft edge, and the shadow.
fn spot_reach(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if frame.spot_pos.w == 0.0 {
        return 0.0;
    }
    let to_p = normalize(world - frame.spot_pos.xyz);
    let cone = smoothstep(frame.spot_dir.w, frame.spot_col.w, dot(to_p, frame.spot_dir.xyz));
    if cone <= 0.0 {
        return 0.0;
    }
    return cone * spot_shadow(world, n);
}

// A point light's fall-off: full at the light, a smooth window to zero at its range.
fn point_reach(l: LightG, world: vec3<f32>) -> f32 {
    let d = length(l.pos_range.xyz - world);
    let x = saturate(1.0 - (d * d) / (l.pos_range.w * l.pos_range.w));
    return x * x;
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}
