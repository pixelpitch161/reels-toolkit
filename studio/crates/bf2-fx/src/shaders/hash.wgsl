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

// The pixel look (E8b): effects drawn under the pixel pass (`FxCamera::art` above 1) are hard shapes whose fades
// are dithered on the art grid, as the current engine draws its effects (`floor(m * 3 + BAYER * 0.9) / 3`).

const BAYER4 = array<u32, 16>(0u, 8u, 2u, 10u, 12u, 4u, 14u, 6u, 3u, 11u, 1u, 9u, 15u, 7u, 13u, 5u);

// The current engine's Bayer value (0..15/16) at the art pixel holding the fragment at `p`, for art pixels `art`
// output pixels wide.
fn art_bayer(p: vec2<f32>, art: f32) -> f32 {
    let q = vec2<u32>(floor(floor(p) / max(art, 1.0)));
    return f32(BAYER4[(q.y % 4u) * 4u + q.x % 4u]) / 16.0;
}

// `v` (0..1) banded into `n` steps with the dither at the art pixel: the current engine's stepped glow.
fn art_steps(v: f32, n: f32, p: vec2<f32>, art: f32) -> f32 {
    return clamp(floor(v * n + art_bayer(p, art) * 0.9) / n, 0.0, 1.0);
}

// A fade `v` (0..1) as all or nothing at the art pixel: a dithered fade (screen door) on the art grid.
fn art_on(v: f32, p: vec2<f32>, art: f32) -> f32 {
    return select(0.0, 1.0, v > art_bayer(p, art) + 1.0 / 32.0);
}
