// Stateless particles: every particle's state is a closed form of its burst record, its index and the record's
// age, so any frame draws the same whatever was drawn before. Mirrored on the CPU by `particles::particle_state`.
// (Prepended with hash.wgsl.)

const SPARK: u32 = 0u;
const EMBER: u32 = 1u;
const SMOKE: u32 = 2u;
const DEBRIS: u32 = 3u;
const NOTE: u32 = 4u;

const EXIT_DISSOLVE: u32 = 1u;
const EXIT_BURST: u32 = 2u;
const EXIT_SHRINK: u32 = 3u;
const EXIT_FLYOFF: u32 = 4u;
const EXIT_EMBER: u32 = 5u;

struct Cam {
    view_proj: mat4x4<f32>,
    // w: world units per pixel at distance 1.
    eye: vec4<f32>,
    // w: how many burst records.
    right: vec4<f32>,
    // w: seconds of video (mod 1000), for flicker.
    up: vec4<f32>,
}

struct Burst {
    // xyz: render-space origin; w: the record's age now.
    origin: vec4<f32>,
    // xyz: unit direction; w: cos of the cone's half angle.
    dir: vec4<f32>,
    // speed min, speed max, gravity (down), drag.
    motion: vec4<f32>,
    // size start, size end, stretch, bounce.
    look: vec4<f32>,
    // rgb: HDR colour at birth; w: emit_over.
    colour: vec4<f32>,
    // rgb: HDR colour at death; w: the record's life.
    colour_end: vec4<f32>,
    // x: the record's envelope now.
    envl: vec4<f32>,
    // seed, count, shape, exit.
    info: vec4<u32>,
    // x: the first particle's global index.
    first: vec4<u32>,
}

@group(0) @binding(0) var<uniform> cam: Cam;
@group(0) @binding(1) var<storage, read> bursts: array<Burst>;

struct State {
    pos: vec3<f32>,
    vel: vec3<f32>,
    // Share of the particle's life gone, 0..1.
    u: f32,
    alive: bool,
}

fn drift(d: f32, a: f32) -> f32 {
    if d < 1e-4 {
        return a;
    }
    return (1.0 - exp(-d * a)) / d;
}

fn fly(p0: vec3<f32>, v0: vec3<f32>, g: f32, d: f32, a: f32) -> vec3<f32> {
    return p0 + v0 * drift(d, a) + vec3<f32>(0.0, -0.5 * g * a * a, 0.0);
}

fn fly_vel(v0: vec3<f32>, g: f32, d: f32, a: f32) -> vec3<f32> {
    return v0 * exp(-d * a) + vec3<f32>(0.0, -g * a, 0.0);
}

fn particle(b: Burst, i: u32) -> State {
    var s: State;
    s.alive = false;
    let seed = b.info.x;
    let tb = b.colour.w * h3(seed, i, 0u);
    let life = b.colour_end.w;
    let age = b.origin.w - tb;
    let plife = max(life - tb, 0.0) * mix(0.45, 1.0, h3(seed, i, 4u));
    if age <= 0.0 || age >= plife {
        return s;
    }
    let d = b.dir.xyz;
    let helper = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(d.y) > 0.9);
    let t = normalize(cross(d, helper));
    let bt = cross(d, t);
    let c = mix(b.dir.w, 1.0, h3(seed, i, 1u));
    let sn = sqrt(max(1.0 - c * c, 0.0));
    let phi = 6.2831853 * h3(seed, i, 2u);
    // Biased fast, so a burst reads as a shell of streaks rather than a filled ball.
    let speed = mix(b.motion.x, b.motion.y, sqrt(h3(seed, i, 3u)));
    let v0 = (t * (cos(phi) * sn) + bt * (sin(phi) * sn) + d * c) * speed;
    let p0 = b.origin.xyz;
    let g = b.motion.z;
    let dr = b.motion.w;
    var pos = fly(p0, v0, g, dr, age);
    var vel = fly_vel(v0, g, dr, age);
    if pos.y < 0.0 && p0.y >= 0.0 {
        // The first time it reaches the canvas: y falls once past its peak, so there is one crossing.
        var lo = 0.0;
        var hi = age;
        for (var k = 0; k < 14; k++) {
            let mid = 0.5 * (lo + hi);
            if fly(p0, v0, g, dr, mid).y > 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let bounce = b.look.w;
        if bounce <= 0.0 {
            return s;
        }
        var pb = fly(p0, v0, g, dr, hi);
        pb.y = 0.0;
        var vb = fly_vel(v0, g, dr, hi);
        vb = vec3<f32>(vb.x * 0.7, -vb.y * bounce, vb.z * 0.7);
        let rest = age - hi;
        pos = fly(pb, vb, g, dr, rest);
        vel = fly_vel(vb, g, dr, rest);
        if pos.y < 0.0 {
            pos.y = 0.0;
            vel.y = 0.0;
        }
    }
    s.pos = pos;
    s.vel = vel;
    s.u = age / plife;
    s.alive = true;
    return s;
}

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // rgb: premultiplied HDR colour; a: coverage weight (over-blended shapes).
    @location(1) colour: vec4<f32>,
    // x: length / width of a streak; y: a per-particle random; z: the particle's life share.
    @location(2) extra: vec3<f32>,
    @location(3) @interpolate(flat) shape: u32,
}

fn find_burst(ii: u32) -> u32 {
    var lo = 0u;
    var hi = u32(cam.right.w);
    // The last record whose first particle is at or before `ii`.
    while hi - lo > 1u {
        let mid = (lo + hi) / 2u;
        if bursts[mid].first.x <= ii {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    return lo;
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VOut {
    var o: VOut;
    o.clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    let b = bursts[find_burst(ii)];
    let i = ii - b.first.x;
    let seed = b.info.x;
    let shape = b.info.z;
    let exit = b.info.w;
    let env = b.envl.x;
    var s = particle(b, i);
    if !s.alive || env <= 0.0 {
        return o;
    }
    if exit == EXIT_DISSOLVE && h3(seed, i, 7u) > env {
        return o;
    }
    if exit == EXIT_FLYOFF {
        let k = 1.0 - env;
        s.pos.y += 1.6 * k * k;
    }
    var size = mix(b.look.x, b.look.y, s.u);
    if exit == EXIT_SHRINK {
        size *= env;
    } else if exit == EXIT_BURST {
        size *= 1.0 + 1.5 * (1.0 - env);
    }
    // Bright at once, fading over the second half of its life.
    var intensity = env * smooth01(s.u / 0.03) * (1.0 - smooth01((s.u - 0.3) / 0.7));
    var col = mix(b.colour.rgb, b.colour_end.rgb, pow(s.u, 0.8));
    if exit == EXIT_EMBER {
        col = mix(vec3<f32>(0.45, 0.03, 0.0) * max(max(col.r, col.g), col.b) * 0.3, col, env);
    }
    if shape == EMBER {
        intensity *= 0.75 + 0.25 * sin(cam.up.w * 23.0 + h3(seed, i, 8u) * 40.0);
    }
    let to_eye = cam.eye.xyz - s.pos;
    let dist = max(length(to_eye), 0.1);
    // Never under 1.5 pixels across; a clamped one keeps its energy, so far sparks don't sparkle.
    let min_size = 1.5 * dist * cam.eye.w;
    if size < min_size {
        intensity *= (size / min_size) * (size / min_size);
        size = min_size;
    }
    let corner = vec2<f32>(f32(vi & 1u) * 2.0 - 1.0, f32(vi >> 1u) * 2.0 - 1.0);
    let speed = length(s.vel);
    var world: vec3<f32>;
    var aspect = 1.0;
    if b.look.z > 0.0 && speed > 1e-3 && (shape == SPARK || shape == DEBRIS) {
        let axis = s.vel / speed;
        var side = cross(axis, to_eye);
        if length(side) < 1e-4 {
            side = cam.right.xyz;
        }
        side = normalize(side);
        let len = size + speed * b.look.z;
        aspect = len / size;
        world = s.pos + axis * (corner.x * len * 0.5) + side * (corner.y * size * 0.5);
    } else {
        let ang = 6.2831853 * h3(seed, i, 5u) + s.u * (h3(seed, i, 6u) - 0.5) * 6.0;
        let r = vec2<f32>(corner.x * cos(ang) - corner.y * sin(ang), corner.x * sin(ang) + corner.y * cos(ang));
        world = s.pos + (cam.right.xyz * r.x + cam.up.xyz * r.y) * (size * 0.5);
    }
    o.clip = cam.view_proj * vec4<f32>(world, 1.0);
    o.uv = corner;
    o.colour = vec4<f32>(col * intensity, intensity);
    o.extra = vec3<f32>(aspect, h3(seed, i, 9u), s.u);
    o.shape = shape;
    return o;
}

// An eighth note in uv (-1..1): a tilted oval head, a stem and a flag.
fn note_sdf(p: vec2<f32>) -> f32 {
    let q = p - vec2<f32>(-0.22, -0.52);
    let rq = vec2<f32>(q.x * 0.87 + q.y * 0.5, -q.x * 0.5 + q.y * 0.87);
    let head = length(rq / vec2<f32>(0.34, 0.24)) - 1.0;
    let stem = max(abs(p.x - 0.08) - 0.05, abs(p.y - 0.1) - 0.62);
    let fq = p - vec2<f32>(0.3, 0.5);
    let flag = max(abs(length(fq) - 0.25) - 0.06, -fq.x);
    return min(min(head * 0.24, stem), flag);
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    // Derivatives only in uniform control flow: both edges are measured before choosing the shape.
    let dn = note_sdf(uv * 1.1);
    let dd = abs(uv.x) * 0.8 + abs(uv.y) * 1.25 - 1.0;
    let aan = max(fwidth(dn), 1e-3);
    let aad = max(fwidth(dd), 1e-3);
    switch in.shape {
        case SPARK: {
            // A soft capsule along the streak, with a white-hot core.
            let a = in.extra.x;
            let q = vec2<f32>(max(abs(uv.x * a) - (a - 1.0), 0.0), uv.y);
            let d2 = dot(q, q);
            let soft = exp(-d2 * 3.5) * (1.0 - smoothstep(0.8, 1.0, sqrt(d2)));
            let core = exp(-d2 * 14.0);
            let lum = max(max(in.colour.r, in.colour.g), in.colour.b);
            return vec4<f32>(in.colour.rgb * soft + vec3<f32>(lum * 0.35 * core), 0.0);
        }
        case EMBER: {
            let r2 = dot(uv, uv);
            let glow = exp(-r2 * 4.0) * (1.0 - smoothstep(0.7, 1.0, r2));
            return vec4<f32>(in.colour.rgb * glow, 0.0);
        }
        case NOTE: {
            let fill = 1.0 - smoothstep(-aan, aan, dn);
            let halo = exp(-max(dn, 0.0) * 12.0) * 0.35;
            return vec4<f32>(in.colour.rgb * (fill + halo), 0.0);
        }
        case SMOKE: {
            // A billow: a soft disc eaten into by noise, premultiplied.
            let r = length(uv);
            let n = fbm(uv * 1.6 + vec2<f32>(in.extra.y * 37.0, in.extra.z * 1.5), u32(in.extra.y * 65536.0));
            let a = (1.0 - smooth01(r)) * smooth01(n * 1.8 - 0.35) * in.colour.a * 0.8;
            return vec4<f32>(in.colour.rgb / max(in.colour.a, 1e-4) * a, a);
        }
        default: {
            // Debris: a shard (a diamond), solid with an anti-aliased edge.
            let a = (1.0 - smoothstep(-aad, aad, dd)) * min(in.colour.a, 1.0);
            return vec4<f32>(in.colour.rgb / max(in.colour.a, 1e-4) * a, a);
        }
    }
}
