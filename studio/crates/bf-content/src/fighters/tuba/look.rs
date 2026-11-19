//! TUBA's look: the port of `Look` in `projects/ballfight/kit/fighters/tuba.py`, hook by hook, with the module's
//! draw helpers (`bell_rings`, `_ring_pts`, `draw_tuba_bell`, `inhale_light`, `draw_dome`, `ripple_light`).
//!
//! What it draws: the brass bell growing out of the ball toward its aim (it opens on "oom", flares on "pah", and
//! on every note of the solo when it wins); the suction cone on the canvas, under every other decal, with air
//! streaks pouring into the mouth; the gulped enemy shrinking into the mouth, hidden inside (no HP number), and
//! popping back out; the tuba ball bulging while it holds the enemy; the pah's blast (a burst, puffs, a smoke ring,
//! the cannonball's streak trail) and the crack where the cannonball hits the ropes; BRASS WALL's dome; SUBSONIC's
//! ripple on the canvas with its foam and shimmer; brass, tube and valve shards; the VS card's rings round its ball.
//!
//! The arithmetic keeps Python's order and dtypes: scalar maths through `bf_core::dmath`, the 3-vector `np.dot` and
//! `np.linalg.norm` fused as OpenBLAS adds them (ruling M3-R30), the dome's blends in `float32` where numpy's are
//! (a `float32` mask times a Python float stays `float32`).

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::rng::Generator;
use bf_core::roster::{AbilityKind, FighterCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{BAYER, FPS, H, W, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::{Img, RgbF};
use bf_picture::lighting::Lights;
use bf_picture::look::{CanvasCtx, DefaultLook, FrameCtx, Fx, Look, Payload, Phase, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::title::layout::Layout;
use bf_picture::{np, pil};

use super::state::{self, Held, Oom, Pah, Sub};
use crate::fighters::shared::reeds;

/// `BRASS`: #32's brass (the bell, the dome).
const BRASS: V3 = [232.0, 184.0, 78.0];
/// `BRASS_HI`.
const BRASS_HI: V3 = [255.0, 236.0, 160.0];
/// `SHARD_BRASS`: #36's shards were drawn in #34's brass.
const SHARD_BRASS: V3 = [232.0, 188.0, 92.0];
/// `SHARD_BRASS_DK`.
const SHARD_BRASS_DK: V3 = [150.0, 104.0, 40.0];
/// `STREAK_COL`: the inhale streaks.
const STREAK_COL: [V3; 2] = [[200.0, 196.0, 255.0], [170.0, 160.0, 240.0]];
/// The pah's light, `(255, 214, 120)`.
const PAH_COL: V3 = [255.0, 214.0, 120.0];
/// `R * 0.85 + 0.5`: the bell's mouth from the ball's centre.
const MOUTH: f64 = R * 0.85 + 0.5;

// ---------------------------------------------------------------- small vectors, as numpy computes them
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `np.dot(a, b)` of two 3-vectors: OpenBLAS `ddot` on the reference PC adds the products fused,
/// `fma(a2, b2, fma(a1, b1, a0 * b0))` (as the sprite camera's dot product, ruling M3-R30).
fn dot(a: V3, b: V3) -> f64 {
    dmath::fma(a[2], b[2], dmath::fma(a[1], b[1], a[0] * b[0]))
}

/// `np.linalg.norm(a)` of a 3-vector: the root of the same fused `a.dot(a)`.
fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// `np.cross(a, b)`.
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `Q.mean(0)`: the rows added in order, then divided by their number.
fn mean(rows: &[V3]) -> V3 {
    let mut s = [0.0; 3];
    for r in rows {
        s = add(s, *r);
    }
    let n = rows.len() as f64;
    [s[0] / n, s[1] / n, s[2] / n]
}

/// `np.array([math.cos(th), math.sin(th)])`.
fn unit(th: f64) -> [f64; 2] {
    [dmath::cos(th), dmath::sin(th)]
}

/// `tuple(int(v) for v in col)` as a Pillow ink.
fn ink(col: V3) -> [u8; 3] {
    [col[0] as u8, col[1] as u8, col[2] as u8]
}

/// A numpy boolean as a factor: `x * (a > b)`.
fn gate(on: bool) -> f64 {
    if on { 1.0 } else { 0.0 }
}

/// `np.linspace(0, 2 * math.pi, n + 1)`: `i * step`, the last value the end itself.
fn turn(n: usize) -> Vec<f64> {
    let end = 2.0 * dmath::PI;
    let step = end / n as f64;
    (0..=n)
        .map(|i| if i == n { end } else { i as f64 * step + 0.0 })
        .collect()
}

/// `BAYER_FULL[y, x]`.
fn bayer(x: usize, y: usize) -> f64 {
    BAYER[y % 4][x % 4]
}

/// `q = lambda v: np.floor(v * 3 + BAYER_FULL[ys, xs] * 0.9) / 3`.
fn q3(v: f64, b: f64) -> f64 {
    (v * 3.0 + b * 0.9).floor() / 3.0
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// The mouth of a bell at `p` aimed along `th`: `p + (R * 0.85 + 0.5) * np.array([math.cos(th), math.sin(th)])`.
fn mouth_of(p: [f64; 2], th: f64) -> [f64; 2] {
    let d = unit(th);
    [p[0] + MOUTH * d[0], p[1] + MOUTH * d[1]]
}

// ---------------------------------------------------------------- #32 styles.py
/// `bell_rings(p, th, open_k, flare, y=R * 1.05)`: the bell growing out of the tuba ball toward its aim, as
/// `(centre in the world, radius)` from the throat to the mouth, and the mouth's axis. `open_k` (0 to 1) widens
/// the mouth while it inhales, `flare` (0 to 1) is the pah's punch.
fn bell_rings(p: [f64; 2], th: f64, open_k: f64, flare: f64) -> ([(V3, f64); 5], V3) {
    let y = R * 1.05;
    let d = unit(th);
    let l = 0.50 + 0.12 * open_k + 0.18 * flare;
    let rm = 0.36 + 0.22 * open_k + 0.16 * flare;
    let ring = |u: f64, rr: f64| {
        // c = p + d * (R * 0.85 + L * u)
        let out = R * 0.85 + l * u;
        (scene::to_world(p[0] + d[0] * out, p[1] + d[1] * out, y), rr)
    };
    let rings = [
        ring(0.0, 0.13),
        ring(0.45, 0.15),
        ring(0.75, 0.2),
        ring(0.92, rm * 0.78),
        ring(1.0, rm),
    ];
    let axis = sub(scene::to_world(d[0], d[1], 0.0), scene::to_world(0.0, 0.0, 0.0));
    (rings, axis)
}

/// `_ring_pts(c, axis, r, n)`: a vertical circle of radius `r` round the world centre `c`, facing the horizontal
/// world direction `axis`.
fn ring_pts(c: V3, axis: V3, r: f64, n: usize) -> Vec<V3> {
    let len = norm(axis) + 1e-9;
    let ax = [axis[0] / len, axis[1] / len, axis[2] / len];
    let up_ = [0.0, 1.0, 0.0];
    let side = cross(up_, ax);
    let len = norm(side) + 1e-9;
    let side = [side[0] / len, side[1] / len, side[2] / len];
    let a = turn(n);
    a[..n]
        .iter()
        .map(|&a| {
            let (ca, sa) = (dmath::cos(a), dmath::sin(a));
            // c[None] + r * (np.cos(a)[:, None] * side[None] + np.sin(a)[:, None] * up[None])
            std::array::from_fn(|ch| c[ch] + r * (ca * side[ch] + sa * up_[ch]))
        })
        .collect()
}

/// `draw_tuba_bell(fr, cam, L, shade, p, th, open_k, flare, col, glow)`: the brass flare (banded segments from the
/// throat to the mouth, lit from the spot side), a bright lip and, when the mouth faces the camera, the dark throat
/// inside (tinted in the fighter's colour while it inhales).
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_tuba_bell(
    p: &PictureCtx,
    fr: &mut Frame,
    cam: &Cam,
    l: &Lights,
    at: [f64; 2],
    th: f64,
    open_k: f64,
    flare: f64,
    col: V3,
    glow: f64,
) {
    let (rings, axis) = bell_rings(at, th, open_k, flare);
    let pts: Vec<Vec<V3>> = rings.iter().map(|&(c, r)| ring_pts(c, axis, r, 14)).collect();
    let n = pts[0].len();
    for j in 0..pts.len() - 1 {
        let (a_, b_) = (&pts[j], &pts[j + 1]);
        for i in 0..n {
            let q = [a_[i], a_[(i + 1) % n], b_[(i + 1) % n], b_[i]];
            let mut nrm = cross(sub(q[1], q[0]), sub(q[3], q[0]));
            let len = norm(nrm) + 1e-9;
            nrm = [nrm[0] / len, nrm[1] / len, nrm[2] / len];
            let ctr = mean(&q);
            if dot(nrm, sub(cam.pos, ctr)) < 0.0 {
                nrm = [-nrm[0], -nrm[1], -nrm[2]];
            }
            // base_ = BRASS * (0.9 + 0.1 * j / 3) + np.array(col) * 0.06
            let band = 0.9 + 0.1 * j as f64 / 3.0;
            let base: V3 = std::array::from_fn(|ch| BRASS[ch] * band + col[ch] * 0.06);
            let c = p.shade(l, &[nrm], &[ctr], &[base], None, 0.6, 0.0, 5.0)[0];
            // np.clip(c * (1 + 0.5 * glow) + 30 * glow, 0, 255)
            let c = c.map(|v| np::clip(v * (1.0 + 0.5 * glow) + 30.0 * glow, 0.0, 255.0));
            fr.poly(cam, &q, c, true);
        }
    }
    let (mouth_c, rm) = rings[4];
    let facing = dot(axis, sub(cam.pos, mouth_c)) > 0.0;
    let mut lip = ring_pts(mouth_c, axis, rm, 20);
    if facing {
        let back: V3 = std::array::from_fn(|ch| mouth_c[ch] - axis[ch] * 0.02);
        let inner = ring_pts(back, axis, rm * 0.82, 20);
        // ink = np.array(col) * (0.18 + 0.35 * open_k) + np.array([10, 6, 16])
        let dark = [10.0, 6.0, 16.0];
        let inside: V3 = std::array::from_fn(|ch| np::clip(col[ch] * (0.18 + 0.35 * open_k) + dark[ch], 0.0, 255.0));
        fr.poly(cam, &inner, inside, true);
    }
    // fr.line(cam, np.vstack([lip, lip[:1]]), np.clip(BRASS_HI * (1 + 0.3 * glow), 0, 255), 1)
    lip.push(lip[0]);
    fr.line(
        cam,
        &lip,
        BRASS_HI.map(|v| np::clip(v * (1.0 + 0.3 * glow), 0.0, 255.0)),
        1,
    );
}

/// `inhale_light(sxp, szp, p, th, reach, k, t, ys, xs, half=0.8)` for one canvas pixel: the suction cone, a wedge in
/// front of the bell (half-angle 0.8 rad, out to the reach) with streaks of air racing in toward the mouth.
/// Returns `(bright, dark)`: thin bright streaks over darker gaps.
#[allow(clippy::too_many_arguments)] // the Python signature
fn inhale_light(sx: f64, sz: f64, p: [f64; 2], th: f64, reach: f64, k: f64, t: f64, b: f64) -> (f64, f64) {
    let half = 0.8;
    let (rel_x, rel_z) = (sx - p[0], sz - p[1]);
    let rd = np::hypot(rel_x, rel_z);
    let ang = dmath::atan2(rel_z, rel_x);
    // da = (ang - th + math.pi) % (2 * math.pi) - math.pi
    let da = py::fmod_py(ang - th + dmath::PI, 2.0 * dmath::PI) - dmath::PI;
    let rmax = R * 2.0 + reach;
    let inside = gate(da.abs() < half && rd > R * 0.95 && rd < rmax);
    // np.clip(1 - np.abs(da) / half, 0, 1) ** 0.7 * np.clip(1 - (rd - R) / (rmax - R), 0, 1) ** 0.5
    let fall = dmath::pow(np::clip(1.0 - da.abs() / half, 0.0, 1.0), 0.7)
        * np::clip(1.0 - (rd - R) / (rmax - R), 0.0, 1.0).sqrt();
    // radial lanes, and dashes travelling inward
    let lane = dmath::sin(da * 9.0) > 0.55;
    let flow = py::fmod_py(rd * 3.2 + t * 9.0 + (da * 9.0 / dmath::PI).floor() * 0.37, 1.0);
    let bright = gate(lane && flow < 0.28) * fall * inside;
    let dark = fall * inside * 0.5;
    (q3(bright, b) * k, q3(dark, b) * k)
}

/// `arr += (col - arr) * (m * a * k)[..., None]` as numpy computes it for a `float32` picture and mask and two
/// Python floats: the mask's products stay `float32`, the blend is `float64`, the sum is stored as `float32`.
fn blend_mask(arr: &mut Img<f32>, m: &Img<f32>, col: V3, a: f64, k: f64) {
    let (a, k) = (a as f32, k as f32);
    for (px, &mv) in arr.data.as_chunks_mut::<3>().0.iter_mut().zip(&m.data) {
        let w = f64::from(mv * a * k);
        for (v, &c) in px.iter_mut().zip(&col) {
            let x = f64::from(*v);
            *v = (x + (c - x) * w) as f32;
        }
    }
}

/// `draw_dome(arr, cam, cx, cz, k, t, col)`: a brass dome over the tuba ball: a banded hemisphere, a gleam sweeping
/// round, a rim ring on the canvas; translucent at the edges so the tuba still reads inside.
fn draw_dome(arr: &mut Img<f32>, cam: &Cam, cx: f64, cz: f64, k: f64, t: f64, col: V3) {
    let rr = R * 1.45;
    let a = turn(40);
    // S.to_world(cx + rr * f * np.cos(a), cz + rr * f * np.sin(a), h)
    let circle = |rad: f64, h: f64| -> Vec<V3> {
        a.iter()
            .map(|&u| scene::to_world(cx + rad * dmath::cos(u), cz + rad * dmath::sin(u), h))
            .collect()
    };
    let bands = [(0.0, 1.0), (rr * 0.45, 0.9), (rr * 0.75, 0.68), (rr * 0.93, 0.38)];
    for (j, &(h, f)) in bands.iter().enumerate() {
        let j = j as f64;
        let m = reeds::mask_all(&[cam.wp(&circle(rr * f, h))]);
        let g = 0.62 + 0.12 * j;
        // col_j = BRASS * g + np.array(col) * 0.10, clipped to 0..255
        let col_j: V3 = std::array::from_fn(|ch| np::clip(BRASS[ch] * g + col[ch] * 0.10, 0.0, 255.0));
        blend_mask(arr, &m, col_j, 0.30 + 0.12 * j, k);
    }
    let sweep = t * 3.0;
    let gleam = scene::to_world(
        cx + rr * 0.7 * dmath::cos(sweep),
        cz + rr * 0.7 * dmath::sin(sweep),
        rr * 0.55,
    );
    let s = cam.wp(&[gleam])[0];
    let (yy, xx) = (py::round_half_even(s.1) as i64, py::round_half_even(s.0) as i64);
    if 1 <= yy && yy < H as i64 - 1 && 1 <= xx && xx < W as i64 - 1 {
        // arr[yy - 1:yy + 2, xx - 1:xx + 2] = arr[...] * (1 - k) + BRASS_HI * k
        let keep = (1.0 - k) as f32;
        for y in yy - 1..yy + 2 {
            for x in xx - 1..xx + 2 {
                for (v, &hi) in arr.px_mut(x as usize, y as usize).iter_mut().zip(&BRASS_HI) {
                    *v = (f64::from(*v * keep) + hi * k) as f32;
                }
            }
        }
    }
    // the rim ring on the canvas: a 1 px line
    let mk = reeds::mask(&[], &[cam.wp(&circle(rr, 0.02))], 1);
    blend_mask(arr, &mk, BRASS_HI, 0.8, k);
}

/// `ripple_light(sxp, szp, cx, cz, t, t0, t_slam, rad, ys, xs)` for one canvas pixel at distance `rd` from the
/// spot: the telegraph, rings sucked in toward the predicted spot; then the hit, water ripples racing out across
/// the canvas and fading over 1.6 s. Returns `(light added, albedo darkened)`.
fn ripple_light(rd: f64, t: f64, t0: f64, t_slam: f64, rad: f64, b: f64) -> (f64, f64) {
    let (mut add_, mut dark) = (0.0, 0.0);
    if t < t_slam {
        let u = (t - t0) / py_max(1e-6, t_slam - t0);
        for j in 0..3 {
            let ph = py::fmod_py(u * 3.0 + f64::from(j) / 3.0, 1.0);
            let r_ = rad * 1.6 * (1.0 - ph);
            add_ += np::clip(1.0 - (rd - r_).abs() / 0.05, 0.0, 1.0) * dmath::sin(dmath::PI * ph) * 0.5;
        }
        dark += gate(rd < rad) * 0.45 * u;
    } else {
        let u = t - t_slam;
        if u < 1.6 {
            for j in 0..4 {
                let j = f64::from(j);
                let r_ = 0.2 + 3.2 * py_max(0.0, u - j * 0.12);
                if r_ <= 0.2 {
                    continue;
                }
                // np.cos(np.clip((rd - r_) / 0.16, -1.5, 1.5) * math.pi / 3) * (np.abs(rd - r_) < 0.24)
                let band =
                    dmath::cos(np::clip((rd - r_) / 0.16, -1.5, 1.5) * dmath::PI / 3.0) * gate((rd - r_).abs() < 0.24);
                add_ += band * (1.0 - u / 1.6) * (0.75 - 0.12 * j);
                dark += np::clip(1.0 - (rd - (r_ - 0.12)).abs() / 0.08, 0.0, 1.0) * (1.0 - u / 1.6) * 0.35;
            }
        }
    }
    (q3(np::clip(add_, 0.0, 1.2), b), q3(np::clip(dark, 0.0, 0.8), b))
}

/// `rings(base_)` of `Look.setup`: the VS card's panel, the bell's mouth head-on as rings of brass round the
/// fighter's ball `b`.
fn title_rings(base: &mut RgbF, cx: f64, cy: f64) {
    // np.array([255, 214, 120]) * 0.28
    let gold = PAH_COL.map(|v| v * 0.28);
    for y in 0..base.h {
        for x in 0..base.w {
            // rr = np.hypot(TT.XX - (b['cx'] - 8.0), TT.YY - (b['cy'] - 12.0)); s_ = (rr / 24.0) % 1.0
            let rr = np::hypot(x as f64 - (cx - 8.0), y as f64 - (cy - 12.0));
            let s_ = py::fmod_py(rr / 24.0, 1.0);
            let band = if s_ < 0.5 { 1.05 } else { 0.90 };
            let lip = (s_ - 0.5).abs() < 0.04;
            for (v, &g) in base.px_mut(x, y).iter_mut().zip(&gold) {
                let b = *v * band;
                *v = if lip { b * 0.72 + g } else { b };
            }
        }
    }
}

/// `held_phase`'s three phases.
#[derive(Clone, Copy, Debug, PartialEq)]
enum HeldPhase {
    /// Gulped into the bell (0.14 s).
    In(f64),
    /// Inside.
    Held,
    /// Popping back out (0.12 s).
    Out(f64),
}

// ---------------------------------------------------------------- the look
/// TUBA's look: `Look` of `kit/fighters/tuba.py`.
#[derive(Clone, Debug, Default)]
pub struct TubaLook {
    /// Its side.
    k: usize,
    /// `OOMS`.
    ooms: Vec<Oom>,
    /// `PAHS`.
    pahs: Vec<Pah>,
    /// `HELD`.
    held: Vec<Held>,
    /// `SUBS`.
    subs: Vec<Sub>,
    /// `self.cfg['abilities']['low']['radius']`: SUBSONIC's radius, `None` for a low ability without one.
    low_radius: Option<f64>,
    /// `self.cfg['abilities']['unlock2']['kind']`.
    unlock2_kind: String,
    /// `ab[slot]['dur_beats']` of the slot that holds the shield (`unlock2` when its kind is `shield`, else
    /// `unlock1`), `None` when that ability has none.
    shield_beats: Option<f64>,
}

impl TubaLook {
    /// A look not yet set up.
    pub fn new() -> TubaLook {
        TubaLook::default()
    }

    /// `held_phase(kk, t)`: the phase the gulped enemy is in at `t` (the swallower is always this look's side, so
    /// its own ball has none).
    fn held_phase(&self, kk: usize, t: f64) -> Option<HeldPhase> {
        if kk == self.k {
            return None;
        }
        for h in &self.held {
            let (tg, tsp) = (h.t_gulp, h.t_spit);
            if tg <= t && t < tg + 0.14 {
                return Some(HeldPhase::In((t - tg) / 0.14));
            }
            if tg + 0.14 <= t && t < tsp {
                return Some(HeldPhase::Held);
            }
            if tsp <= t && t < tsp + 0.12 {
                return Some(HeldPhase::Out((t - tsp) / 0.12));
            }
        }
        None
    }

    /// `holding(t)`: how full the tuba ball looks (0 to 1): it bulges while the enemy is inside.
    fn holding(&self, t: f64) -> f64 {
        for h in &self.held {
            let (tg, tsp) = (h.t_gulp, h.t_spit);
            if tg <= t && t < tsp + 0.1 {
                return ss(tg, tg + 0.12, t) * (1.0 - ss(tsp, tsp + 0.1, t));
            }
        }
        0.0
    }

    /// `self.OOMS[np.searchsorted(self.OOM_T, t, 'right') - 1]`: the last oom not after `t`.
    fn oom_at(&self, t: f64) -> Option<&Oom> {
        let after = self.ooms.partition_point(|o| o.t0 <= t);
        after.checked_sub(1).map(|j| &self.ooms[j])
    }

    /// `bell_state(t)`: `(open_k, flare, glow)` of the bell: it opens while it inhales, flares on the pah (more
    /// with a spit).
    fn bell_state(&self, t: f64) -> (f64, f64, f64) {
        let (mut open_k, mut flare) = (0.0, 0.0);
        if let Some(o) = self.oom_at(t) {
            open_k = ss(o.t0 - 0.02, o.t0 + 0.08, t) * (1.0 - ss(o.t_in, o.t_in + 0.15, t));
        }
        for q in &self.pahs {
            if 0.0 <= t - q.t && t - q.t < 0.3 {
                let punch = if q.caught != 0 { 1.0 } else { 0.55 };
                flare = py_max(flare, dmath::pow(1.0 - (t - q.t) / 0.3, 1.5) * punch);
            }
        }
        (open_k, flare, 0.6 * open_k + 0.8 * flare)
    }

    /// `inhaling(t)`: `(reach, strength)` while the bell inhales.
    fn inhaling(&self, t: f64) -> Option<(f64, f64)> {
        let o = self.oom_at(t)?;
        if o.t0 <= t && t < o.t_in + 0.12 {
            return Some((o.reach, ss(o.t0, o.t0 + 0.1, t) * (1.0 - ss(o.t_in, o.t_in + 0.12, t))));
        }
        None
    }

    /// `mouth(t)`: the ball, the aim and the bell's mouth at `t`, from the recording.
    fn mouth(&self, p: &PictureCtx, t: f64) -> ([f64; 2], f64, [f64; 2]) {
        let i = p.sim_i(t);
        let at = p.run.p(i, self.k);
        let th = p.run.f("TH", i, self.k).unwrap_or(0.0);
        (at, th, mouth_of(at, th))
    }
}

impl Look for TubaLook {
    fn shards(&self) -> Vec<String> {
        // ['brass'] * 7 + ['tube'] * 4 + ['valve'] * 3
        [("brass", 7), ("tube", 4), ("valve", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_flash(&self) -> &[&str] {
        &["slam", "cannon"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "cannon"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "cannon"]
    }

    /// `self.title_pattern = rings`: rings of brass round its own ball on the card (`title.BALLS[k]`).
    ///
    /// The card's looks are never set up, so the side is read off the card: the right ball when the right name is
    /// TUBA's and the left is not, else the left (the look is only ever made for TUBA).
    fn title_pattern_with(&self, base: &mut RgbF, card: &Layout) {
        let right = card.words[2].text == "TUBA" && card.words[0].text != "TUBA";
        let b = card.balls[usize::from(right)];
        title_rings(base, b.cx, b.cy);
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `setup_as` took from `cfg`, by the keys `tuba.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.low.radius={:?}; abilities.unlock2.kind={}",
            self.low_radius, self.unlock2_kind
        ))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: SUBSONIC's radius, the kind of the second ability and
    /// the shield's length come from `cfg`, the events from side `k`. For its own side `cfg` is `p.cfg[k]`, and
    /// this is `setup`.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        self.ooms = state::ooms(ev, k);
        self.pahs = state::pahs(ev, k);
        self.held = state::held(ev, k, &self.pahs);
        self.subs = state::subs(ev, k);
        self.low_radius = cfg.low.get("radius");
        self.unlock2_kind = cfg.unlock2.kind.name().to_string();
        // slot = 'unlock2' if ab['unlock2']['kind'] == 'shield' else 'unlock1'
        let slot = if cfg.unlock2.kind == AbilityKind::Shield {
            &cfg.unlock2
        } else {
            &cfg.unlock1
        };
        self.shield_beats = slot.get("dur_beats");
    }

    // ------------------------------------------------------------ OOM-PAH SWALLOW
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if phase != Phase::Fight {
            return;
        }
        let k = self.k;
        let i = p.sim_i(py_min(t, p.t_ko));
        if kk == 1 - k {
            // swallowed by the bell
            match self.held_phase(kk, t) {
                Some(HeldPhase::Held) => st.vis = false,
                Some(HeldPhase::In(u)) => {
                    let pre = p.run.p(p.sim_i(t - u * 0.14 - 1.0 / f64::from(FPS)), kk);
                    let th_sw = p.run.f("TH", i, k).unwrap_or(0.0);
                    let mouth = mouth_of(p.run.p(i, k), th_sw);
                    // st['p'] = pre + (mouth - pre) * (u * u)
                    st.p = [
                        pre[0] + (mouth[0] - pre[0]) * (u * u),
                        pre[1] + (mouth[1] - pre[1]) * (u * u),
                    ];
                    st.scale = 1.0 - 0.75 * u;
                }
                Some(HeldPhase::Out(u)) => st.scale = 0.6 + 0.4 * u,
                None => {}
            }
        } else if kk == k {
            // the tuba ball bulges while it holds the enemy
            let hb = self.holding(t);
            if hb > 0.0 {
                st.scale = 1.0 + hb * (0.10 + 0.04 * dmath::sin(2.0 * dmath::PI * t * 7.0));
                // st['glow'] = max(st['glow'], 0.55 * hb): Python's max keeps the glow it has unless the new one
                // is larger, and the new one is a Python float
                if 0.55 * hb > st.glow {
                    st.glow = 0.55 * hb;
                    st.glow_np = false;
                }
            }
        }
    }

    /// Held inside the bell (the HP number shows again as the enemy pops back out at full size).
    fn hides_hp(&self, p: &PictureCtx, kk: usize, t: f64) -> bool {
        if kk != 1 - self.k || !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return false;
        }
        self.held_phase(kk, t) == Some(HeldPhase::Held)
    }

    // ------------------------------------------------------------ events
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "slam" => Some(2.5),
            "cannon" => Some(2.0),
            "spit" => Some(1.2),
            "gulp" => Some(0.8),
            _ => DefaultLook.shake(p, e),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "slam" | "wave" | "shockwave" | "cannon" | "gulp" => Some((14.0, 0.35)),
            _ => None,
        }
    }

    /// oom: the bell inhales, air streaks pour into it; pah: a blast of air (and the spat enemy).
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        let origin = up(0.0, 0.0, 0.0);
        for o in &self.ooms {
            if o.t0 > p.t_ko {
                break;
            }
            let (_, _, m) = self.mouth(p, o.t0);
            fx.fl
                .add(o.t0, up(m[0], m[1], 0.8), p.fl_col[k], 1.8, 0.45, 0.03, 0.45, 0.0);
            // rng = np.random.default_rng(int(t0 * 100))
            let mut rng = Generator::new((o.t0 * 100.0) as u64);
            for j in 0..4 {
                let tj = o.t0 + (o.t_in - o.t0) * f64::from(j) / 4.0;
                if tj > p.t_ko {
                    break;
                }
                let (at, th, m) = self.mouth(p, tj);
                let d = unit(th);
                let nrm = [-d[1], d[0]];
                for _ in 0..6 {
                    // src = p + d * (R + reach * rng.uniform(0.5, 1.0)) + nrm * rng.uniform(-1, 1) * reach * 0.6
                    let out = R + o.reach * rng.uniform(0.5, 1.0);
                    let across = rng.uniform(-1.0, 1.0);
                    let src = [
                        at[0] + d[0] * out + nrm[0] * across * o.reach * 0.6,
                        at[1] + d[1] * out + nrm[1] * across * o.reach * 0.6,
                    ];
                    let to = [m[0] - src[0], m[1] - src[1]];
                    let pos = up(src[0], src[1], R * rng.uniform(0.5, 1.4));
                    let emit = Emit {
                        n: Some(1),
                        dir: sub(up(to[0], to[1], 0.0), origin),
                        col: Some(STREAK_COL.to_vec()),
                        ..Emit::default()
                    };
                    fx.pa.emit(tj, pos, "streaks", &emit);
                }
            }
        }
        for q in &self.pahs {
            let tp = q.t;
            if tp > p.t_ko {
                break;
            }
            let (_, th, m) = self.mouth(p, tp);
            let pos = up(m[0], m[1], R);
            let d3 = sub(up(dmath::cos(th), dmath::sin(th), 0.0), origin);
            if q.caught != 0 {
                fx.bu.add(tp, pos, PAH_COL, 1.4, None, true);
                fx.fl.add(tp, pos, PAH_COL, 2.0, 0.9, 0.03, 0.3, 0.0);
                let puffs = Emit {
                    n: Some(12),
                    dir: d3,
                    ..Emit::default()
                };
                fx.pa.emit(tp, pos, "puffs", &puffs);
                fx.mk.add(
                    tp,
                    Mark {
                        size: 0.7,
                        life: 2.5,
                        seed: np::f64_to_i64(tp * 10.0).max(0) as u64,
                        strength: 0.6,
                        ..Mark::new(MarkKind::SmokeRing, m[0], m[1])
                    },
                );
                // the cannonball's trail, in the enemy's colour
                let vic = 1 - k;
                // [tuple(int(v) for v in P.COL[vic] * 0.5 + 120)]
                let trail = p.col[vic].map(|v| (v * 0.5 + 120.0).trunc());
                for j in 0..15 {
                    let tj = tp + f64::from(j) / f64::from(FPS);
                    if tj > p.t_ko {
                        break;
                    }
                    let at = p.run.p(p.sim_i(tj), vic);
                    let emit = Emit {
                        n: Some(1),
                        dir: [-d3[0], -d3[1], -d3[2]],
                        speed: Some((0.3, 0.6)),
                        col: Some(vec![trail]),
                        ..Emit::default()
                    };
                    fx.pa.emit(tj, up(at[0], at[1], R), "streaks", &emit);
                }
            } else {
                let puffs = Emit {
                    n: Some(5),
                    dir: d3,
                    ..Emit::default()
                };
                fx.pa.emit(tp, pos, "puffs", &puffs);
                fx.fl.add(tp, pos, PAH_COL, 1.4, 0.35, 0.03, 0.25, 0.0);
            }
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let t = e.t;
        let brass = |n: i64| Emit {
            n: Some(n),
            ..Emit::default()
        };
        if e.kind == "hit" {
            let Some(att) = e.who().filter(|&a| a < 2) else { return };
            let (x, z) = (e.f(5), e.f(6));
            let pos = up(x, z, R);
            match how(e) {
                "gulp" => {
                    // the bell swallows
                    fx.bu.add(t, pos, p.col[att], 0.9, None, true);
                    fx.fl.add(t, pos, p.fl_col[att], 1.6, 0.7, 0.03, 0.3, 0.0);
                    let emit = Emit {
                        n: Some(8),
                        col: Some(vec![STREAK_COL[0]]),
                        ..Emit::default()
                    };
                    fx.pa.emit(t, pos, "streaks", &emit);
                }
                "spit" => fx.pa.emit(t, pos, "brass", &brass(6)),
                "cannon" => {
                    // spat into the ropes
                    fx.bu.add(t, pos, [255.0, 236.0, 200.0], 1.8, None, true);
                    fx.fl.add(t, pos, [255.0, 214.0, 140.0], 2.2, 1.1, 0.03, 0.35, 0.0);
                    fx.pa.emit(t, pos, "brass", &brass(22));
                    fx.mk.add(
                        t,
                        Mark {
                            size: 0.6,
                            n: 5,
                            life: 4.0,
                            seed: np::f64_to_i64(t * 100.0).max(0) as u64,
                            ..Mark::new(MarkKind::Crack, x, z)
                        },
                    );
                }
                // SUBSONIC lands on the enemy
                "slam" => fx.bu.add(t, pos, p.col[att], 1.6, None, true),
                _ => {}
            }
        } else if e.kind == "block" {
            // a hit lands on BRASS WALL: brass chips
            fx.pa.emit(t, up(e.f(4), e.f(5), R), "brass", &brass(5));
        }
    }

    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {
        // SUBSONIC: the floor ripples like water
        for s in &self.subs {
            if s.t_slam > p.t_ko {
                continue;
            }
            fx.fl.add(
                s.t_slam,
                up(s.cx, s.cz, 1.0),
                p.fl_col[self.k],
                2.6,
                1.0,
                0.03,
                0.6,
                0.0,
            );
            fx.mk.add(
                s.t_slam,
                Mark {
                    size: 1.8,
                    life: 5.0,
                    seed: np::f64_to_i64(s.t_slam * 10.0).max(0) as u64,
                    strength: 0.8,
                    ..Mark::new(MarkKind::Water, s.cx, s.cz)
                },
            );
            let foam = Emit {
                n: Some(16),
                spread: Some(180.0),
                col: Some(vec![[200.0, 196.0, 255.0], [236.0, 236.0, 255.0]]),
                ..Emit::default()
            };
            fx.pa.emit(s.t_slam, up(s.cx, s.cz, 0.05), "foam", &foam);
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let emit = Emit {
            n: Some(30),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "brass", &emit);
    }

    // ------------------------------------------------------------ canvas
    /// The suction cone while the bell inhales: under every other look's decals.
    fn canvas_first(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let st = &c.fs[self.k];
        if !(c.in_fight && st.vis && c.t < p.t_ko) {
            return;
        }
        let Some((reach, kk)) = self.inhaling(c.t) else { return };
        let tint = [0.85, 0.85, 1.2];
        for (j, light) in ic.iter_mut().enumerate() {
            let b = bayer(c.xs[j], c.ys[j]);
            let (br, dk) = inhale_light(c.sxp[j], c.szp[j], st.p, st.th, reach, kk, c.t, b);
            // Ic *= (1 - 0.45 * dk_)[:, None]; Ic += (br_ * 1.1)[:, None] * np.array([0.85, 0.85, 1.2])[None]
            let dim = 1.0 - 0.45 * dk;
            let lit = br * 1.1;
            for (v, &w) in light.iter_mut().zip(&tint) {
                *v *= dim;
                *v += lit * w;
            }
        }
    }

    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        // SUBSONIC: the floor ripples like water
        for s in &self.subs {
            if !(s.t0 <= c.t && c.t < s.t_slam + 1.6 && s.t0 < p.t_hs) {
                continue;
            }
            // rad = self.cfg['abilities']['low']['radius'] (Python raises for an ability without one)
            let Some(rad) = self.low_radius else { continue };
            // P.COL[owner] / 255.0 * 0.9 + 0.25
            let tint = p.col[self.k].map(|v| v / 255.0 * 0.9 + 0.25);
            for (j, light) in ic.iter_mut().enumerate() {
                let rd = np::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                let (add_, dark) = ripple_light(rd, c.t, s.t0, s.t_slam, rad, bayer(c.xs[j], c.ys[j]));
                let dim = 1.0 - dark * 0.6;
                for (v, &w) in light.iter_mut().zip(&tint) {
                    *v *= dim;
                    *v += add_ * w;
                }
            }
        }
    }

    // ------------------------------------------------------------ the dome, the bell, shards
    fn items_ball(&self, _p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        let mut out: Vec<(f64, Payload)> = Vec::new();
        if st.shield > 0.0 {
            let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
            let dome: Payload = Box::new("dome");
            out.push((dz - 0.05, dome));
        }
        if !st.shattered && st.scale > 0.5 {
            let (rings, _) = bell_rings(st.p, st.th, 0.0, 0.0);
            let bell: Payload = Box::new("bell");
            out.push((f.cam.depth(&[rings[4].0])[0], bell));
        }
        out
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let k = self.k;
        let st = &f.fs[k];
        let (cam, t) = (f.cam, f.t);
        if payload.downcast_ref::<&str>() == Some(&"dome") {
            // BRASS WALL: the bell becomes a brass dome
            // dur = ab[slot]['dur_beats'] * P.T.BEAT (Python raises for an ability without one)
            let Some(beats) = self.shield_beats else { return };
            let dur = beats * p.show.grid.beat;
            // k_ = min(1.0, st['shield'] / 0.2, (dur - st['shield']) / 0.12 + 0.2)
            let k_ = py_min(py_min(1.0, st.shield / 0.2), (dur - st.shield) / 0.12 + 0.2);
            let (mut arr, depth) = (fr.array(), fr.depth());
            draw_dome(&mut arr, cam, st.p[0], st.p[1], k_, t, p.col[k]);
            fr.set_array(&arr, Some(&depth));
        } else {
            let (ok, mut fl, gl) = if t < p.t_ko {
                self.bell_state(t)
            } else {
                (0.0, 0.0, 0.0)
            };
            if k == p.win && t >= p.t_hs {
                // the champion's solo: a pah on every solo note
                fl = 0.0;
                for &ts in &p.solo_t {
                    if 0.0 <= t - ts {
                        fl = py_max(fl, py_max(0.0, 1.0 - (t - ts) / 0.25) * 0.7);
                    }
                }
            }
            draw_tuba_bell(p, fr, cam, f.l, st.p, st.th, ok, fl, p.col[k], gl);
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
        curl: f64,
    ) {
        let (sx, sz) = (pos[0], pos[2]);
        let d = unit(rot);
        let nn = [-d[1], d[0]];
        let on_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            screen(cam, &world)
        };
        if kind == "tube" {
            // a curl of brass tubing
            let pts: Vec<[f64; 2]> = (0..7)
                .map(|j| {
                    let a = rot + f64::from(j) * 0.5 * (1.0 + curl);
                    [sx + 0.16 * dmath::cos(a), sz + 0.16 * dmath::sin(a)]
                })
                .collect();
            let sp = on_screen(&pts);
            // im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8)); the line; back as float32: the whole
            // picture is cut to whole numbers
            let bytes = arr.data.iter().map(|&v| np::clip_u8_f32(v)).collect();
            let mut im: Img<u8> = Img::from_vec(arr.w, arr.h, arr.c, bytes);
            pil::line(&mut im, &sp, &ink(SHARD_BRASS), 2);
            for (v, &b) in arr.data.iter_mut().zip(&im.data) {
                *v = f32::from(b);
            }
            return;
        }
        // a rotary valve (a squat disc), brass chips
        let (col, sz_) = match kind {
            "valve" => (SHARD_BRASS_DK.map(|v| v * 1.3), size * 0.8),
            "brass" => (SHARD_BRASS, size * 1.3),
            // (Python has no colour for another kind and raises; the look's `shards` names no other)
            _ => return,
        };
        let tri = [
            [sx + d[0] * sz_, sz + d[1] * sz_],
            [
                sx - d[0] * sz_ * 0.7 + nn[0] * sz_ * 0.8,
                sz - d[1] * sz_ * 0.7 + nn[1] * sz_ * 0.8,
            ],
            [
                sx - d[0] * sz_ * 0.4 - nn[0] * sz_ * 0.7,
                sz - d[1] * sz_ * 0.4 - nn[1] * sz_ * 0.7,
            ],
        ];
        let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
        let col = col.map(|c| np::clip(c * shade_k, 0.0, 255.0));
        layer_blend(arr, &[on_screen(&tri)], col, 1.0, None);
    }

    // ------------------------------------------------------------ lens
    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        for s in &self.subs {
            if s.t_slam <= t && t < s.t_slam + 1.0 && s.t_slam < p.t_hs {
                let u = t - s.t_slam;
                let c = scene::to_world(s.cx, s.cz, 0.05);
                img.shimmer_world(cam, c, 0.2 + 3.2 * u, 0.4, 3.0, t, 1.0 - u, 0.0);
            }
        }
        // the pah blast
        for q in &self.pahs {
            if q.caught != 0 && 0.0 <= t - q.t && t - q.t < 0.35 && q.t < p.t_ko {
                let (_, _, m) = self.mouth(p, q.t);
                let c = scene::to_world(m[0], m[1], R);
                let k_ = 1.0 - (t - q.t) / 0.35;
                img.shimmer_world(cam, c, 0.3 + 2.0 * (t - q.t), 0.25, 2.0, t, k_, 0.0);
            }
        }
    }

    // ------------------------------------------------------------ checks
    fn focus_points(&self, p: &PictureCtx, t: f64, st: &BallState) -> Vec<(V3, String)> {
        if p.show.grid.t_fight <= t && t < p.t_hs && st.vis && !st.shattered {
            let m = mouth_of(st.p, st.th);
            return vec![(scene::to_world(m[0], m[1], R), "bell".to_string())];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> TubaLook {
        TubaLook {
            k: 0,
            ooms: vec![
                Oom {
                    t0: 10.0,
                    t_in: 10.45,
                    t_pah: 10.9,
                    reach: 1.8,
                },
                Oom {
                    t0: 12.0,
                    t_in: 12.45,
                    t_pah: 12.9,
                    reach: 1.92,
                },
            ],
            pahs: vec![
                Pah {
                    t: 10.9,
                    caught: 1,
                    angle: 0.5,
                },
                Pah {
                    t: 12.9,
                    caught: 0,
                    angle: 0.7,
                },
            ],
            held: vec![Held {
                t_gulp: 10.2,
                t_spit: 10.9,
            }],
            ..TubaLook::default()
        }
    }

    /// `held_phase`: 0.14 s into the mouth, inside until the spit, 0.12 s back out; never the tuba's own ball.
    #[test]
    fn the_gulped_enemy_goes_in_rides_and_pops_out() {
        let l = look();
        assert_eq!(l.held_phase(1, 10.19), None);
        assert_eq!(l.held_phase(1, 10.2), Some(HeldPhase::In(0.0)));
        assert_eq!(l.held_phase(1, 10.27), Some(HeldPhase::In((10.27 - 10.2) / 0.14)));
        assert_eq!(l.held_phase(1, 10.5), Some(HeldPhase::Held));
        assert_eq!(l.held_phase(1, 10.9), Some(HeldPhase::Out(0.0)));
        assert_eq!(l.held_phase(1, 11.03), None);
        assert_eq!(l.held_phase(0, 10.5), None);
        // the tuba ball bulges from the gulp to just after the spit
        assert_eq!(l.holding(10.1), 0.0);
        assert_eq!(l.holding(10.5), 1.0);
        assert!(l.holding(10.95) > 0.0 && l.holding(10.95) < 1.0);
        assert_eq!(l.holding(11.0), 0.0);
    }

    /// `bell_state` and `inhaling`: the bell opens on the oom and shuts after the inhale; the pah flares it, more
    /// with a spit; before the first oom there is nothing.
    #[test]
    fn the_bell_opens_on_oom_and_flares_on_pah() {
        let l = look();
        assert_eq!(l.bell_state(9.0), (0.0, 0.0, 0.0));
        assert_eq!(l.inhaling(9.0), None);
        let (open_k, flare, glow) = l.bell_state(10.2);
        assert_eq!((open_k, flare, glow), (1.0, 0.0, 0.6));
        assert_eq!(l.inhaling(10.2), Some((1.8, 1.0)));
        // the spit: a full flare at the pah, the mouth shut again
        let (open_k, flare, _) = l.bell_state(10.9);
        assert_eq!((open_k, flare), (0.0, 1.0));
        // a pah with nothing caught flares 0.55
        assert_eq!(l.bell_state(12.9).1, 0.55);
        assert_eq!(l.inhaling(12.3).map(|v| v.0), Some(1.92));
        assert_eq!(l.inhaling(12.6), None);
    }

    /// The bell has five rings from the throat to the mouth, each of 14 points; the mouth widens with the inhale
    /// and the flare.
    #[test]
    fn the_bell_has_five_rings() {
        let (rings, axis) = bell_rings([0.3, -0.2], 0.7, 0.0, 0.0);
        assert_eq!(rings.map(|r| r.1), [0.13, 0.15, 0.2, 0.36 * 0.78, 0.36]);
        assert!((norm(axis) - 1.0).abs() < 1e-12);
        let (wide, _) = bell_rings([0.3, -0.2], 0.7, 1.0, 1.0);
        assert_eq!(wide[4].1, 0.36 + 0.22 + 0.16);
        let pts = ring_pts(rings[4].0, axis, rings[4].1, 14);
        assert_eq!(pts.len(), 14);
        for q in &pts {
            assert!((norm(sub(*q, rings[4].0)) - 0.36).abs() < 1e-9);
        }
    }

    /// The suction cone lights only the wedge in front of the bell, inside the reach.
    #[test]
    fn the_cone_is_a_wedge_in_front_of_the_bell() {
        let lit = |sx: f64, sz: f64| inhale_light(sx, sz, [0.0, 0.0], 0.0, 1.8, 1.0, 0.0, 0.5).1;
        assert!(lit(1.5, 0.0) > 0.0);
        assert_eq!(lit(-1.5, 0.0), 0.0, "behind the bell");
        assert_eq!(lit(0.3, 0.0), 0.0, "inside the ball");
        assert_eq!(lit(3.5, 0.0), 0.0, "past the reach");
        assert_eq!(lit(0.0, 1.5), 0.0, "outside the half-angle");
    }

    /// The card's rings: brighter and darker bands 24 px wide round a centre up and left of the ball, a gold lip
    /// between them.
    #[test]
    fn the_cards_rings_are_round_the_ball() {
        let mut base: RgbF = Img::filled(360, 640, &[100.0, 100.0, 100.0]);
        title_rings(&mut base, 78.0, 222.0);
        // the centre: s_ = 0, the bright band
        assert_eq!(base.px(70, 210), &[105.0, 105.0, 105.0]);
        // 18 px out: s_ = 0.75, the dark band
        assert_eq!(base.px(88, 210), &[90.0, 90.0, 90.0]);
        // 12 px out: s_ = 0.5, the lip (on the dark band)
        let lip: Vec<f64> = PAH_COL.iter().map(|v| 90.0 * 0.72 + v * 0.28).collect();
        assert_eq!(base.px(82, 210), &lip[..]);
    }
}
