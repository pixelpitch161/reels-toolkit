// Shared by the effect shaders: the PCG hash every random choice comes from. Mirrored by `hash.rs`.

fn pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

// A number in [0, 1) from a seed, an index and a channel.
fn h3(seed: u32, i: u32, k: u32) -> f32 {
    return f32(pcg(seed ^ pcg(i * 16u + k)) >> 8u) / 16777216.0;
}

fn smooth01(x: f32) -> f32 {
    let y = clamp(x, 0.0, 1.0);
    return y * y * (3.0 - 2.0 * y);
}

// 2D value noise on a lattice hashed from `seed`, 0..1.
fn vnoise(p: vec2<f32>, seed: u32) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let ix = vec2<u32>(vec2<i32>(i) + vec2<i32>(4096));
    let a = h3(seed, ix.x * 7919u + ix.y, 0u);
    let b = h3(seed, (ix.x + 1u) * 7919u + ix.y, 0u);
    let c = h3(seed, ix.x * 7919u + ix.y + 1u, 0u);
    let d = h3(seed, (ix.x + 1u) * 7919u + ix.y + 1u, 0u);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Four octaves of value noise, 0..1.
fn fbm(p: vec2<f32>, seed: u32) -> f32 {
    var s = 0.0;
    var amp = 0.5;
    var q = p;
    for (var o = 0u; o < 4u; o++) {
        s += amp * vnoise(q, seed + o * 101u);
        q = q * 2.03 + vec2<f32>(17.1, 9.7);
        amp *= 0.5;
    }
    return s / 0.9375;
}
