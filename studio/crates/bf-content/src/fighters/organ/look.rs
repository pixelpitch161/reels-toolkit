//! ORGAN's look: the port of `Look` in `projects/ballfight/kit/fighters/organ.py` and of the module's draw code
//! (#35's `styles.py`), hook by hook.
//!
//! What it draws: the tin pipe rows rising and sinking in sequence (hex prisms, a dark mouth, an ink outline, the
//! tops glowing wine as they speak), their shadows from the spot, the wine glow pools on the canvas, stone dust,
//! a dusty mark under every third pipe and a wine light along the row; a landed row's burst, light, dust and
//! crack; CATHEDRAL's dome (a stained-glass tint, golden ribs, echo rings rolling out on the canvas with a heat
//! shimmer, and its wine light from inside); TOCCATA's darkening target with a flashing rim, the forest of pipes,
//! the stomp ring, its shimmer, crack and scorch marks; the ball thumping on every row; the champion's solo as a
//! ring of pipes erupting round it; shards of tin pipe, dark case wood and gilt.
//!
//! Its VS card panel is the card's own (`_organ_pipes`, `bf_picture::title::patterns::organ`).
//!
//! The two `ConvexHull` call sites (a pipe's outline, the dome's silhouette) go through
//! `bf_picture::hull::convex_hull`. Where scipy raises (flat or too few points) the kit's `try / except` skips the
//! outline, or the whole dome: so does this. Where Qhull would merge, the hull module answers with its plain scan
//! and counts the fallback (ruling M3-R4): the look draws that answer, and the gates stop on the count.
//!
//! One thing of the kit's look has no way into the Studio: a solo note that is a chord symbol (`('dim', r)`).
//! The fight file's solos hold plain notes, which is the `else` branch of `m[1] if isinstance(m, tuple) else m`.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::rng::Generator;
use bf_core::roster::{AbilityKind, FighterCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{self, H, W};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::hull::{self, Unusual};
use bf_picture::img::Img;
use bf_picture::lighting::{Lights, shade};
use bf_picture::look::{CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use bf_picture::raster::Frame;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::shadows::proj_point;
use bf_picture::{np, pil};

/// `TIN`.
const TIN: V3 = [222.0, 228.0, 242.0];
/// `TIN_DK`.
const TIN_DK: V3 = [128.0, 132.0, 150.0];
/// `MOUTH`.
const MOUTH: V3 = [30.0, 22.0, 34.0];
/// `PIPE_DX`: the spacing along a row.
const PIPE_DX: f64 = 0.34;
/// `PIPE_R`.
const PIPE_R: f64 = 0.15;
/// `RISE`: seconds a pipe takes to shoot up.
const RISE: f64 = 0.07;
/// `SINK`: seconds a pipe takes to sink back.
const SINK: f64 = 0.12;
/// `GLASS_A`.
const GLASS_A: V3 = [120.0, 40.0, 120.0];
/// `GLASS_B`.
const GLASS_B: V3 = [60.0, 90.0, 200.0];
/// `GOLD`.
const GOLD: V3 = [255.0, 206.0, 110.0];
/// `DUST_COL`: stone dust with a hint of wine.
const DUST_COL: [V3; 4] = [
    [178.0, 160.0, 150.0],
    [150.0, 128.0, 126.0],
    [208.0, 190.0, 176.0],
    [176.0, 120.0, 136.0],
];
/// `CASE`: the dark case wood (shards).
const CASE: V3 = [92.0, 52.0, 40.0];

/// `np.array([math.cos(th), math.sin(th)])`.
fn unit(th: f64) -> [f64; 2] {
    [dmath::cos(th), dmath::sin(th)]
}

/// `np.dot(a, b)` as the plain sum (ruling R25).
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `Q.mean(0)`: the rows added in order, then divided by their number.
fn mean(rows: &[V3]) -> V3 {
    let mut s = [0.0; 3];
    for r in rows {
        s = [s[0] + r[0], s[1] + r[1], s[2] + r[2]];
    }
    let n = rows.len() as f64;
    [s[0] / n, s[1] / n, s[2] / n]
}

/// `np.linspace(a, b, n)`: `i * step + a`, the last value `b` itself.
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    let step = (b - a) / (n - 1) as f64;
    (0..n)
        .map(|i| if i == n - 1 { b } else { i as f64 * step + a })
        .collect()
}

/// A numpy boolean as a factor: `x * (a > b)`.
fn gate(on: bool) -> f64 {
    if on { 1.0 } else { 0.0 }
}

/// `1 - (1 - k) ** 2`.
fn ease_out(k: f64) -> f64 {
    1.0 - (1.0 - k) * (1.0 - k)
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// A number of the recording's events and ragged arrays.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// `int(x)` of a time that is never negative, as a seed.
fn seed_of(x: f64) -> u64 {
    np::f64_to_i64(x).max(0) as u64
}

// ---------------------------------------------------------------- #35 styles.py: organ pipes (line)
/// One pipe row, rebuilt from its `line` event (`self.lines`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Row {
    id: i64,
    owner: usize,
    t0: f64,
    p0: [f64; 2],
    u: [f64; 2],
    l: f64,
    speed: f64,
    linger: f64,
}

/// A pipe out of the floor: `(sx, sz, h, glow, owner, r)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pipe {
    sx: f64,
    sz: f64,
    h: f64,
    glow: f64,
    owner: usize,
    r: f64,
}

/// `pipe_height(x, L)`: a row ramps up toward the enemy (a crescendo of pipes), with a little organ-front rhythm
/// in it.
fn pipe_height(x: f64, l: f64) -> f64 {
    let u = x / py_max(l, 1e-6);
    0.95 + 0.95 * u + 0.14 * dmath::cos(x * 7.3)
}

/// `max(1, int(L / PIPE_DX))`: the pipes of a row of length `l`.
fn row_count(l: f64) -> i64 {
    np::f64_to_i64(l / PIPE_DX).max(1)
}

/// `row_pipes(line, t)`: `(sx, sz, height, glow)` of one row's pipes that are out of the floor at time `t`.
fn row_pipes(line: &Row, t: f64) -> Vec<(f64, f64, f64, f64)> {
    let mut out = Vec::new();
    let l = line.l;
    if l <= 0.05 {
        return out;
    }
    let n = row_count(l);
    for j in 0..n {
        let x = (j as f64 + 0.5) * l / n as f64;
        let te = line.t0 + x / line.speed;
        let u = t - te;
        if u < 0.0 {
            continue;
        }
        let up = py_min(1.0, u / RISE);
        let down = 1.0 - py_min(1.0, py_max(0.0, (u - line.linger) / SINK));
        let k = py_min(up, down);
        if k <= 0.0 {
            continue;
        }
        let k = ease_out(k);
        let p = [line.p0[0] + line.u[0] * x, line.p0[1] + line.u[1] * x];
        let h = pipe_height(x, l) * k;
        let glow = dmath::exp(-u / 0.18);
        out.push((p[0], p[1], h, glow));
    }
    out
}

/// What a face of [`pipe_faces`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Face {
    /// A side quad, with its number round the pipe.
    Side(usize),
    /// The top cap.
    Top,
}

/// `pipe_faces(cx, cz, h, r, n=6, y0=0.0)`: the side quads and the top cap of a hex pipe in world coordinates:
/// `(Q, normal, kind)`.
fn pipe_faces(cx: f64, cz: f64, h: f64, r: f64) -> Vec<(Vec<V3>, V3, Face)> {
    const N: usize = 6;
    // a = np.arange(n) * 2 * math.pi / n + math.pi / 6
    let ring: Vec<[f64; 2]> = (0..N)
        .map(|j| {
            let a = (j as f64 * 2.0) * dmath::PI / N as f64 + dmath::PI / 6.0;
            [cx + r * dmath::cos(a), cz + r * dmath::sin(a)]
        })
        .collect();
    let bot: Vec<V3> = ring.iter().map(|q| scene::to_world(q[0], q[1], 0.0)).collect();
    let top: Vec<V3> = ring.iter().map(|q| scene::to_world(q[0], q[1], 0.0 + h)).collect();
    let mut out = Vec::with_capacity(N + 1);
    for j in 0..N {
        let j2 = (j + 1) % N;
        let q = vec![bot[j], bot[j2], top[j2], top[j]];
        // mid = (ring[j] + ring[j2]) / 2 - np.array([cx, cz])
        let mid = [
            (ring[j][0] + ring[j2][0]) / 2.0 - cx,
            (ring[j][1] + ring[j2][1]) / 2.0 - cz,
        ];
        let nw = scene::to_world(mid[0], mid[1], 0.0);
        // nw = nw / (np.linalg.norm(nw) + 1e-9)
        let len = dot(nw, nw).sqrt() + 1e-9;
        out.push((q, [nw[0] / len, nw[1] / len, nw[2] / len], Face::Side(j)));
    }
    out.push((top, [0.0, 1.0, 0.0], Face::Top));
    out
}

/// `draw_pipe(fr, cam, L, shade, cx, cz, h, glow, col, r)`: one tin pipe; `col` is the owner's colour (the top
/// glows in it as the pipe speaks).
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_pipe(fr: &mut Frame, cam: &Cam, l: &Lights, cx: f64, cz: f64, h: f64, glow: f64, col: V3, r: f64) {
    if h < 0.03 {
        return;
    }
    for (q, nw, face) in pipe_faces(cx, cz, h, r) {
        match face {
            Face::Side(j) => {
                let ctr = mean(&q);
                // if np.dot(nw, cam.pos - Q.mean(0)) <= 0: continue
                let to_cam = [cam.pos[0] - ctr[0], cam.pos[1] - ctr[1], cam.pos[2] - ctr[2]];
                if dot(nw, to_cam) <= 0.0 {
                    continue;
                }
                let base = if j % 2 == 0 { TIN } else { TIN.map(|c| c * 0.92) };
                let c = shade(l, &[nw], &[ctr], &[base], None, 0.55, 0.0, 5.0)[0];
                fr.poly(cam, &q, c, true);
            }
            Face::Top => {
                // c = TIN_DK * (0.75 - 0.35 * glow) + np.array(col) * 0.75 * glow + 30 * glow
                let c: V3 = std::array::from_fn(|ch| {
                    np::clip(
                        TIN_DK[ch] * (0.75 - 0.35 * glow) + col[ch] * 0.75 * glow + 30.0 * glow,
                        0.0,
                        255.0,
                    )
                });
                fr.poly(cam, &q, c, true);
            }
        }
    }
    // the mouth: a dark slot a third of the way up, facing the camera
    let foot = scene::to_world(cx, cz, 0.0);
    let v = [cam.pos[0] - foot[0], cam.pos[1] - foot[1], cam.pos[2] - foot[2]];
    let ang = dmath::atan2(v[2], v[0]);
    let d = unit(ang);
    // back to the ring frame: world X = (sx - sz)/sqrt2, Z = (sx + sz)/sqrt2
    let root2 = 2.0_f64.sqrt();
    let (sx_d, sz_d) = ((d[0] + d[1]) / root2, (d[1] - d[0]) / root2);
    let m = [cx + sx_d * r * 1.01, cz + sz_d * r * 1.01];
    // t_ = np.array([-sz_d, sx_d]) * r * 0.45
    let t = [-sz_d * r * 0.45, sx_d * r * 0.45];
    let (ym, hm) = (py_min(0.32, h * 0.3), py_min(0.1, h * 0.12));
    let q = [
        scene::to_world(m[0] - t[0], m[1] - t[1], ym),
        scene::to_world(m[0] + t[0], m[1] + t[1], ym),
        scene::to_world(m[0] + t[0] * 0.6, m[1] + t[1] * 0.6, ym + hm),
        scene::to_world(m[0] - t[0] * 0.6, m[1] - t[1] * 0.6, ym + hm),
    ];
    fr.poly(cam, &q, MOUTH, true);
    // an ink outline round the pipe's silhouette, so a row reads as separate pipes
    // a = np.arange(6) * math.pi / 3 + math.pi / 6
    let ring: Vec<[f64; 2]> = (0..6)
        .map(|j| {
            let a = f64::from(j) * dmath::PI / 3.0 + dmath::PI / 6.0;
            [cx + r * dmath::cos(a), cz + r * dmath::sin(a)]
        })
        .collect();
    let mut world: Vec<V3> = ring.iter().map(|q| scene::to_world(q[0], q[1], 0.0)).collect();
    world.extend(ring.iter().map(|q| scene::to_world(q[0], q[1], h)));
    let sp = cam.wp(&world);
    // try: hull = sp[ConvexHull(sp).vertices]; fr.draw.polygon(hull, outline=(40, 30, 48))  except Exception: pass
    if let Some(outline) = hull_of(&sp) {
        pil::polygon_outline(&mut fr.pil, &outline, None, &[40, 30, 48]);
    }
}

/// `pts[ConvexHull(pts).vertices]`, or `None` where scipy raises (points on one line or in one place, fewer than
/// three, a coordinate that is not finite), which the kit's `try / except` turns into "draw nothing". A hull the
/// port cannot order as Qhull does (a merge) is returned as the hull module's plain scan and counted there.
fn hull_of(pts: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let found = hull::convex_hull(pts);
    match found.fallback {
        Some(Unusual::Flat | Unusual::NotEnoughPoints) => None,
        None | Some(Unusual::Merge) => Some(found.vertices.iter().map(|&i| pts[i]).collect()),
    }
}

/// `pipe_shadow_quad(cx, cz, h, r)`: a vertical card through the pipe (for the spot's shadow on the canvas).
fn pipe_shadow_quad(cx: f64, cz: f64, h: f64, r: f64) -> [V3; 4] {
    [
        scene::to_world(cx - r, cz + r, 0.0),
        scene::to_world(cx + r, cz - r, 0.0),
        scene::to_world(cx + r, cz - r, h),
        scene::to_world(cx - r, cz + r, h),
    ]
}

// ---------------------------------------------------------------- #35 styles.py: masks, the dome, the forest
/// `dome_points(cx, cz, rb, hgt, n_rib=8, m=9)`: the ribs of a pointed (gothic) dome: 8 arcs of 9 points from the
/// base ring to the apex.
fn dome_points(cx: f64, cz: f64, rb: f64, hgt: f64) -> Vec<Vec<V3>> {
    const N_RIB: usize = 8;
    const M: usize = 9;
    (0..N_RIB)
        .map(|j| {
            let a = 2.0 * dmath::PI * j as f64 / N_RIB as f64 + 0.2;
            (0..M)
                .map(|i| {
                    let v = i as f64 / (M - 1) as f64;
                    // a pointed arch: steep near the top
                    let r = rb * dmath::pow(1.0 - v, 0.7);
                    let y = hgt * dmath::pow(dmath::sin(v * dmath::PI / 2.0), 0.8);
                    scene::to_world(cx + r * dmath::cos(a), cz + r * dmath::sin(a), y)
                })
                .collect()
        })
        .collect()
}

/// `draw_dome(arr, cam, cx, cz, t, k, col, echo_t0)`: `k` is the strength, 0 to 1. A stained-glass tint inside
/// the silhouette and golden ribs. (The echo rings are on the canvas.)
fn draw_dome(arr: &mut Img<f32>, cam: &Cam, cx: f64, cz: f64, k: f64, col: V3) {
    let (rb, hgt) = (R * 1.55, R * 3.0);
    let ribs = dome_points(cx, cz, rb, hgt);
    let pts: Vec<(f64, f64)> = ribs.iter().flat_map(|r| cam.wp(r)).collect();
    // try: hull = pts[ConvexHull(pts).vertices]  except Exception: return
    let Some(outline) = hull_of(&pts) else { return };
    // mk = mask_of([hull])
    let mut mk: Img<u8> = Img::new(W, H, 1);
    if outline.len() >= 3 {
        pil::polygon(&mut mk, &outline, &[255]);
    }
    let y0 = outline.iter().map(|q| q.1).fold(f64::INFINITY, np::minimum);
    let y1 = outline.iter().map(|q| q.1).fold(f64::NEG_INFINITY, np::maximum);
    let span = py_max(1.0, y1 - y0);
    let bayer = config::bayer_full();
    let k32 = k as f32;
    for y in 0..H {
        // v = np.clip((yy - y0) / max(1.0, y1 - y0), 0, 1)
        let v = np::clip((y as f64 - y0) / span, 0.0, 1.0);
        // tint = GLASS_A * (1 - v) + GLASS_B * v;  tint = tint * 0.5 + col * 0.5
        let tint: V3 = std::array::from_fn(|c| (GLASS_A[c] * (1.0 - v) + GLASS_B[c] * v) * 0.5 + col[c] * 0.5);
        for x in 0..W {
            let m = mk.px(x, y)[0];
            if m == 0 {
                continue; // arr + (tint - arr) * 0 is arr
            }
            let b = bayer.px(x, y)[0];
            // dz = np.floor(BAYER_FULL * 0.9 + 0.5) if k > 0.6 else np.floor(BAYER_FULL * 0.9 + k)
            let dz = if k > 0.6 {
                (b * 0.9 + 0.5).floor()
            } else {
                (b * 0.9 + k).floor()
            };
            // arr += (tint - arr) * (mk * 0.30 * k * dz)[..., None]: the mask float32 up to `dz`, the sum
            // float64, stored as float32
            let w = f64::from(f32::from(m) / 255.0f32 * 0.30f32 * k32) * dz;
            for (a, &tc) in arr.px_mut(x, y).iter_mut().zip(&tint) {
                let was = f64::from(*a);
                *a = (was + (tc - was) * w) as f32;
            }
        }
    }
    // the ribs and the base ring, as 1 px lines
    let mut mr: Img<u8> = Img::new(W, H, 1);
    for r in &ribs {
        let s = cam.wp(r);
        for seg in s.windows(2) {
            pil::line(&mut mr, seg, &[255], 1);
        }
    }
    let ring: Vec<V3> = linspace(0.0, 2.0 * dmath::PI, 33)
        .iter()
        .map(|&a| scene::to_world(cx + rb * dmath::cos(a), cz + rb * dmath::sin(a), 0.03))
        .collect();
    let base = cam.wp(&ring);
    for seg in base.windows(2) {
        pil::line(&mut mr, seg, &[255], 1);
    }
    // arr += (np.clip(GOLD * 0.8 + col * 0.3, 0, 255) - arr) * (mr * 0.85 * k)[..., None]
    let gold: V3 = std::array::from_fn(|c| np::clip(GOLD[c] * 0.8 + col[c] * 0.3, 0.0, 255.0));
    for (i, &m) in mr.data.iter().enumerate() {
        if m == 0 {
            continue;
        }
        let w = f64::from(f32::from(m) / 255.0f32 * 0.85f32 * k32);
        for (a, &gc) in arr.data[i * 3..i * 3 + 3].iter_mut().zip(&gold) {
            let was = f64::from(*a);
            *a = (was + (gc - was) * w) as f32;
        }
    }
}

/// `echo_rings(t, t0, period)`: the radii (beyond the dome) and the alpha of the echo rings: one rolls out every
/// `period` seconds while the dome stands.
fn echo_rings(t: f64, t0: f64, period: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    if t < t0 {
        return out;
    }
    // j = int((t - t0) // period)
    let j = np::f64_to_i64(py::floor_div(t - t0, period));
    for q in (j - 2).max(0)..=j {
        let u = t - (t0 + q as f64 * period);
        if (0.0..1.0).contains(&u) {
            out.push((R * 1.55 + 2.2 * u, (1.0 - u) * 0.6));
        }
    }
    out
}

/// `forest(cx, cz, rad, seed, n=13)`: the pipes' positions, heights, stagger and radii inside the slam circle
/// (seeded per slam): `(x, z, hmax, lag, r)`.
fn forest(cx: f64, cz: f64, rad: f64, seed: u64) -> Vec<(f64, f64, f64, f64, f64)> {
    let mut rng = Generator::new(seed);
    (0..13)
        .map(|_| {
            let rr = rad * rng.uniform(0.02, 1.0).sqrt();
            let a = rng.uniform(0.0, 2.0 * dmath::PI);
            let (x, z) = (cx + rr * dmath::cos(a), cz + rr * dmath::sin(a));
            let hmax = rng.uniform(1.1, 2.5);
            let lag = rng.uniform(0.0, 0.12);
            let r = rng.uniform(0.14, 0.20);
            (x, z, hmax, lag, r)
        })
        .collect()
}

/// `forest_pipes(t, t_slam, cx, cz, rad, seed)`: `(sx, sz, h, glow, r)` while the forest is up (from the slam to
/// 0.65 s after).
fn forest_pipes(t: f64, t_slam: f64, cx: f64, cz: f64, rad: f64, seed: u64) -> Vec<(f64, f64, f64, f64, f64)> {
    let mut out = Vec::new();
    for (x, z, hmax, lag, r) in forest(cx, cz, rad, seed) {
        let u = t - t_slam - lag;
        if u < 0.0 {
            continue;
        }
        let up = py_min(1.0, u / 0.08);
        let down = 1.0 - py_min(1.0, py_max(0.0, (u - 0.45) / 0.18));
        let kk = py_min(up, down);
        if kk <= 0.0 {
            continue;
        }
        out.push((x, z, hmax * ease_out(kk), dmath::exp(-u / 0.25), r));
    }
    out
}

/// One row of `RUN['SLAMS'][i]`: a TOCCATA in the air or just landed.
#[derive(Clone, Copy, Debug)]
struct Slam {
    owner: i64,
    cx: f64,
    cz: f64,
    t0: f64,
    t_slam: f64,
    rad: f64,
}

/// `RUN['SLAMS'][i]`: the rows of recording frame `i` (none when the recording has no `SLAMS`, where Python
/// raises; a row that is not `[owner, x, z, start, slam, radius]` is left out).
fn slams(p: &PictureCtx, i: usize) -> Vec<Slam> {
    let Some(Value::List(rows)) = p.run.nested("SLAMS", i) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let Value::List(v) = row else { return None };
            let Value::Int(owner) = *v.first()? else { return None };
            Some(Slam {
                owner,
                cx: num(v.get(1)?)?,
                cz: num(v.get(2)?)?,
                t0: num(v.get(3)?)?,
                t_slam: num(v.get(4)?)?,
                rad: num(v.get(5)?)?,
            })
        })
        .collect()
}

/// What a painter item of this look is.
enum Item {
    /// `('pipe', pp)`.
    Pipe(Pipe),
    /// `('dome', sk)`.
    Dome(f64),
}

// ---------------------------------------------------------------- the look
/// The number of the next look set up ([`OrganLook::id`]).
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// `_pipes`: the last answer of `pipes_at`, with the look it is of and its time. Python keeps one per look, so
    /// a frame works its pipes out once (and asks `P.fighter` for the champion's place once per solo pipe). A frame
    /// is drawn on one thread, so one entry per thread does the same when frames are drawn on several threads at
    /// once; the answer itself is a pure function of the look and the time.
    static LAST_PIPES: RefCell<Option<(u64, f64, Vec<Pipe>)>> = const { RefCell::new(None) };
}

/// ORGAN's look: `Look` of `kit/fighters/organ.py`.
#[derive(Clone, Debug, Default)]
pub struct OrganLook {
    /// Its side.
    k: usize,
    /// `lines`: its pipe rows, rebuilt from the run.
    lines: Vec<Row>,
    /// `line_t`: when each row was fired.
    line_t: Vec<f64>,
    /// `slam_ev`: `(t_slam, owner, cx, cz)` of its TOCCATA stomps.
    slam_ev: Vec<(f64, usize, f64, f64)>,
    /// `forest_rad`: `abilities['low'].get('radius', 1.4)`.
    forest_rad: f64,
    /// `shield`: CATHEDRAL's `(name, seconds)`, `None` for a fighter without a shield.
    shield: Option<(String, f64)>,
    /// `shield_on`: when each CATHEDRAL was raised.
    shield_on: Vec<f64>,
    /// `solo_n`: its solo as `(beat, midi note, beats)`.
    solo_n: Vec<(f64, i32, f64)>,
    /// `reg`: its register, lowest and highest note.
    reg: (i32, i32),
    /// Which look this is among all made so far (set by `setup_as`): the key of its entry in [`LAST_PIPES`].
    id: u64,
}

impl OrganLook {
    /// `shield_k(st)`: the strength, 0 to 1, of CATHEDRAL's look (it fades in over 0.12 s and out over the last
    /// 0.2 s).
    fn shield_k(&self, st: &BallState) -> f64 {
        let Some((_, dur)) = &self.shield else { return 0.0 };
        if st.shield <= 0.0 {
            return 0.0;
        }
        // float(min(1.0, st['shield'] / 0.2, (dur - st['shield']) / 0.12 + 0.2))
        py_min(py_min(1.0, st.shield / 0.2), (dur - st.shield) / 0.12 + 0.2)
    }

    /// The start of solo note `bt`: `P.T_SOLO + bt * P.T.BEAT`.
    fn solo_t0(p: &PictureCtx, bt: f64) -> f64 {
        p.t_solo + bt * p.show.grid.beat
    }

    /// `solo_pipes(t)`: the organ's solo: a ring of pipes erupts around the champion on every solo note:
    /// `(t0, j, m)` of the notes whose pipe is up.
    fn solo_pipes(&self, p: &PictureCtx, t: f64) -> Vec<(f64, usize, i32)> {
        if p.win != self.k {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (j, &(bt, m, _)) in self.solo_n.iter().enumerate() {
            let t0 = Self::solo_t0(p, bt);
            // (the notes the Sound plays)
            if t0 <= t && t < t0 + 0.7 && t0 < p.show.grid.t_reassemble - 0.05 {
                out.push((t0, j, m));
            }
        }
        out
    }

    /// Where solo note `j`'s pipe stands round the champion at `c`.
    fn solo_seat(c: [f64; 2], j: usize) -> (f64, f64) {
        let a = 2.0 * dmath::PI * j as f64 / 8.0 + 0.4;
        (c[0] + 1.25 * dmath::cos(a), c[1] + 1.25 * dmath::sin(a))
    }

    /// `pipes_at(t)`: its pipes out of the floor (the rows, the TOCCATA forest, the solo ring).
    fn pipes_at(&self, p: &PictureCtx, t: f64) -> Vec<Pipe> {
        // if self._pipes[0] == t: return self._pipes[1]
        let kept = LAST_PIPES.with_borrow(|last| match last {
            Some((id, at, pipes)) if *id == self.id && *at == t => Some(pipes.clone()),
            _ => None,
        });
        if let Some(pipes) = kept {
            return pipes;
        }
        let out = self.pipes_now(p, t);
        LAST_PIPES.set(Some((self.id, t, out.clone())));
        out
    }

    /// The body of `pipes_at(t)`.
    fn pipes_now(&self, p: &PictureCtx, t: f64) -> Vec<Pipe> {
        let mut out = Vec::new();
        let tc = p.fx_clock(t);
        if tc >= p.show.grid.t_fight && !self.line_t.is_empty() {
            // j1 = int(np.searchsorted(self.line_t, tc, 'right'));  self.lines[max(0, j1 - 8):j1]
            let j1 = self.line_t.partition_point(|&v| v <= tc);
            for l in &self.lines[j1.saturating_sub(8)..j1] {
                for (sx, sz, h, glow) in row_pipes(l, tc) {
                    out.push(Pipe {
                        sx,
                        sz,
                        h,
                        glow,
                        owner: l.owner,
                        r: PIPE_R,
                    });
                }
            }
        }
        for &(ts, owner, cx, cz) in &self.slam_ev {
            if ts - 0.05 <= tc && tc < ts + 1.0 {
                // int(ts * 1000)
                for (sx, sz, h, glow, r) in forest_pipes(tc, ts, cx, cz, self.forest_rad, seed_of(ts * 1000.0)) {
                    out.push(Pipe {
                        sx,
                        sz,
                        h,
                        glow,
                        owner,
                        r,
                    });
                }
            }
        }
        let (lo, hi) = self.reg;
        for (t0, j, m) in self.solo_pipes(p, t) {
            // the champion's solo ring
            let c = p.fighter(self.k, t).p;
            let (sx, sz) = Self::solo_seat(c, j);
            let u = t - t0;
            let k = py_min(1.0, u / 0.07) * (1.0 - py_min(1.0, py_max(0.0, (u - 0.45) / 0.2)));
            // h = (0.7 + 1.6 * (hi - mm) / float(hi - lo)) * (1 - (1 - k) ** 2)
            let h = (0.7 + 1.6 * f64::from(hi - m) / f64::from(hi - lo)) * ease_out(k);
            out.push(Pipe {
                sx,
                sz,
                h,
                glow: dmath::exp(-u / 0.2),
                owner: self.k,
                r: 0.14,
            });
        }
        out
    }

    /// The last CATHEDRAL raised at or before `t` (`past[-1]`).
    fn last_shield(&self, t: f64) -> Option<f64> {
        self.shield_on.iter().copied().rfind(|&x| x <= t)
    }
}

/// `DUST_COL` as an emit colour list.
fn dust() -> Option<Vec<V3>> {
    Some(DUST_COL.to_vec())
}

impl Look for OrganLook {
    fn shards(&self) -> Vec<String> {
        // ['pipe'] * 7 + ['case'] * 6 + ['gold'] * 2: #35's organ shatter (after the 24 glass shards)
        [("pipe", 7), ("case", 6), ("gold", 2)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["pipe", "slam", "wave", "shockwave"]
    }

    fn spark_flash(&self) -> &[&str] {
        &["pipe", "slam"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["pipe", "slam", "wave", "shockwave"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["pipe", "slam", "wave"]
    }

    /// A hit on CATHEDRAL: #35's block burst.
    fn block_burst(&self) -> [f64; 3] {
        [220.0, 230.0, 255.0]
    }

    /// A hit on CATHEDRAL: #35's spray.
    fn block_spark(&self) -> Spark {
        ([255.0, 226.0, 170.0], 12, 2.6, 0.3)
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `organ.py` reads from `self.cfg` by name: TOCCATA's radius and its register. (It also walks
    /// `abilities.values()` for the shield, CATHEDRAL's name and length, which has no key of its own: that comes
    /// from the same `cfg`.)
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.low.radius={:?}; register={:?}",
            self.forest_rad, self.reg
        ))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        // its pipe rows, rebuilt from the run (#35 sim.hazards)
        self.lines = p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "line" && e.who() == Some(k))
            .map(|e| Row {
                id: e.i(3),
                owner: k,
                t0: e.t,
                p0: [e.f(4), e.f(5)],
                u: [e.f(6), e.f(7)],
                l: e.f(8),
                speed: e.f(9),
                linger: e.f(10),
            })
            .collect();
        self.line_t = self.lines.iter().map(|l| l.t0).collect();
        // (t_slam, owner, cx, cz)
        self.slam_ev = p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "slam" && e.who() == Some(k))
            .map(|e| (e.t, k, e.f(3), e.f(4)))
            .collect();
        self.forest_rad = cfg.low.get("radius").unwrap_or(1.4);
        // sh = [a for a in abl.values() if a['kind'] == 'shield']: in the roster's order
        let sh = [&cfg.unlock1, &cfg.unlock2, &cfg.low]
            .into_iter()
            .find(|a| a.kind == AbilityKind::Shield);
        // (sh[0]['name'], sh[0]['dur_beats'] * P.T.BEAT): CATHEDRAL
        self.shield = sh.map(|a| {
            let beats = a
                .get("dur_beats")
                .expect("a shield has dur_beats (the roster refuses one without)");
            (a.name.clone(), beats * p.show.grid.beat)
        });
        self.shield_on = match &self.shield {
            Some((name, _)) => p
                .rec
                .events
                .iter()
                .filter(|e| e.kind == "ability" && e.who() == Some(k) && e.s(3) == name)
                .map(|e| e.t)
                .collect(),
            None => Vec::new(),
        };
        self.solo_n = p.solos[k].clone();
        self.reg = cfg.register;
    }

    // ------------------------------------------------------------ state
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        // the organ ball thumps on every row
        if kk != self.k || phase != Phase::Fight || self.line_t.is_empty() {
            return;
        }
        let tt = py_min(t, p.t_ko);
        // j = np.searchsorted(self.line_t, tt, 'right') - 1; if j >= 0: ...
        let after = self.line_t.partition_point(|&v| v <= tt);
        if let Some(j) = after.checked_sub(1) {
            st.glow = 0.42 * dmath::exp(-(tt - self.line_t[j]) / 0.10);
        }
    }

    // ------------------------------------------------------------ events
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "pipe" => Some(1.1),
            "slam" => Some(2.5),
            // TOCCATA's stomp (the 'slam' event): 1.6
            _ => DefaultLook.shake(p, e),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        if self.spark_big().contains(&how(e)) {
            Some((12.0, 0.4))
        } else {
            None
        }
    }

    fn fringe_size(&self, _p: &PictureCtx, e: &Event) -> (f64, f64) {
        if how(e) == "pipe" { (18.0, 1.6) } else { (26.0, 2.5) }
    }

    /// CATHEDRAL glows from inside: a wine point light in the dome lights the canvas, ropes and crowd.
    fn light(&self, p: &PictureCtx, l: &mut Lights, t: f64) {
        if self.shield.is_none() || !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return;
        }
        let st = p.fighter(self.k, t);
        let sk = self.shield_k(&st);
        if sk > 0.0 {
            // P.COL[self.k] / 255.0 * 0.9 * sk
            let rgb = p.col[self.k].map(|c| c / 255.0 * 0.9 * sk);
            l.point(scene::to_world(st.p[0], st.p[1], R * 1.6), rgb, 2.0);
        }
    }

    // ------------------------------------------------------------ effects (reelkit.fx), as #35 built them
    /// The rows: each pipe kicks up dust as it bursts out; light runs along the row; the floor stays dusty a
    /// while.
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        for l in &self.lines {
            let n = row_count(l.l);
            for j in 0..n {
                let x = (j as f64 + 0.5) * l.l / n as f64;
                let te = l.t0 + x / l.speed;
                let q = [l.p0[0] + l.u[0] * x, l.p0[1] + l.u[1] * x];
                let emit = Emit {
                    n: Some(2),
                    col: dust(),
                    vel: scene::to_world(l.u[0] * 0.3, l.u[1] * 0.3, 0.0),
                    ..Emit::default()
                };
                fx.pa.emit(te, scene::to_world(q[0], q[1], 0.06), "dust", &emit);
                if j % 3 == 1 {
                    let at = scene::to_world(q[0], q[1], 1.0);
                    fx.fl.add(te, at, p.fl_col[self.k], 1.2, 0.6, 0.03, 0.18, 0.0);
                }
                if j % 3 == 0 {
                    fx.mk.add(
                        te,
                        Mark {
                            size: 0.24,
                            col: [196.0, 180.0, 172.0],
                            life: 2.4,
                            // seed=l['id'] * 31 + j
                            seed: (l.id * 31 + j).max(0) as u64,
                            strength: 0.7,
                            ..Mark::new(MarkKind::Dust, q[0], q[1])
                        },
                    );
                }
            }
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.who() != Some(self.k) {
            return;
        }
        let t = e.t;
        if e.kind == "hit" {
            let (att, x, z) = (self.k, e.f(5), e.f(6));
            let pos = scene::to_world(x, z, R);
            match how(e) {
                "pipe" => {
                    // a row lands: burst, wine light, dust, a crack
                    fx.bu.add(t, pos, p.col[att], 1.3, None, true);
                    fx.fl.add(t, pos, p.fl_col[att], 1.6, 0.9, 0.03, 0.28, 0.0);
                    let emit = Emit {
                        n: Some(10),
                        col: dust(),
                        speed: Some((0.6, 1.6)),
                        ..Emit::default()
                    };
                    fx.pa.emit(t, scene::to_world(x, z, 0.1), "dust", &emit);
                    fx.mk.add(
                        t,
                        Mark {
                            size: 0.55,
                            n: 5,
                            life: 4.0,
                            // seed=int(e[0] * 100)
                            seed: seed_of(t * 100.0),
                            ..Mark::new(MarkKind::Crack, x, z)
                        },
                    );
                }
                "slam" => fx.bu.add(t, pos, p.col[att], 2.0, None, true),
                _ => {}
            }
        } else if e.kind == "block" {
            // CATHEDRAL blocks a hit: stone dust
            let emit = Emit {
                n: Some(5),
                col: dust(),
                ..Emit::default()
            };
            fx.pa.emit(t, scene::to_world(e.f(4), e.f(5), R), "dust", &emit);
        }
    }

    /// TOCCATA's forest: dust everywhere, a cracked crater, a big wine light (after the event pass, as #35).
    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {
        for &(ts, owner, cx, cz) in &self.slam_ev {
            let c = scene::to_world(cx, cz, 0.1);
            fx.fl.add(
                ts,
                scene::to_world(cx, cz, 1.2),
                p.fl_col[owner],
                2.4,
                1.2,
                0.03,
                0.5,
                0.0,
            );
            let emit = Emit {
                n: Some(30),
                col: dust(),
                spread: Some(180.0),
                speed: Some((0.6, 1.8)),
                scale: 1.3,
                ..Emit::default()
            };
            fx.pa.emit(ts, c, "dust", &emit);
            // seed=int(ts * 10)
            let seed = seed_of(ts * 10.0);
            fx.mk.add(
                ts,
                Mark {
                    size: 1.6,
                    n: 8,
                    life: 7.0,
                    seed,
                    ..Mark::new(MarkKind::Crack, cx, cz)
                },
            );
            fx.mk.add(
                ts,
                Mark {
                    size: 1.2,
                    col: [120.0, 40.0, 70.0],
                    life: 7.0,
                    seed: seed + 1,
                    strength: 0.6,
                    ..Mark::new(MarkKind::Scorch, cx, cz)
                },
            );
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let emit = Emit {
            n: Some(30),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: dust(),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "dust", &emit);
    }

    /// The organ's solo: every solo pipe kicks up a little dust and lights up wine (after the K.O., as #35).
    fn fx_end(&self, p: &PictureCtx, fx: &mut Fx) {
        if p.win != self.k {
            return;
        }
        for (j, &(bt, _, _)) in self.solo_n.iter().enumerate() {
            let t0 = Self::solo_t0(p, bt);
            if t0 < p.show.grid.t_reassemble - 0.05 {
                let c = p.fighter(self.k, t0).p;
                let (qx, qz) = Self::solo_seat(c, j);
                let emit = Emit {
                    n: Some(3),
                    col: dust(),
                    ..Emit::default()
                };
                let at = t0 - PictureCtx::HITSTOP;
                fx.pa.emit(at, scene::to_world(qx, qz, 0.06), "dust", &emit);
                fx.fl.add(
                    at,
                    scene::to_world(qx, qz, 1.2),
                    p.fl_col[self.k],
                    1.4,
                    0.7,
                    0.03,
                    0.25,
                    0.0,
                );
            }
        }
    }

    // ------------------------------------------------------------ the canvas
    fn shadows_free(&self, p: &PictureCtx, f: &FrameCtx<'_>, spot: V3) -> Vec<Vec<(f64, f64)>> {
        self.pipes_at(p, f.t)
            .iter()
            .filter(|q| q.h > 0.05)
            .map(|q| proj_point(&pipe_shadow_quad(q.sx, q.sz, q.h, q.r), spot, 0.93))
            .collect()
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        // the pipes' glow pools while they speak
        for q in self.pipes_at(p, c.t) {
            if q.glow > 0.15 {
                let tint = p.col[q.owner].map(|v| v / 255.0 + 0.15);
                for (j, light) in ic.iter_mut().enumerate() {
                    let (dx, dz) = (c.sxp[j] - q.sx, c.szp[j] - q.sz);
                    if dx.abs() < 0.5 && dz.abs() < 0.5 {
                        let dd = np::hypot(dx, dz);
                        let v = np::clip(1.0 - dd / 0.45, 0.0, 1.0) * q.glow * 0.35;
                        for ch in 0..3 {
                            light[ch] += v * tint[ch];
                        }
                    }
                }
            }
        }
        // CATHEDRAL's echo rings
        let st = &c.fs[k];
        if self.shield.is_some()
            && st.shield > 0.0
            && c.in_fight
            && let Some(t0) = self.last_shield(c.t)
        {
            let tint = p.col[k].map(|v| v / 255.0 + 0.25);
            for (rr, a) in echo_rings(c.t, t0, 2.0 * p.show.grid.beat) {
                for (j, light) in ic.iter_mut().enumerate() {
                    let rd = np::hypot(c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                    let band = np::clip(1.0 - (rd - rr).abs() / 0.08, 0.0, 1.0);
                    let v = band * a * 0.7;
                    for ch in 0..3 {
                        light[ch] += v * tint[ch];
                    }
                }
            }
        }
    }

    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        // TOCCATA's target + the stomp ring
        for s in slams(p, c.i) {
            if s.owner != self.k as i64 {
                continue;
            }
            if c.t < s.t_slam {
                let u = (c.t - s.t0) / (s.t_slam - s.t0);
                let dim = 1.0 - 0.55 * u;
                // (P.COL[owner] / 255.0) * (0.4 + 0.5 * (math.sin(C.t * 40) > 0))
                let blink = 0.4 + 0.5 * gate(dmath::sin(c.t * 40.0) > 0.0);
                let ring = p.col[self.k].map(|v| v / 255.0 * blink);
                for (j, light) in ic.iter_mut().enumerate() {
                    let rd = np::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                    if rd < s.rad {
                        for v in light.iter_mut() {
                            *v *= dim;
                        }
                    }
                    if (rd - s.rad).abs() < 0.05 {
                        for ch in 0..3 {
                            light[ch] += ring[ch];
                        }
                    }
                }
            } else {
                let u = (c.t - s.t_slam) / 0.6;
                let white = [0.9, 0.9, 1.0];
                for (j, light) in ic.iter_mut().enumerate() {
                    let rd = np::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                    let band = np::clip(1.0 - (rd - (s.rad + 1.6 * u)).abs() / 0.2, 0.0, 1.0) * (1.0 - u);
                    for ch in 0..3 {
                        light[ch] += band * white[ch];
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ the painter: pipes, the dome
    fn items_ball(&self, _p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        let sk = self.shield_k(st);
        if sk <= 0.0 {
            return Vec::new();
        }
        let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
        let dome: Payload = Box::new(Item::Dome(sk));
        vec![(dz - 0.05, dome)]
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        self.pipes_at(p, f.t)
            .into_iter()
            .map(|q| {
                let depth = f.cam.depth(&[scene::to_world(q.sx, q.sz, 0.0)])[0];
                let pipe: Payload = Box::new(Item::Pipe(q));
                (depth, pipe)
            })
            .collect()
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        match payload.downcast_ref::<Item>() {
            Some(Item::Pipe(q)) => draw_pipe(fr, f.cam, f.l, q.sx, q.sz, q.h, q.glow, p.col[q.owner], q.r),
            Some(Item::Dome(sk)) => {
                let at = f.fs[self.k].p;
                let (mut arr, depth) = (fr.array(), fr.depth());
                draw_dome(&mut arr, f.cam, at[0], at[1], *sk, p.col[self.k]);
                fr.set_array(&arr, Some(&depth));
            }
            None => {}
        }
    }

    fn draw_shard(
        &self,
        _p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        kind: &str,
        pos: V3,
        rot: f64,
        size: f64,
        _curl: f64,
    ) {
        let (sx, sz) = (pos[0], pos[2]);
        let d = unit(rot);
        let nn = [-d[1], d[0]];
        let on_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            screen(cam, &world)
        };
        if kind == "pipe" {
            // a length of tin pipe
            let q4: Vec<[f64; 2]> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(a1, a2)| {
                    [
                        sx + d[0] * 0.22 * a1 + nn[0] * 0.06 * a2,
                        sz + d[1] * 0.22 * a1 + nn[1] * 0.06 * a2,
                    ]
                })
                .collect();
            let k = 0.8 + 0.2 * dmath::cos(rot * 2.0);
            layer_blend(arr, &[on_screen(&q4)], TIN.map(|c| c * k), 1.0, None);
            return;
        }
        // case wood chips, gilt (Python has no colour for another kind and raises; `shards` names no other)
        let col = match kind {
            "gold" => GOLD,
            "case" => CASE,
            _ => return,
        };
        let s = size * if kind == "case" { 1.2 } else { 0.9 };
        let tri = [
            [sx + d[0] * s, sz + d[1] * s],
            [
                sx - d[0] * s * 0.7 + nn[0] * s * 0.8,
                sz - d[1] * s * 0.7 + nn[1] * s * 0.8,
            ],
            [
                sx - d[0] * s * 0.4 - nn[0] * s * 0.7,
                sz - d[1] * s * 0.4 - nn[1] * s * 0.7,
            ],
        ];
        let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
        let col = col.map(|c| np::clip(c * shade_k, 0.0, 255.0));
        layer_blend(arr, &[on_screen(&tri)], col, 1.0, None);
    }

    // ------------------------------------------------------------ lens, glows
    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        let tc = p.fx_clock(t);
        // TOCCATA's shock ring
        for &(ts, _, cx, cz) in &self.slam_ev {
            let u = (tc - ts) / 0.6;
            if (0.0..1.0).contains(&u) {
                let c = scene::to_world(cx, cz, 0.05);
                img.shimmer_world(cam, c, self.forest_rad + 1.6 * u, 0.35, 3.0, t, 1.0 - u, 0.0);
            }
        }
        // CATHEDRAL's echo rings
        if self.shield.is_some() && p.show.grid.t_fight <= t && t < p.t_hs {
            let st = p.fighter(self.k, t);
            if st.shield > 0.0
                && let Some(t0) = self.last_shield(t)
            {
                for (rr, a) in echo_rings(t, t0, 2.0 * p.show.grid.beat) {
                    let c = scene::to_world(st.p[0], st.p[1], 0.05);
                    img.shimmer_world(cam, c, rr, 0.2, 1.6, t, a / 0.6, 0.0);
                }
            }
        }
    }

    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _drawn: &[DrawnBall]) {
        // the speaking pipe tops glow
        for q in self.pipes_at(p, f.t) {
            if q.glow > 0.3 {
                let s = f.cam.wp(&[scene::to_world(q.sx, q.sz, q.h)])[0];
                fr.glow(s.0, s.1, 5.0, p.col[q.owner].map(|c| c * 0.7 + 70.0), 0.35 * q.glow);
            }
        }
    }

    fn focus_points(&self, p: &PictureCtx, t: f64, _st: &BallState) -> Vec<(V3, String)> {
        self.pipes_at(p, t)
            .iter()
            .map(|q| (scene::to_world(q.sx, q.sz, q.h), "pipe top".to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(l: f64) -> Row {
        Row {
            id: 0,
            owner: 0,
            t0: 10.0,
            p0: [0.0, 0.0],
            u: [1.0, 0.0],
            l,
            speed: 6.0,
            linger: 0.55,
        }
    }

    /// A row's pipes come up one after another as the front runs out, stay up `linger` seconds and sink.
    #[test]
    fn a_row_rises_in_sequence_and_sinks() {
        let line = row(1.6);
        // int(1.6 / 0.34) = 4 pipes, at x = 0.2, 0.6, 1.0, 1.4
        assert!(row_pipes(&line, 10.0).is_empty());
        let early = row_pipes(&line, 10.0 + 0.2 / 6.0 + 0.01);
        assert_eq!(early.len(), 1);
        assert!((early[0].0 - 0.2).abs() < 1e-12 && early[0].2 > 0.0 && early[0].2 < pipe_height(0.2, 1.6));
        let all = row_pipes(&line, 10.4);
        assert_eq!(all.len(), 4);
        for (i, q) in all.iter().enumerate() {
            let x = (i as f64 + 0.5) * 1.6 / 4.0;
            assert_eq!(q.2, pipe_height(x, 1.6), "pipe {i} is fully up");
        }
        // the first has sunk 0.55 + 0.12 s after it came up; the last is still there
        let late = row_pipes(&line, 10.0 + 0.2 / 6.0 + 0.68);
        assert_eq!(late.len(), 3);
        assert!(row_pipes(&line, 12.0).is_empty());
        // a row too short to hold a pipe draws nothing
        assert!(row_pipes(&row(0.05), 10.3).is_empty());
    }

    /// The echo rings: one every `period`, each alive for a second, growing from the dome's rim and fading.
    #[test]
    fn an_echo_ring_rolls_out_every_period() {
        assert!(echo_rings(4.9, 5.0, 0.9).is_empty());
        assert_eq!(echo_rings(5.0, 5.0, 0.9), vec![(R * 1.55, 0.6)]);
        let two = echo_rings(5.0 + 0.95, 5.0, 0.9);
        assert_eq!(two.len(), 2);
        assert!(
            two[0].0 > two[1].0 && two[0].1 < two[1].1,
            "the older ring is wider and fainter"
        );
        // a ring is gone a second after it started
        assert_eq!(echo_rings(5.0 + 1.85, 5.0, 0.9).len(), 2);
    }

    /// TOCCATA's forest is 13 pipes inside the circle, the same for the same seed, up from the slam to 0.65 s
    /// after its last lag.
    #[test]
    fn the_forest_is_seeded_and_inside_its_circle() {
        let a = forest(0.5, -0.5, 1.5, 21_345);
        assert_eq!(a, forest(0.5, -0.5, 1.5, 21_345));
        assert_ne!(a, forest(0.5, -0.5, 1.5, 21_346));
        assert_eq!(a.len(), 13);
        for &(x, z, hmax, lag, r) in &a {
            assert!(dmath::hypot(x - 0.5, z + 0.5) <= 1.5 + 1e-12);
            assert!((1.1..2.5).contains(&hmax) && (0.0..0.12).contains(&lag) && (0.14..0.20).contains(&r));
        }
        assert!(forest_pipes(9.99, 10.0, 0.5, -0.5, 1.5, 21_345).is_empty());
        assert_eq!(forest_pipes(10.3, 10.0, 0.5, -0.5, 1.5, 21_345).len(), 13);
        assert!(forest_pipes(10.76, 10.0, 0.5, -0.5, 1.5, 21_345).is_empty());
    }

    /// A pipe has six side quads and a six-cornered top; the shadow card stands across it.
    #[test]
    fn a_pipe_is_a_hex_prism() {
        let faces = pipe_faces(0.3, -0.2, 1.2, PIPE_R);
        assert_eq!(faces.len(), 7);
        assert_eq!(faces.iter().filter(|f| matches!(f.2, Face::Side(_))).count(), 6);
        assert_eq!(
            (faces[6].0.len(), faces[6].1, faces[6].2),
            (6, [0.0, 1.0, 0.0], Face::Top)
        );
        for f in &faces[..6] {
            assert!(
                (dot(f.1, f.1) - 1.0).abs() < 1e-6 && f.1[1] == 0.0,
                "a side's normal is level"
            );
        }
        let card = pipe_shadow_quad(0.3, -0.2, 1.2, PIPE_R);
        assert_eq!((card[0][1], card[3][1]), (0.0, 1.2));
    }

    /// Where scipy raises, the kit draws nothing: a flat set of points has no outline.
    #[test]
    fn a_flat_hull_is_skipped_as_the_kit_does() {
        assert_eq!(hull_of(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]), None);
        assert_eq!(hull_of(&[(0.0, 0.0), (1.0, 1.0)]), None);
        let square = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0), (2.0, 2.0)];
        assert_eq!(hull_of(&square).map(|h| h.len()), Some(4));
    }
}
