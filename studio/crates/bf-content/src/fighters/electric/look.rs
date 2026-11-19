//! ELECTRIC's look: the port of `Look` in `projects/ballfight/kit/fighters/electric.py`, hook by hook, with the
//! module's draw helpers (`bolt_path`, `draw_bolt`, `bolt_branches`, `jag_ring`, `title_pattern`; `_mask`, `blend`,
//! `add` and `beam_lane` are `shared::reeds`').
//!
//! What it draws: thin jagged bolts with a white-hot core, a neon glow and a soft halo, re-drawn every frame (seeded
//! by the frame, so they flicker), side branches, a fork round the enemy under DISTORTION, a fizzle where a bolt
//! falls short; a post strike lights the post top and sparks race along its two ropes into it first; a lit lane on
//! the canvas under every bolt and a small scorch where it struck; DISTORTION crackles round its glass, TAPPING
//! flickers two hand-sparks on it. The smash: a jagged neon ring (a lit band on the canvas, an inked ring, heat
//! shimmer), splinters flying out, the glass and the instrument's parts tumbling out and snapping back in a beat, a
//! crack left on the canvas; its HP number hides while it lies in pieces. Its shards are the black body, the maple
//! neck, pickups and strings. Its VS-card panel is its own `title_pattern` (pickups under neon lightning).
//!
//! The kit class reads nothing from `self.cfg`, so `cfg_summary` is the default `None` and `setup_as` takes only
//! the side's events. The arithmetic keeps Python's order and dtypes: scalar maths through `bf_core::dmath`, a
//! 3-vector `np.linalg.norm` fused as numpy's dot is on this PC (ruling M3-R30), the row norms and sums of
//! `bolt_path` plain sums (numpy's `add.reduce`).

use bf_core::dmath::{self, PI};
use bf_core::event::Event;
use bf_core::py::{self, py_max, py_min};
use bf_core::rng::Generator;
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W, bayer_full, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::{Img, RgbF};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, BACK, BLUE, CORNERS, FRONT, POST_H, R_BALL as R, RED, ROPE_Y};
use bf_picture::title::layout;

use crate::fighters::shared::reeds::{self, INK, Num, Px};

/// `CORE`: the bolts' white-hot core.
const CORE: V3 = [246.0, 255.0, 238.0];
/// `SPARK_COLS`.
const SPARK_COLS: [V3; 3] = [[220.0, 255.0, 220.0], [120.0, 255.0, 140.0], [255.0, 255.0, 236.0]];
/// `POST_NAMES = ('red', 'front', 'blue', 'back')`: the engine's posts 0 to 3, as the renderer's corners.
const POST_CORNER: [usize; 4] = [RED, FRONT, BLUE, BACK];
/// `POST_SIDES`: the rope sides meeting at each post, by post (`red` (0, 3), `front` (0, 1), `blue` (1, 2), `back`
/// (2, 3)).
const POST_SIDES: [(usize, usize); 4] = [(0, 3), (0, 1), (1, 2), (2, 3)];
/// `BOLT_LIFE`: how long a bolt shows.
const BOLT_LIFE: f64 = 0.13;
/// `BODY`: the black body.
const BODY: V3 = [28.0, 26.0, 34.0];
/// `MAPLE`: the maple neck.
const MAPLE: V3 = [226.0, 196.0, 140.0];
/// `CHROME`.
const CHROME: V3 = [214.0, 218.0, 228.0];

// ---------------------------------------------------------------- lightning
/// `a @ b` of two 3-vectors, as numpy's dot is on the reference PC (fused, ruling M3-R30).
fn dot3(a: V3, b: V3) -> f64 {
    dmath::fma(a[2], b[2], dmath::fma(a[1], b[1], a[0] * b[0]))
}

/// `np.linalg.norm(x)` of one 3-vector: the square root of the fused `x @ x`.
fn norm3(x: V3) -> f64 {
    dot3(x, x).sqrt()
}

/// `a - b` of two 3-vectors.
fn sub3(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `bolt_path(p0, p1, seed, jag=0.16, levels=4)`: a jagged path between two ring points `(sx, sz, height)`:
/// midpoint displacement, seeded per frame (it flickers).
fn bolt_path(p0: V3, p1: V3, seed: u64, jag: f64, levels: u32) -> Vec<V3> {
    let mut rng = Generator::new(seed);
    let mut pts = vec![p0, p1];
    // amp = jag * float(np.linalg.norm(P[1] - P[0]))
    let amp = jag * norm3(sub3(p1, p0));
    for lv in 0..levels {
        let m = pts.len() - 1;
        // r = rng.normal(0, 1, mids.shape): row by row
        let mut r = vec![0.0f64; m * 3];
        rng.fill_normal(0.0, 1.0, &mut r);
        // 0.5 ** lv: a power of two, exact
        let scale = bf_core::dmath::pow(0.5, lv as f64);
        let mut mids = Vec::with_capacity(m);
        for i in 0..m {
            let (a, b) = (pts[i], pts[i + 1]);
            let mid: V3 = std::array::from_fn(|c| (a[c] + b[c]) / 2.0);
            let d = sub3(b, a);
            // np.linalg.norm(d, axis=1, keepdims=True): sqrt of add.reduce of the squares (a plain sum from 0.0)
            let nd = (0.0 + ((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2])).sqrt() + 1e-9;
            let dn: V3 = std::array::from_fn(|c| d[c] / nd);
            let ri = &mut r[i * 3..i * 3 + 3];
            // r -= (r * dn).sum(1, keepdims=True) * dn
            let s = 0.0 + ((ri[0] * dn[0] + ri[1] * dn[1]) + ri[2] * dn[2]);
            for c in 0..3 {
                ri[c] -= s * dn[c];
            }
            // r *= np.array([1.0, 1.0, 0.6])
            ri[2] *= 0.6;
            // mids = mids + r * amp * (0.5 ** lv)
            let ri: [f64; 3] = [ri[0], ri[1], ri[2]];
            mids.push(std::array::from_fn(|c| mid[c] + ri[c] * amp * scale));
        }
        let mut out = Vec::with_capacity(pts.len() + m);
        for i in 0..m {
            out.push(pts[i]);
            out.push(mids[i]);
        }
        out.push(pts[m]);
        pts = out;
    }
    pts
}

/// `_screen(cam, P)`: ring-frame points `(sx, sz, height)` on the screen.
fn on_screen(cam: &Cam, pts: &[V3]) -> Vec<(f64, f64)> {
    let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], q[2])).collect();
    cam.wp(&world)
}

/// `draw_bolt(img, cam, P, k, col, halo=5, glow=3, core=1)`: a bolt: a soft halo (additive), the neon glow, the
/// white-hot core.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_bolt(img: &mut Img<f32>, cam: &Cam, pts: &[V3], k: f64, col: V3, halo: u32, glow: u32, core: u32) {
    if k <= 0.02 || pts.len() < 2 {
        return;
    }
    let sp = on_screen(cam, pts);
    let lines = std::slice::from_ref(&sp);
    reeds::add(img, &reeds::mask(&[], lines, halo), col.map(|c| c * 0.5), 0.36 * k);
    let neon = col.map(|c| np::clip(c * 0.75 + 70.0, 0.0, 255.0));
    reeds::blend(img, &reeds::mask(&[], lines, glow), neon, Num::Py(0.85 * k));
    reeds::blend(img, &reeds::mask(&[], lines, core), CORE, Num::Py(k));
}

/// `bolt_branches(P, seed, n=2, frac=0.28)`: short side branches off the main path (each a smaller jagged path).
fn bolt_branches(pts: &[V3], seed: u64, n: usize, frac: f64) -> Vec<Vec<V3>> {
    let mut rng = Generator::new(seed + 7);
    let len = pts.len() as u64;
    // L = float(np.linalg.norm(P[-1] - P[0]))
    let l = norm3(sub3(pts[pts.len() - 1], pts[0]));
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        // i = int(rng.integers(len(P) // 5, max(len(P) // 5 + 1, 4 * len(P) // 5)))
        let lo = len / 5;
        let hi = (len / 5 + 1).max(4 * len / 5);
        let i = (lo + rng.integers(hi - lo)) as usize;
        // d = rng.normal(0, 1, 3) * np.array([1.0, 1.0, 0.5]); d /= np.linalg.norm(d) + 1e-9
        let mut d = [0.0f64; 3];
        rng.fill_normal(0.0, 1.0, &mut d);
        d[2] *= 0.5;
        let nd = norm3(d) + 1e-9;
        d = d.map(|c| c / nd);
        // P[i] + d * L * frac * rng.uniform(0.5, 1.0)
        let u = rng.uniform(0.5, 1.0);
        let end: V3 = std::array::from_fn(|c| pts[i][c] + d[c] * l * frac * u);
        out.push(bolt_path(pts[i], end, seed + 31 + i as u64, 0.2, 3));
    }
    out
}

/// `np.linspace(a, b, n)`: `i * step + a`, `step = (b - a) / (n - 1)`, the last value `b` itself.
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    let step = (b - a) / (n - 1) as f64;
    (0..n)
        .map(|i| if i == n - 1 { b } else { i as f64 * step + a })
        .collect()
}

/// `np.sign(x)`.
fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// `jag_ring(cx, cz, r, t, y=0.08, n=64, amp=0.10)`: a jagged ring of `n + 1` ring points (the first repeated).
fn jag_ring(cx: f64, cz: f64, r: f64, t: f64) -> Vec<V3> {
    let (y, n, amp) = (0.08, 64, 0.10);
    linspace(0.0, 2.0 * PI, n + 1)
        .into_iter()
        .map(|a| {
            // r + amp * np.sign(np.sin(a * 23 + t * 40)) * np.abs(np.sin(a * 7 + t * 13))
            let jag = r + amp * sign(dmath::sin(a * 23.0 + t * 40.0)) * dmath::sin(a * 7.0 + t * 13.0).abs();
            [cx + jag * dmath::cos(a), cz + jag * dmath::sin(a), y]
        })
        .collect()
}

// ---------------------------------------------------------------- the VS card panel
/// `title_pattern(base)`: ELECTRIC's VS-card panel: humbucker pickups in rows along the divider (black covers,
/// chrome pole pieces) under neon lightning. `base` is the panel's gradient, changed in place.
pub fn title_pattern(base: &mut RgbF) {
    let dd = layout::dd();
    let side = layout::side();
    let da = layout::DA;
    // np.array([20, 22, 26]) * 0.65, np.array([100, 255, 120]) * 0.4
    let cover = [20.0 * 0.65, 22.0 * 0.65, 26.0 * 0.65];
    let neon = [100.0 * 0.4, 255.0 * 0.4, 120.0 * 0.4];
    for y in 0..base.h {
        for x in 0..base.w {
            let u = (x as f64 - da[0]) * dd[0] + (y as f64 - da[1]) * dd[1];
            let av = side.px(x, y)[0].abs();
            let (cu, cv) = (py::fmod_py(u, 96.0) - 48.0, py::fmod_py(av, 110.0) - 55.0);
            let pick = cu.abs() < 30.0 && cv.abs() < 13.0;
            let pole = pick && (py::fmod_py(cu + 30.0, 10.0) - 5.0).abs() < 1.6 && (cv.abs() - 6.5).abs() < 1.6;
            // z = (|v| % 110) - 55 + 9 * |((u / 26) % 2) - 1| * 2 - 9: zig-zag bolts between the rows
            let z = py::fmod_py(av, 110.0) - 55.0 + 9.0 * (py::fmod_py(u / 26.0, 2.0) - 1.0).abs() * 2.0 - 9.0;
            let lit = py::fmod_py((u / 52.0).floor(), 3.0) != 1.0;
            let off = (z.abs() - 44.0).abs();
            let px = base.px_mut(x, y);
            if pick {
                for (ch, b) in px.iter_mut().enumerate() {
                    *b = *b * 0.35 + cover[ch];
                }
            }
            if pole {
                px.copy_from_slice(&[214.0, 220.0, 230.0]);
            }
            if off < 4.0 && lit {
                for (ch, b) in px.iter_mut().enumerate() {
                    *b = *b * 0.6 + neon[ch];
                }
            }
            if off < 1.4 && lit {
                px.copy_from_slice(&[236.0, 255.0, 230.0]);
            }
        }
    }
}

// ---------------------------------------------------------------- what the look keeps from the run
/// One `bolt` event of its own: `dict(i, t0, note, src, x0, y0, x1, y1, hit, riff, fork, tap)` (the drawing reads
/// all but `note`, `x0`, `y0` and `riff`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Bolt {
    /// The event's index in the run's events (`b['i']`, seeding the bolt's jags).
    i: usize,
    t0: f64,
    src: i64,
    x1: f64,
    y1: f64,
    hit: bool,
    fork: bool,
    tap: bool,
}

/// One splinter of a smash: `dict(j, ang, speed, t1, hit)` (the drawing reads `ang`, `speed`, `t1`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Splinter {
    ang: f64,
    speed: f64,
    /// When it ended (`splinter_end`), else the K.O.
    t1: f64,
}

/// One `smash` event of its own: `dict(t0, c, radius, dur, n, sp)` (the drawing reads `t0`, `c`, `dur`, `sp`).
#[derive(Clone, Debug, PartialEq)]
struct Smash {
    t0: f64,
    c: [f64; 2],
    dur: f64,
    sp: Vec<Splinter>,
}

/// The painter item of the smash: its pieces (`('pieces', (sm, u))`).
struct Pieces {
    /// The smash, an index into `SMASH`.
    sm: usize,
    /// How far through lying in pieces, 0 to 1.
    u: f64,
}

/// `st['glow'] = max(st['glow'], x)` with `x` a Python float: it is the glow when it is greater (and then a Python
/// float, ruling M3-R37; `glow_types.py` finds no numpy glow in ELECTRIC's left fight).
fn raise_glow(st: &mut BallState, x: f64) {
    if x > st.glow {
        st.glow = x;
        st.glow_np = false;
    }
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(bf_core::event::Value::Str(s))) => s,
        _ => "",
    }
}

/// `img[y, x] = img[y, x] * (1 - k) + col * k` for one pixel of the `float32` frame (a `float32` sample times a
/// Python float, plus a `float64` colour, stored back).
fn paint(img: &mut Img<f32>, x: usize, y: usize, col: V3, k: f64) {
    for (v, c) in img.px_mut(x, y).iter_mut().zip(col) {
        *v = f32::set(v.times(1.0 - k) + c * k);
    }
}

/// `arr[y, x] = col` on the `float32` frame.
fn img_set(arr: &mut Img<f32>, x: usize, y: usize, col: V3) {
    for (v, c) in arr.px_mut(x, y).iter_mut().zip(col) {
        *v = c as f32;
    }
}

// ---------------------------------------------------------------- the look
/// ELECTRIC's look: `Look` of `kit/fighters/electric.py`.
#[derive(Clone, Debug, Default)]
pub struct ElectricLook {
    /// Its side.
    k: usize,
    /// `BOLTS`.
    bolts: Vec<Bolt>,
    /// `BOLT_T`: their times.
    bolt_t: Vec<f64>,
    /// `FORK`: `(t0, t1)` of its DISTORTIONs.
    fork: Vec<(f64, f64)>,
    /// `TAP`: `(t0, t1)` of its TAPPINGs.
    tap: Vec<(f64, f64)>,
    /// `SMASH`.
    smash: Vec<Smash>,
    /// `post_w`: each post's top in the world (`S.to_world(*S.CORNERS[nm], S.POST_H + 0.06)`).
    post_w: [V3; 4],
}

impl ElectricLook {
    /// A look before `setup`.
    pub fn new() -> ElectricLook {
        ElectricLook::default()
    }

    /// `show(t)`: attacks show from FIGHT! to just after the shatter.
    fn show(&self, p: &PictureCtx, t: f64) -> bool {
        p.show.grid.t_fight <= t && t < p.t_hs + 0.12
    }

    /// `ta(t)`: the attack clock, frozen at the K.O.
    fn ta(&self, p: &PictureCtx, t: f64) -> f64 {
        py_min(t, p.t_ko)
    }

    /// `np.searchsorted(BOLT_T, x)`.
    fn left(&self, x: f64) -> usize {
        self.bolt_t.partition_point(|&b| b < x)
    }

    /// `np.searchsorted(BOLT_T, x, 'right')`.
    fn right(&self, x: f64) -> usize {
        self.bolt_t.partition_point(|&b| b <= x)
    }

    /// `live_bolts(F)`: `(bolt, strength)` of the bolts showing at `t` (the age is read by no caller).
    fn live_bolts(&self, p: &PictureCtx, t: f64) -> Vec<(Bolt, f64)> {
        let mut out = Vec::new();
        if self.show(p, t) && !self.bolt_t.is_empty() {
            let ta = self.ta(p, t);
            let (j0, j1) = (self.left(ta - BOLT_LIFE), self.right(ta));
            for b in self.bolts.get(j0..j1).unwrap_or(&[]) {
                let age = ta - b.t0;
                if (0.0..BOLT_LIFE).contains(&age) {
                    out.push((*b, dmath::pow(1.0 - age / BOLT_LIFE, 0.7)));
                }
            }
        }
        out
    }

    /// `bolt_ends(F, b)`: the bolt's two ends now (ring frame + height): from the headstock (following the ball)
    /// or a post top, to the enemy's glass (following it) or, for a miss, the spot it fizzled at.
    fn bolt_ends(&self, fs: &[BallState; 2], b: &Bolt) -> (V3, V3) {
        let me = fs[self.k].p;
        let en = fs[1 - self.k].p;
        let p0 = if b.src >= 0 {
            let pw = self.post_w[b.src as usize];
            let (sx, sz) = scene::to_ring(pw[0], pw[2]);
            [sx, sz, POST_H + 0.06]
        } else {
            let tgt = if b.hit { en } else { [b.x1, b.y1] };
            let d = [tgt[0] - me[0], tgt[1] - me[1]];
            let nd = np::hypot(d[0], d[1]) + 1e-9;
            let u = [d[0] / nd, d[1] / nd];
            [me[0] + u[0] * R * 0.85, me[1] + u[1] * R * 0.85, R * 1.25]
        };
        let p1 = if b.hit {
            let d = [en[0] - p0[0], en[1] - p0[1]];
            let nd = np::hypot(d[0], d[1]) + 1e-9;
            let u = [d[0] / nd, d[1] / nd];
            [en[0] - u[0] * R * 0.9, en[1] - u[1] * R * 0.9, R]
        } else {
            [b.x1, b.y1, R * 0.9]
        };
        (p0, p1)
    }

    /// `smash_at(t)`: `(smash, u 0..1)` while it lies in pieces (on the attack clock: held through the hit-stop),
    /// else `None`.
    fn smash_at(&self, p: &PictureCtx, t: f64) -> Option<(usize, f64)> {
        let tc = p.fx_clock(t);
        let i = self.smash.iter().position(|sm| sm.t0 <= tc && tc < sm.t0 + sm.dur)?;
        let sm = &self.smash[i];
        Some((i, (tc - sm.t0) / sm.dur))
    }

    /// `rope_sparks(img, F, b, age)`: a post strike: sparks race along the post's two ropes into it (from 0.10 s
    /// before the bolt).
    fn rope_sparks(&self, p: &PictureCtx, f: &FrameCtx<'_>, img: &mut Img<f32>, b: &Bolt, age: f64) {
        let post = b.src as usize;
        let lead = 0.10;
        let u = (age + lead) / (lead + 0.04);
        if !(0.0..=1.4).contains(&u) {
            return;
        }
        let k = 1.0 - py_max(0.0, u - 1.0) / 0.4;
        let col = p.col[self.k].map(|c| np::clip(c * 0.8 + 60.0, 0.0, 255.0));
        let c6 = col.map(|c| c * 0.6);
        let corner = CORNERS[POST_CORNER[post]];
        let (s0, s1) = POST_SIDES[post];
        for s in [s0, s1] {
            let (a, _) = p.side_ends(s);
            // at_post = 0 if np.allclose(a, S.CORNERS[nm]) else 1 (|a - c| <= 1e-8 + 1e-5 |c| for both)
            let close = (0..2).all(|i| (a[i] - corner[i]).abs() <= 1e-8 + 1e-5 * corner[i].abs());
            for y in [ROPE_Y[2], ROPE_Y[0]] {
                let mut pts = p.rope_points(s, y, f.t, 15);
                if close {
                    // run toward the post end
                    pts.reverse();
                }
                let last = pts.len() - 1;
                for j in 0..3 {
                    let fj = py_min(1.0, u) * (0.55 + 0.45 * py::fmod_py(f64::from(j) * 0.37 + b.i as f64 * 0.11, 1.0));
                    for tr in 0..3 {
                        let ff = py_max(0.0, fj - f64::from(tr) * 0.04);
                        let i = ff * last as f64;
                        let i0 = i as usize;
                        let i1 = (i0 + 1).min(last);
                        let fr = i - i0 as f64;
                        let w: V3 = std::array::from_fn(|c| pts[i0][c] * (1.0 - fr) + pts[i1][c] * fr);
                        let sc = f.cam.wp(&[w])[0];
                        let (xi, yi) = (py::round_half_even(sc.0) as i64, py::round_half_even(sc.1) as i64);
                        if 1 <= xi && xi < W as i64 - 1 && 1 <= yi && yi < H as i64 - 1 {
                            let (xi, yi) = (xi as usize, yi as usize);
                            let kk = k * (1.0 - 0.3 * f64::from(tr));
                            paint(img, xi, yi, if tr == 0 { CORE } else { col }, kk);
                            if tr == 0 {
                                for (x, y) in [(xi + 1, yi), (xi - 1, yi), (xi, yi + 1), (xi, yi - 1)] {
                                    // img[y, x] * (1 - 0.6 * kk) + col * 0.6 * kk
                                    for (v, c) in img.px_mut(x, y).iter_mut().zip(c6) {
                                        *v = f32::set(v.times(1.0 - 0.6 * kk) + c * kk);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Look for ElectricLook {
    /// `['body'] * 5 + ['neck'] * 3 + ['pickup'] * 3 + ['string'] * 4`.
    fn shards(&self) -> Vec<String> {
        [("body", 5), ("neck", 3), ("pickup", 3), ("string", 4)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["smash"]
    }

    fn spark_flash(&self) -> &[&str] {
        &["bolt"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["smash"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["smash"]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[]
    }

    fn title_pattern(&self, base: &mut RgbF) {
        title_pattern(base);
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the look reads no roster number, only side `k`'s events.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, _cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        let mine = |e: &Event, kind: &str| e.kind == kind && e.who() == Some(k);
        self.bolts = ev
            .iter()
            .enumerate()
            .filter(|(_, e)| mine(e, "bolt"))
            .map(|(i, e)| Bolt {
                i,
                t0: e.t,
                src: e.i(4),
                x1: e.f(7),
                y1: e.f(8),
                hit: e.at(9).truthy(),
                fork: e.at(11).truthy(),
                tap: e.at(12).truthy(),
            })
            .collect();
        self.bolt_t = self.bolts.iter().map(|b| b.t0).collect();
        self.fork = ev.iter().filter(|e| mine(e, "fork")).map(|e| (e.t, e.f(3))).collect();
        self.tap = ev
            .iter()
            .filter(|e| mine(e, "tapping"))
            .map(|e| (e.t, e.f(3)))
            .collect();
        self.smash = ev
            .iter()
            .filter(|e| mine(e, "smash"))
            .map(|e| {
                let sp = ev
                    .iter()
                    .filter(|f| mine(f, "splinter") && (f.t - e.t).abs() < 1e-6)
                    .map(|f| {
                        let j = f.i(3);
                        let end = ev.iter().find(|g| mine(g, "splinter_end") && g.i(3) == j && g.t >= f.t);
                        Splinter {
                            ang: f.f(4),
                            speed: f.f(5),
                            t1: end.map_or(p.t_ko, |g| g.t),
                        }
                    })
                    .collect();
                Smash {
                    t0: e.t,
                    c: [e.f(3), e.f(4)],
                    dur: e.f(6),
                    sp,
                }
            })
            .collect();
        self.post_w = POST_CORNER.map(|c| scene::to_world(CORNERS[c][0], CORNERS[c][1], POST_H + 0.06));
    }

    // ------------------------------------------------------------ state
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k || phase == Phase::Rev {
            return;
        }
        if self.smash_at(p, t).is_some() && (phase == Phase::Fight || t < p.t_hs + 0.6) {
            // in pieces (drawn by items_free)
            st.vis = false;
            return;
        }
        if phase != Phase::Fight {
            return;
        }
        let tc = self.ta(p, t);
        for sm in &self.smash {
            // snapped back together: a neon flash
            let te = sm.t0 + sm.dur;
            if te <= tc && tc < te + 0.3 {
                raise_glow(st, 1.0 - (tc - te) / 0.3);
            }
        }
        if !self.bolt_t.is_empty() {
            // every bolt lights the glass
            let j = self.right(tc) as i64 - 1;
            if j >= 0 {
                raise_glow(st, 0.4 * dmath::exp(-(tc - self.bolt_t[j as usize]) / 0.08));
            }
        }
    }

    fn hides_hp(&self, p: &PictureCtx, kk: usize, t: f64) -> bool {
        kk == self.k && self.smash_at(p, t).is_some()
    }

    // ------------------------------------------------------------ events
    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "smash" {
            return Some(2.5);
        }
        if e.kind == "hit" && how(e) == "bolt" {
            return Some(if e.f(8) > 0.0 { 0.4 } else { 0.25 });
        }
        // super().shake(e)
        if e.kind == "slam" { Some(1.6) } else { None }
    }

    fn spark(&self, _p: &PictureCtx, e: &Event) -> Spark {
        match how(e) {
            "smash" => ([140.0, 255.0, 160.0], 18, 3.4, 0.32),
            "splinter" => ([226.0, 196.0, 140.0], 6, 2.0, 0.2),
            _ => ([170.0, 255.0, 180.0], 6, 2.2, 0.16),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "smash" => Some((16.0, 0.4)),
            "bolt" => Some((11.0, 0.35)),
            _ => None,
        }
    }

    fn rope(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "smash" { Some(1.0) } else { None }
    }

    // ------------------------------------------------------------ effects
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        let sparks = |n: i64, speed: Option<(f64, f64)>| Emit {
            n: Some(n),
            col: Some(SPARK_COLS.to_vec()),
            speed,
            ..Emit::default()
        };
        for b in &self.bolts {
            if b.t0 > p.t_ko {
                break;
            }
            if b.src >= 0 {
                // the post lights up in neon
                fx.fl.add(
                    b.t0,
                    self.post_w[b.src as usize],
                    p.fl_col[k],
                    1.4,
                    0.55,
                    0.03,
                    0.14,
                    0.0,
                );
            }
            let end = up(b.x1, b.y1, R);
            if b.hit {
                let peak = if b.tap { 0.3 } else { 0.5 };
                fx.fl.add(b.t0, end, p.fl_col[k], 1.3, peak, 0.03, 0.14, 0.0);
                let scorch = Mark {
                    size: 0.16,
                    life: 2.4,
                    seed: (b.i % 997) as u64,
                    strength: 0.4,
                    ..Mark::new(MarkKind::Scorch, b.x1, b.y1)
                };
                fx.mk.add(b.t0, scorch);
            } else if !b.tap {
                fx.pa.emit(b.t0, end, "sparks", &sparks(3, Some((0.6, 1.4))));
            }
        }
        for &(t0, t1) in &self.fork {
            // DISTORTION crackles off the glass
            let mut tt = t0;
            while tt < py_min(t1, p.t_ko) {
                let q = p.run.p(p.sim_i(tt), k);
                fx.pa
                    .emit(tt, up(q[0], q[1], R * 1.2), "sparks", &sparks(2, Some((0.8, 1.8))));
                tt += 0.24;
            }
        }
        for sm in &self.smash {
            // the smash
            let pos = up(sm.c[0], sm.c[1], R * 0.6);
            fx.bu.add(sm.t0, pos, p.col[k], 2.0, None, true);
            fx.fl.add(sm.t0, pos, p.fl_col[k], 2.8, 1.1, 0.03, 0.45, 0.0);
            let burst = Emit {
                n: Some(30),
                col: Some(SPARK_COLS.to_vec()),
                spread: Some(180.0),
                speed: Some((1.5, 4.0)),
                ..Emit::default()
            };
            fx.pa.emit(sm.t0, pos, "sparks", &burst);
            let shards = Emit {
                n: Some(14),
                col: Some(vec![[28.0, 26.0, 34.0], [100.0, 255.0, 120.0], [226.0, 196.0, 140.0]]),
                ..Emit::default()
            };
            fx.pa.emit(sm.t0, pos, "shards", &shards);
            let crack = Mark {
                size: 0.9,
                life: 7.0,
                seed: 31,
                strength: 0.9,
                ..Mark::new(MarkKind::Crack, sm.c[0], sm.c[1])
            };
            fx.mk.add(sm.t0, crack);
            let scorch = Mark {
                size: 0.7,
                life: 6.0,
                seed: 32,
                strength: 0.7,
                ..Mark::new(MarkKind::Scorch, sm.c[0], sm.c[1])
            };
            fx.mk.add(sm.t0, scorch);
            // snapping back together
            let te = sm.t0 + sm.dur;
            fx.fl.add(te, pos, p.fl_col[k], 1.8, 0.6, 0.03, 0.25, 0.0);
            let motes = Emit {
                n: Some(8),
                col: Some(SPARK_COLS.to_vec()),
                ..Emit::default()
            };
            fx.pa.emit(te, pos, "motes", &motes);
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind != "hit" {
            return;
        }
        let att = e.who().filter(|&a| a < 2).unwrap_or(0);
        let pos = scene::to_world(e.f(5), e.f(6), R);
        match how(e) {
            "bolt" => {
                let lo = self.left(e.t).saturating_sub(2);
                let hi = self.right(e.t);
                let tap = self
                    .bolts
                    .get(lo..hi)
                    .unwrap_or(&[])
                    .iter()
                    .any(|b| (b.t0 - e.t).abs() < 1e-6 && b.tap);
                fx.bu.add(e.t, pos, p.col[att], if tap { 0.5 } else { 0.8 }, None, !tap);
                let sparks = Emit {
                    n: Some(if tap { 4 } else { 7 }),
                    col: Some(SPARK_COLS.to_vec()),
                    ..Emit::default()
                };
                fx.pa.emit(e.t, pos, "sparks", &sparks);
            }
            "smash" => {
                fx.bu.add(e.t, pos, p.col[att], 1.6, None, true);
                fx.fl.add(e.t, pos, p.fl_col[att], 2.0, 0.9, 0.03, 0.3, 0.0);
            }
            "splinter" => {
                fx.bu.add(e.t, pos, [226.0, 196.0, 140.0], 0.5, None, false);
                let chips = Emit {
                    n: Some(5),
                    ..Emit::default()
                };
                fx.pa.emit(e.t, pos, "chips", &chips);
            }
            _ => {}
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let sparks = Emit {
            n: Some(30),
            spread: Some(180.0),
            speed: Some((0.8, 2.6)),
            col: Some(SPARK_COLS.to_vec()),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
        let shards = Emit {
            n: Some(10),
            col: Some(vec![[28.0, 26.0, 34.0], [100.0, 255.0, 120.0], [214.0, 218.0, 228.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "shards", &shards);
    }

    // ------------------------------------------------------------ the canvas
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        // P.COL[self.k] / 255.0 * 0.9 + 0.15
        let col = p.col[self.k].map(|v| v / 255.0 * 0.9 + 0.15);
        for (b, k_) in self.live_bolts(p, c.t) {
            // a lit lane under every bolt
            let (p0, p1) = self.bolt_ends(c.fs, &b);
            let d = [p1[0] - p0[0], p1[1] - p0[1]];
            let ln = np::hypot(d[0], d[1]);
            if ln > 1e-6 {
                let aim = dmath::atan2(d[1], d[0]);
                let lane = reeds::beam_lane(c.sxp, c.szp, [p0[0], p0[1]], aim, ln, 0.06, c.ys, c.xs);
                let gain = if b.hit { 0.55 } else { 0.3 };
                for (light, lane) in ic.iter_mut().zip(lane) {
                    let v = lane * gain * k_;
                    for ch in 0..3 {
                        light[ch] += v * col[ch];
                    }
                }
            }
        }
    }

    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "smash" {
            return;
        }
        // the smash's shockwave: a jagged neon band
        let fade = dmath::pow(1.0 - w.r / w.rmax, 0.7);
        let tint = p.col[w.owner].map(|v| v / 255.0 * 0.8 + 0.2);
        let bayer = bayer_full();
        for (j, light) in ic.iter_mut().enumerate() {
            let (dx, dz) = (c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let rd = np::hypot(dx, dz);
            let ang = dmath::atan2(dz, dx);
            let jag = 0.09 * (py::fmod_py(ang * 14.0 / PI + c.t * 25.0, 2.0) - 1.0).abs();
            let band = np::clip(1.0 - (rd - w.r - jag).abs() / 0.22, 0.0, 1.0) * fade;
            let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
            let v = band * 1.15;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    // ------------------------------------------------------------ the painter pass: its pieces while smashed
    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let Some((sm, u)) = self.smash_at(p, f.t) else {
            return Vec::new();
        };
        let c = self.smash[sm].c;
        let depth = f.cam.depth(&[scene::to_world(c[0], c[1], R)])[0];
        let item: Payload = Box::new(Pieces { sm, u });
        vec![(depth, item)]
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(&Pieces { sm, u }) = payload.downcast_ref::<Pieces>() else {
            return;
        };
        let cam = f.cam;
        let (mut arr, depth) = (fr.array(), fr.depth());
        let smd = &self.smash[sm];
        let c = smd.c;
        // fly out, hang, snap back in
        let out = if u < 0.7 {
            dmath::sin(PI * py_min(1.0, u / 0.7))
        } else {
            0.0
        };
        let back = ss(0.55, 1.0, u);
        let rr = (0.95 * dmath::pow(py_min(1.0, u / 0.3), 0.5)) * (1.0 - back);
        // np.random.default_rng(int(sm['t0'] * 1000) % 99991)
        let mut rng = Generator::new(py::imod_py(np::f64_to_i64(smd.t0 * 1000.0), 99991) as u64);
        let mut parts: Vec<(&str, f64)> = vec![("glass", 0.14); 9];
        parts.extend([("body", 0.26), ("neck", 0.3), ("pickup", 0.12), ("pickup", 0.12)]);
        let n = parts.len() as f64;
        let col = p.col[self.k];
        // `self.draw_shard(...)` dispatches through the instance, which is the side's look in `P.LOOKS` (a subclass
        // or wrapper of it sees the call, as the export's spied looks do): the side's registered look when it is
        // this fighter's, this look itself when it was made for another side's fighter (EUPHONIUM's copy).
        let me: &dyn Look = match p.looks.get(self.k) {
            Some(l) if p.names[self.k] == super::NAME => &**l,
            _ => self,
        };
        for (j, &(kind, size)) in parts.iter().enumerate() {
            let a = 2.0 * PI * j as f64 / n + rng.uniform(-0.3, 0.3);
            let rad = rr * rng.uniform(0.6, 1.1);
            let h = R * (0.3 + 0.9 * out * rng.uniform(0.4, 1.0)) * (1.0 - back) + R * back;
            let q = [c[0] + rad * dmath::cos(a), c[1] + rad * dmath::sin(a), h];
            let rot = a + 8.0 * u * rng.uniform(-1.0, 1.0);
            if kind == "glass" {
                let d = [dmath::cos(rot), dmath::sin(rot)];
                let nn = [-d[1], d[0]];
                let tri: [[f64; 2]; 3] = [
                    std::array::from_fn(|i| q[i] + d[i] * size),
                    std::array::from_fn(|i| q[i] - d[i] * size * 0.6 + nn[i] * size * 0.7),
                    std::array::from_fn(|i| q[i] - d[i] * size * 0.5 - nn[i] * size * 0.6),
                ];
                let world: Vec<V3> = tri.iter().map(|v| scene::to_world(v[0], v[1], q[2])).collect();
                let sp = screen(cam, &world);
                layer_blend(&mut arr, &[sp], col.map(|v| v * 0.35 + 170.0), 0.85, None);
            } else {
                me.draw_shard(p, &mut arr, cam, kind, [q[0], q[2], q[1]], rot, size, 0.5);
            }
        }
        if back > 0.6 {
            // the snap: a neon ring closing on the ball
            let cs = cam.wp(&[scene::to_world(c[0], c[1], R)])[0];
            fr.glow(cs.0, cs.1, 18.0, col.map(|v| v * 0.8 + 50.0), 0.6 * (back - 0.6) / 0.4);
        }
        fr.set_array(&arr, Some(&depth));
    }

    // ------------------------------------------------------------ effects over the ring
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let (cam, k, t, fno) = (f.cam, self.k, f.t, f.f as u64);
        let col = p.col[k];
        // posts about to strike: the ropes buzz first
        if self.show(p, t) && !self.bolt_t.is_empty() {
            let ta = self.ta(p, t);
            let (j0, j1) = (self.left(ta - 0.05), self.right(ta + 0.11));
            for b in self.bolts.get(j0..j1).unwrap_or(&[]) {
                if b.src >= 0 {
                    self.rope_sparks(p, f, arr, b, ta - b.t0);
                }
            }
        }
        for (b, k_) in self.live_bolts(p, t) {
            let (p0, p1) = self.bolt_ends(f.fs, &b);
            let seed = b.i as u64 * 131 + fno * 7;
            let path = bolt_path(p0, p1, seed, 0.16, 4);
            if b.fork {
                // DISTORTION: the bolt forks round the enemy
                let mid = path[path.len() * 3 / 5];
                let d = [p1[0] - p0[0], p1[1] - p0[1]];
                let nd = np::hypot(d[0], d[1]) + 1e-9;
                let n = [-d[1] / nd, d[0] / nd];
                for sgn in [-1.0, 1.0] {
                    let q = [p1[0] + n[0] * sgn * 0.32, p1[1] + n[1] * sgn * 0.32, p1[2] + 0.12 * sgn];
                    // seed + 3 + sgn
                    let fork = bolt_path(mid, q, (seed as f64 + 3.0 + sgn) as u64, 0.22, 3);
                    draw_bolt(arr, cam, &fork, k_ * 0.8, col, 4, 2, 1);
                }
            }
            for br in bolt_branches(&path, seed, if b.tap { 1 } else { 2 }, 0.28) {
                draw_bolt(arr, cam, &br, k_ * 0.7, col, 4, 2, 1);
            }
            let (kk, halo, glow, core) = if b.tap { (0.8, 6, 3, 1) } else { (1.0, 8, 4, 2) };
            draw_bolt(arr, cam, &path, k_ * kk, col, halo, glow, core);
            if !b.hit {
                // a miss fizzles out
                for br in bolt_branches(&path[path.len() - 6..], seed + 5, 3, 0.9) {
                    draw_bolt(arr, cam, &br, k_ * 0.5, col, 3, 1, 1);
                }
            }
        }
        let smashed = self.smash_at(p, t);
        if f.in_fight || smashed.is_some() {
            if f.in_fight {
                for w in p.run.waves(f.i) {
                    if w.kind == "smash" && w.owner == k {
                        // the shockwave: an inked jagged neon ring
                        let kk = dmath::pow(1.0 - w.r / w.rmax, 0.7);
                        let ring = jag_ring(w.cx, w.cz, w.r, t);
                        let sp = on_screen(cam, &ring);
                        reeds::blend(
                            arr,
                            &reeds::mask(&[], std::slice::from_ref(&sp), 3),
                            INK,
                            Num::Py(0.55 * kk),
                        );
                        let c = col.map(|v| np::clip(v * 0.6 + 100.0, 0.0, 255.0));
                        reeds::blend(arr, &reeds::mask(&[], &[sp], 1), c, Num::Py(kk));
                    }
                }
            }
            if let Some((sm, _u)) = smashed {
                // the splinters flying out
                let smd = &self.smash[sm];
                let tc = p.fx_clock(t);
                let c1 = col.map(|v| np::clip(v * 0.5 + 90.0, 0.0, 255.0));
                let c2 = MAPLE.map(|v| v * 0.8);
                for s in &smd.sp {
                    if !(smd.t0 <= tc && tc < s.t1 + 0.05) {
                        continue;
                    }
                    let tau = py_min(tc, s.t1) - smd.t0;
                    let dv = [dmath::cos(s.ang), dmath::sin(s.ang)];
                    let q = [smd.c[0] + dv[0] * s.speed * tau, smd.c[1] + dv[1] * s.speed * tau];
                    let a = [q[0] - dv[0] * 0.22, q[1] - dv[1] * 0.22];
                    let seg = [[a[0], a[1], R * 0.7], [q[0], q[1], R * 0.75]];
                    let sp = on_screen(cam, &seg);
                    reeds::blend(arr, &reeds::mask(&[], std::slice::from_ref(&sp), 3), c1, Num::Py(0.45));
                    reeds::blend(arr, &reeds::mask(&[], &[sp], 2), c2, Num::Py(1.0));
                }
            }
        }
        // DISTORTION crackles round its glass; TAPPING flickers two hand-sparks on it
        let st = &f.fs[k];
        if self.show(p, t) && st.vis && t < p.t_ko {
            let ta = self.ta(p, t);
            let fork = self.fork.iter().any(|&(t0, t1)| t0 <= ta && ta < t1);
            let tap = self.tap.iter().any(|&(t0, t1)| t0 <= ta && ta < t1);
            let q = st.p;
            if fork {
                let mut rng = Generator::new(fno * 13 + 5);
                for j in 0..2u64 {
                    let a0 = rng.uniform(0.0, 2.0 * PI);
                    let arc: Vec<V3> = linspace(a0, a0 + 0.9, 4)
                        .into_iter()
                        .map(|a| {
                            let x = q[0] + R * 1.08 * dmath::cos(a);
                            let y = q[1] + R * 1.08 * dmath::sin(a);
                            [x, y, R * (1.0 + 0.5 * rng.uniform(-1.0, 1.0))]
                        })
                        .collect();
                    let path = bolt_path(arc[0], arc[arc.len() - 1], fno * 17 + j, 0.35, 2);
                    draw_bolt(arr, cam, &path, 0.7, col, 3, 2, 1);
                }
            }
            if tap {
                let s = cam.wp(&[scene::to_world(q[0], q[1], R * 1.3)])[0];
                for (j, dx) in [-4.0, 5.0].into_iter().enumerate() {
                    if (fno as usize + j).is_multiple_of(2) {
                        // int(s_[0] + dx), int(s_[1] - 3 + 2 * j): toward zero
                        let x_ = np::f64_to_i64(s.0 + dx);
                        let y_ = np::f64_to_i64(s.1 - 3.0 + 2.0 * j as f64);
                        if 1 <= x_ && x_ < W as i64 - 1 && 1 <= y_ && y_ < H as i64 - 1 {
                            let (x_, y_) = (x_ as usize, y_ as usize);
                            for yy in y_ - 1..y_ + 2 {
                                img_set(arr, x_, yy, CORE);
                            }
                            for xx in x_ - 1..x_ + 2 {
                                img_set(arr, xx, y_, CORE);
                            }
                        }
                    }
                }
            }
        }
    }

    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _drawn: &[DrawnBall]) {
        // the post tops glow neon as they strike
        let col = p.col[self.k].map(|v| v * 0.8 + 50.0);
        for (b, k_) in self.live_bolts(p, f.t) {
            let (p0, _) = self.bolt_ends(f.fs, &b);
            // where it leaves: the headstock / the post
            let s0 = f.cam.wp(&[scene::to_world(p0[0], p0[1], p0[2])])[0];
            let (r, a) = if b.src >= 0 { (11.0, 0.6) } else { (8.0, 0.4) };
            fr.glow(s0.0, s0.1, r, col, a * k_);
        }
    }

    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        if w.kind == "smash" {
            let c = scene::to_world(w.cx, w.cz, 0.05);
            img.shimmer_world(cam, c, w.r, 0.45, 3.0, t, 1.0 - w.r / w.rmax, w.owner as f64);
        }
    }

    // ------------------------------------------------------------ its parts (K.O. shatter + the smash)
    fn draw_shard(
        &self,
        p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        kind: &str,
        pos: V3,
        rot: f64,
        size: f64,
        curl: f64,
    ) {
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let c = [pos[0], pos[2]];
        let to_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            screen(cam, &world)
        };
        let quad = |ln: f64, wd: f64| -> Vec<[f64; 2]> {
            [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(a1, a2)| std::array::from_fn(|i| c[i] + d[i] * ln * a1 + nn[i] * wd * a2))
                .collect()
        };
        match kind {
            "string" => {
                // c + d * (j - 2) * 0.12 + nn * curl * 0.06 * math.sin(j * 1.7)
                let pts: Vec<[f64; 2]> = (0..5)
                    .map(|j| {
                        let (jj, s) = (f64::from(j - 2), dmath::sin(f64::from(j) * 1.7));
                        std::array::from_fn(|i| c[i] + d[i] * jj * 0.12 + nn[i] * curl * 0.06 * s)
                    })
                    .collect();
                let sp = to_screen(&pts);
                reeds::blend(arr, &reeds::mask(&[], &[sp], 1), CHROME, Num::Py(0.9));
            }
            "neck" => {
                let sp = to_screen(&quad(0.30, 0.05));
                let shade = 0.8 + 0.2 * dmath::cos(rot * 2.0);
                layer_blend(arr, &[sp], MAPLE.map(|v| v * shade), 1.0, None);
            }
            "pickup" => {
                let sp = to_screen(&quad(0.12, 0.05));
                layer_blend(arr, &[sp], BODY, 1.0, None);
                for j in [-1.0, 0.0, 1.0] {
                    // c + d * 0.06 * j
                    let q: [f64; 2] = std::array::from_fn(|i| c[i] + d[i] * 0.06 * j);
                    let s = cam.wp(&[scene::to_world(q[0], q[1], pos[1])])[0];
                    let (xi, yi) = (py::round_half_even(s.0) as i64, py::round_half_even(s.1) as i64);
                    if 0 <= xi && xi < W as i64 && 0 <= yi && yi < H as i64 {
                        img_set(arr, xi as usize, yi as usize, CHROME);
                    }
                }
            }
            _ => {
                // body: black chips with a neon edge
                let sz = size * 1.5;
                let tri: Vec<[f64; 2]> = vec![
                    std::array::from_fn(|i| c[i] + d[i] * sz),
                    std::array::from_fn(|i| c[i] - d[i] * sz * 0.7 + nn[i] * sz * 0.8),
                    std::array::from_fn(|i| c[i] - d[i] * sz * 0.4 - nn[i] * sz * 0.7),
                ];
                let sp = to_screen(&tri);
                let shade = 0.8 + 0.4 * dmath::cos(rot).abs();
                layer_blend(arr, std::slice::from_ref(&sp), BODY.map(|v| v * shade), 1.0, None);
                let mut closed = sp.clone();
                closed.push(sp[0]);
                let edge = p.col[self.k].map(|v| v * 0.8 + 40.0);
                reeds::blend(arr, &reeds::mask(&[], &[closed], 1), edge, Num::Py(0.7));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path of four levels from two points has 17 points, the ends kept; it is seeded.
    #[test]
    fn a_bolt_path_keeps_its_ends() {
        let (a, b) = ([0.0, 0.0, 0.5], [2.0, 1.0, 0.6]);
        let p = bolt_path(a, b, 12345, 0.16, 4);
        assert_eq!(p.len(), 17);
        assert_eq!((p[0], p[16]), (a, b));
        assert_eq!(bolt_path(a, b, 12345, 0.16, 4), p);
        assert_eq!(bolt_path(a, b, 7, 0.2, 3).len(), 9);
    }

    /// The jagged ring closes on itself (its first point repeated).
    #[test]
    fn the_jag_ring_has_65_points() {
        let r = jag_ring(0.5, -0.5, 1.2, 3.0);
        assert_eq!(r.len(), 65);
        assert!((r[0][0] - r[64][0]).abs() < 1e-12 && (r[0][1] - r[64][1]).abs() < 1e-12);
    }
}
