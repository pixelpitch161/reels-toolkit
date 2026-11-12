//! CLARINET's look: the `Look` half of `kit/fighters/clarinet.py` and its module helpers (`zip_path`, `draw_zip`,
//! `draw_poof`, `stun_stars`, `draw_squeak`, `squeak_light`), from #37's `styles.py` and `render.py`.
//!
//! The ball shrinks away in a lavender burst and reappears past the enemy, the zip line arcing like a leap (white
//! core, fading from the start) and lit on the canvas; lavender sparks and claw marks on a landed cut; SQUEAK's
//! jagged inked ring and the stunned enemy's circling stars; no speed lines across a teleport.
//!
//! The kit's look reads no roster number of its own (`self.cfg` is never read), so there is no `setup_as` and no
//! `cfg_summary` here: a copy of it drawn for another side is the same look.
//!
//! numpy's dtypes, kept: the frame of `over_ring` is `float32`, the picture after the grade either; every mask is
//! `float32`; every alpha is a Python float (the ages come from Python floats, the rings' radii from the recording's
//! lists), so the reeds helpers get [`Num::Py`]. A single pixel mixed in place (`img[y, x] = img[y, x] * (1 - k) +
//! col * k`) multiplies the `float32` pixel by the Python float in `float32`, adds the `float64` colour term in
//! `float64` and stores the sum back in the picture's dtype ([`mix`]).

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_min};
use bf_core::rng::Generator;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{FPS, H, W, bayer_full};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Phase, Pic, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{R_BALL as R, to_world};

use crate::fighters::shared::reeds::{self, INK, KO_CHIPS, LAV_SPARK, Num, Px, WHITE};

/// `np.linspace(a, b, n)`: `i * step + a`, the last value `b` itself.
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    let step = (b - a) / (n - 1) as f64;
    (0..n)
        .map(|i| if i == n - 1 { b } else { i as f64 * step + a })
        .collect()
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// `e[8]` of a hit, its sub-number (a cut's: 0 for a jump, 1 for a jump of the RHAPSODY chain).
fn sub(e: &Event) -> Option<i64> {
    match e.fields.get(6) {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    }
}

/// `e[2]` as a side, for `P.COL[e[2]]`: the event's fighter.
fn owner(e: &Event) -> usize {
    e.who().filter(|&k| k < 2).unwrap_or(0)
}

/// `img[y, x] = img[y, x] * keep + add` on one pixel of a `float32` or `float64` picture: `keep` a Python float (so a
/// `float32` pixel is multiplied in `float32`), `add` a `float64` colour term; the sum is stored in the picture's
/// dtype.
fn mix<T: Px>(px: &mut [T], keep: f64, add: V3) {
    for (v, a) in px.iter_mut().zip(add) {
        *v = T::set(v.times(keep) + a);
    }
}

// ---------------------------------------------------------------- TWELFTH JUMP (#37 styles.py)
/// `zip_path(p0, p1, y, n=14)`: the jump's path: a straight line in the ring plane that arcs up like a leap (higher
/// for longer jumps), as 14 points `[x, z, height]`.
pub fn zip_path(p0: [f64; 2], p1: [f64; 2], y: f64) -> Vec<V3> {
    let d = [p1[0] - p0[0], p1[1] - p0[1]];
    let hop = 0.18 + 0.09 * np::hypot(d[0], d[1]);
    linspace(0.0, 1.0, 14)
        .into_iter()
        .map(|u| [p0[0] + d[0] * u, p0[1] + d[1] * u, y + hop * dmath::sin(dmath::PI * u)])
        .collect()
}

/// A zip line showing in a frame (`live_zips`): `(p0, p1, age, hot)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Zip {
    p0: [f64; 2],
    p1: [f64; 2],
    age: f64,
    hot: f64,
}

/// `draw_zip(img, cam, p0, p1, age, col, y, life=0.34, hot=1.0)`: the zip line from where the clarinet was to where
/// it lands: a tapered lavender bolt with a white core, drawn the moment it jumps and fading over `life` seconds
/// (the tail end first).
fn draw_zip(img: &mut Img<f32>, cam: &Cam, z: &Zip, col: V3, y: f64) {
    let (life, hot) = (0.34, z.hot);
    if z.age < 0.0 || z.age >= life {
        return;
    }
    let k = 1.0 - z.age / life;
    let path = zip_path(z.p0, z.p1, y);
    let n = path.len();
    // tail = np.clip(np.linspace(0, 1, n) - (1 - k) * 0.9, 0, 1) ** 0.6: the start fades first
    let tail: Vec<f64> = linspace(0.0, 1.0, n)
        .into_iter()
        .map(|u| dmath::pow(np::clip(u - (1.0 - k) * 0.9, 0.0, 1.0), 0.6))
        .collect();
    let widths = |f: &dyn Fn(f64) -> f64| -> Vec<f64> { tail.iter().map(|&t| f(t)).collect() };
    // a luminous lavender halo
    let glow = reeds::ribbon(cam, &path, &widths(&|t| 0.26 * (0.4 + 0.6 * t) * (0.6 + 0.4 * hot)));
    reeds::add(
        img,
        &reeds::mask(&[glow], &[], 1),
        col.map(|c| c * 0.6 + 30.0),
        0.34 * k,
    );
    // ink edge: reads on the lilac canvas
    let ink = reeds::ribbon(cam, &path, &widths(&|t| 0.13 * (0.3 + 0.7 * t)));
    reeds::blend(img, &reeds::mask(&[ink], &[], 1), INK, Num::Py(0.75 * k));
    let mid = reeds::ribbon(cam, &path, &widths(&|t| 0.085 * (0.3 + 0.7 * t)));
    let core = col.map(|c| np::clip(c * 0.85 + 60.0, 0.0, 255.0));
    reeds::blend(img, &reeds::mask(&[mid], &[], 1), core, Num::Py(1.0 * k));
    // sp = cam.wp(S.to_world(P[:, 0], P[:, 1], P[:, 2])); the white core from sp[int(n * (1 - k) * 0.6):]
    let world: Vec<V3> = path.iter().map(|q| to_world(q[0], q[1], q[2])).collect();
    let sp = cam.wp(&world);
    let from = ((n as f64 * (1.0 - k) * 0.6) as usize).min(n);
    reeds::blend(
        img,
        &reeds::mask(&[], &[sp[from..].to_vec()], 2),
        WHITE,
        Num::Py(1.0 * k),
    );
}

/// What a vanish or appear burst is drawn with: `draw_poof`'s `life`, `r0`, `r1` and `seed` (`n` is always 10).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Poof {
    life: f64,
    r0: f64,
    r1: f64,
    seed: u64,
}

/// `draw_poof(img, cam, p, age, col, y, life=0.22, n=10, r0=0.25, r1=0.95, seed=0)`: a vanish or appear burst: a
/// ring of ten lavender sparks expanding from the spot (no flash, a few pixels each), its turn drawn from a
/// generator of its own.
fn draw_poof(img: &mut Img<f32>, cam: &Cam, p: [f64; 2], age: f64, col: V3, y: f64, poof: Poof) {
    let n = 10;
    if age < 0.0 || age >= poof.life {
        return;
    }
    let u = age / poof.life;
    // rr = r0 + (r1 - r0) * (1 - (1 - u) ** 2); k = (1 - u) ** 1.2
    let rr = poof.r0 + (poof.r1 - poof.r0) * (1.0 - (1.0 - u) * (1.0 - u));
    let k = dmath::pow(1.0 - u, 1.2);
    let mut rng = Generator::new(poof.seed);
    let ph = rng.uniform(0.0, 2.0 * dmath::PI);
    // a = ph + np.arange(n) * 2 * math.pi / n
    let world: Vec<V3> = (0..n)
        .map(|j| {
            let a = ph + f64::from(j * 2) * dmath::PI / f64::from(n);
            to_world(
                p[0] + rr * dmath::cos(a),
                p[1] + rr * dmath::sin(a),
                y + 0.25 * dmath::sin(a * 3.0 + ph) * u,
            )
        })
        .collect();
    let c = col.map(|v| np::clip(v * 0.8 + 50.0, 0.0, 255.0));
    let (ink, lav, white) = (INK.map(|v| v * 0.7 * k), c.map(|v| v * k), WHITE.map(|v| v * k));
    for (x, yy) in cam.wp(&world) {
        if x.is_nan() || yy.is_nan() {
            continue;
        }
        let (xi, yi) = (py::round_half_even(x), py::round_half_even(yy));
        if !(2.0 <= xi && xi < (W - 2) as f64 && 2.0 <= yi && yi < (H - 2) as f64) {
            continue;
        }
        let (xi, yi) = (xi as i64, yi as i64);
        let mut at = |dx: i64, dy: i64, keep: f64, add: V3| {
            mix(img.px_mut((xi + dx) as usize, (yi + dy) as usize), keep, add);
        };
        for (dx, dy) in [(2, 0), (-2, 0), (0, 2), (0, -2), (1, 1), (-1, 1), (1, -1), (-1, -1)] {
            at(dx, dy, 1.0 - 0.7 * k, ink);
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            at(dx, dy, 1.0 - k, lav);
        }
        at(0, 0, 1.0 - k, white);
    }
}

// ---------------------------------------------------------------- SQUEAK, stun (#37 styles.py)
/// `stun_stars(img, x, y, rp, t, k, col)`: stunned: three little inked stars circling above the ball, on the
/// picture after the grade (either dtype).
fn stun_stars<T: Px>(img: &mut Img<T>, d: &DrawnBall, t: f64, k: f64, col: V3) {
    if k <= 0.02 {
        return;
    }
    let ((x, y), rp) = (d.s, d.rp);
    let c = col.map(|v| np::clip(v * 0.5 + 140.0, 0.0, 255.0));
    // (dx, dy, cc, w_): an inked cross and corners, a lavender plus, a white heart
    let dots: [(i64, i64, V3, f64); 13] = [
        (0, -2, INK, 0.8),
        (0, 2, INK, 0.8),
        (-2, 0, INK, 0.8),
        (2, 0, INK, 0.8),
        (1, 1, INK, 0.5),
        (-1, 1, INK, 0.5),
        (1, -1, INK, 0.5),
        (-1, -1, INK, 0.5),
        (0, -1, c, 1.0),
        (0, 1, c, 1.0),
        (-1, 0, c, 1.0),
        (1, 0, c, 1.0),
        (0, 0, WHITE, 1.0),
    ];
    for j in 0..3 {
        // a = t * 7.0 + j * 2 * math.pi / 3
        let a = t * 7.0 + f64::from(j * 2) * dmath::PI / 3.0;
        let sx = x + rp * 0.95 * dmath::cos(a);
        let sy = y - rp * 1.1 + rp * 0.28 * dmath::sin(a);
        if sx.is_nan() || sy.is_nan() {
            continue;
        }
        let (xi, yi) = (py::round_half_even(sx) as i64, py::round_half_even(sy) as i64);
        for (dx, dy, cc, w) in dots {
            let (px, py_) = (xi + dx, yi + dy);
            if 0 <= px && px < W as i64 && 0 <= py_ && py_ < H as i64 {
                // img[..] = img[..] * (1 - k * w_) + cc * k * w_
                mix(
                    img.px_mut(px as usize, py_ as usize),
                    1.0 - k * w,
                    cc.map(|v| v * k * w),
                );
            }
        }
    }
}

/// `draw_squeak(img, cam, cx, cz, r, rmax, t, col, y=0.25)`: SQUEAK: the shrill burst as a jagged, inked white ring
/// expanding from the clarinet (a squeal drawn as a sawtooth).
fn draw_squeak(img: &mut Img<f32>, cam: &Cam, w: &Wave, col: V3) {
    let y = 0.25;
    let k = 1.0 - w.r / w.rmax;
    if k <= 0.02 {
        return;
    }
    // a = np.linspace(0, 2 * math.pi, 73); jag = r + 0.12 * np.where(np.arange(len(a)) % 2 == 0, 1.0, -0.6)
    let world: Vec<V3> = linspace(0.0, 2.0 * dmath::PI, 73)
        .into_iter()
        .enumerate()
        .map(|(i, a)| {
            let jag = w.r + 0.12 * if i % 2 == 0 { 1.0 } else { -0.6 };
            to_world(w.cx + jag * dmath::cos(a), w.cz + jag * dmath::sin(a), y)
        })
        .collect();
    let ring = cam.wp(&world);
    reeds::blend(
        img,
        &reeds::mask(&[], std::slice::from_ref(&ring), 3),
        INK,
        Num::Py(0.6 * k),
    );
    let white = [0, 1, 2].map(|c| WHITE[c] * 0.7 + col[c] * 0.3);
    reeds::blend(img, &reeds::mask(&[], &[ring], 1), white, Num::Py(1.0 * k));
}

/// `squeak_light(sxp, szp, cx, cz, r, rmax, t)` for one canvas pixel: SQUEAK's shrill ring on the canvas: a jagged,
/// high-frequency band (a squeal drawn as a sawtooth front).
pub fn squeak_light(sx: f64, sz: f64, w: &Wave, t: f64) -> f64 {
    let (dx, dz) = (sx - w.cx, sz - w.cz);
    let rd = np::hypot(dx, dz);
    let ang = dmath::atan2(dz, dx);
    // jag = 0.07 * np.abs(((ang * 18 / math.pi + t * 30) % 2) - 1)
    let jag = 0.07 * (py::fmod_py(ang * 18.0 / dmath::PI + t * 30.0, 2.0) - 1.0).abs();
    let band = np::clip(1.0 - (rd - w.r - jag).abs() / 0.08, 0.0, 1.0);
    band * (1.0 - w.r / w.rmax)
}

// ---------------------------------------------------------------- the look
/// One of its jumps (`JUMPS`): a `jump` event of its own.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Jump {
    /// When.
    t: f64,
    /// Where it was.
    p0: [f64; 2],
    /// Where it lands.
    p1: [f64; 2],
    /// 1 when the zip line cut the enemy.
    hit: i64,
    /// 0 for a jump, 1 for a jump of the RHAPSODY chain.
    kind: i64,
}

/// The CLARINET's look: `Look` of `kit/fighters/clarinet.py`.
#[derive(Clone, Debug, Default)]
pub struct ClarinetLook {
    /// Its side.
    k: usize,
    /// `JUMPS`: its jumps, in event order.
    jumps: Vec<Jump>,
    /// `STUNS`: `(t, victim, dur)` of the stuns its rings left.
    stuns: Vec<(f64, usize, f64)>,
}

impl ClarinetLook {
    /// `jump_gap(j)`: the time to its next jump (CLARION halves it): the vanish and appear windows shrink to fit.
    fn jump_gap(&self, j: usize) -> f64 {
        match self.jumps.get(j + 1) {
            Some(next) => next.t - self.jumps[j].t,
            None => 1.0,
        }
    }

    /// `jump_phase(t)` at a time before `t_ko`: `(scale, phase)`: the ball shrinks away over the last frames before
    /// a jump (`Some(false)`, Python's `'out'`) and grows back in after it (`Some(true)`, `'in'`).
    fn jump_phase(&self, t_ko: f64, t: f64) -> (f64, Option<bool>) {
        let ts = &self.jumps;
        if ts.is_empty() || t >= t_ko {
            return (1.0, None);
        }
        // j = int(np.searchsorted(ts, t, 'right')): the next jump's index
        let j = ts.partition_point(|x| x.t <= t);
        let (mut sc, mut ph) = (1.0, None);
        let fr = 1.0 / f64::from(FPS);
        if j < ts.len() {
            let win = 2.0 * fr * py_min(1.0, self.jump_gap(j.saturating_sub(1)) / 0.4);
            if ts[j].t - win <= t && t < ts[j].t {
                let u = (t - (ts[j].t - win)) / win;
                (sc, ph) = (1.0 - 0.5 * u, Some(false));
            }
        }
        if j > 0 {
            let win = 2.5 * fr * py_min(1.0, self.jump_gap(j - 1) / 0.4);
            if ts[j - 1].t <= t && t < ts[j - 1].t + win {
                let u = (t - ts[j - 1].t) / win;
                (sc, ph) = (0.5 + 0.5 * u, Some(true));
            }
        }
        (sc, ph)
    }

    /// `stun_at(kk, t)`: how strongly ball `kk` shows its stun at `t` (0 to 1: in over 0.05 s, out over 0.12 s).
    fn stun_at(&self, t_ko: f64, kk: usize, t: f64) -> f64 {
        for &(t0, v, dur) in &self.stuns {
            if v == kk && t0 <= t && t < t0 + dur && t0 < t_ko {
                // min(1.0, (t0 + dur - t) / 0.12, (t - t0) / 0.05)
                return py_min(py_min(1.0, (t0 + dur - t) / 0.12), (t - t0) / 0.05);
            }
        }
        0.0
    }

    /// `live_zips(F)`: its zip lines showing at `t` (each for 0.34 s from its jump, through the hit-stop).
    fn live_zips(&self, p: &PictureCtx, t: f64) -> Vec<Zip> {
        let mut out = Vec::new();
        if p.show.grid.t_fight <= t && t < p.t_hs {
            let tc = py_min(t, p.t_ko);
            for j in &self.jumps {
                if j.t <= tc && tc < j.t + 0.34 {
                    let hot = if j.kind == 1 { 1.0 } else { 0.6 + 0.4 * j.hit as f64 };
                    out.push(Zip {
                        p0: j.p0,
                        p1: j.p1,
                        age: tc - j.t,
                        hot,
                    });
                }
            }
        }
        out
    }
}

impl Look for ClarinetLook {
    fn shards(&self) -> Vec<String> {
        // ['wood'] * 6 + ['key'] * 8 + ['bell'] * 2
        [("wood", 6), ("key", 8), ("bell", 2)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["wave", "snip", "stun"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["wave", "snip"]
    }

    fn teleports(&self) -> bool {
        true
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[("wave", 1.0), ("shockwave", 0.5), ("stun", 0.5)]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.k = k;
        // (e[0], e[2], [e[3], e[4]], [e[5], e[6]], e[7], e[8]) for e in P.EV if e[1] == 'jump' and e[2] == k
        self.jumps = p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "jump" && e.who() == Some(k))
            .map(|e| Jump {
                t: e.t,
                p0: [e.f(3), e.f(4)],
                p1: [e.f(5), e.f(6)],
                hit: e.i(7),
                kind: e.i(8),
            })
            .collect();
        // (e[0], e[2], e[4]) for e in P.EV if e[1] == 'stun' and e[3] == k: t, victim, dur
        self.stuns = p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "stun" && e.i(3) == k as i64)
            .filter_map(|e| Some((e.t, e.who()?, e.f(4))))
            .collect();
    }

    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, gl: f64) {
        if phase == Phase::Fight {
            // its SQUEAK freezes the enemy
            st.extra.insert("stun".to_string(), self.stun_at(p.t_ko, kk, t));
            if kk == self.k {
                // st['scale'], st['jump'] = sc, ph (nothing reads `jump`, so it is not kept)
                let (sc, ph) = self.jump_phase(p.t_ko, t);
                st.scale = sc;
                if ph.is_some() {
                    st.glow = 0.6 * (1.0 - (sc - 0.75).abs() / 0.25);
                }
            }
        } else if phase == Phase::Tail && kk == self.k {
            reeds::tail_theta(p, kk, st, gl);
        }
    }

    // ---------------------------------------------------------------- events
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "cut" => Some(if sub(e) == Some(0) { 0.6 } else { 1.2 }),
            "stun" => Some(0.8),
            "wave" => Some(2.0),
            _ => DefaultLook.shake(p, e),
        }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[owner(e)].map(|c| c * 0.6 + 100.0);
        let what = how(e);
        if what == "cut" {
            (col, 10, 2.6, 0.24)
        } else if self.spark_big().contains(&what) {
            (col, 16, 3.2, 0.3)
        } else {
            (col, 6, 2.0, 0.2)
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "wave" | "snip" => Some((14.0, 0.35)),
            "cut" if sub(e) == Some(1) => Some((14.0, 0.35)),
            "cut" | "tune" | "stun" => Some((8.0, 0.2)),
            _ => None,
        }
    }

    fn fringe(&self, _p: &PictureCtx, e: &Event) -> bool {
        matches!(how(e), "wave" | "snip") || (how(e) == "cut" && sub(e) == Some(1))
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        // TWELFTH JUMP: lavender light at both ends, motes
        for j in &self.jumps {
            if j.t > p.t_ko {
                break;
            }
            let (a, b) = (up(j.p0[0], j.p0[1], R), up(j.p1[0], j.p1[1], R));
            let peak = if j.kind == 0 { 0.45 } else { 0.7 };
            fx.fl.add(j.t, b, p.col[self.k], 1.6, peak, 0.03, 0.22, 0.0);
            let motes = |n: i64| Emit {
                n: Some(n),
                col: Some(LAV_SPARK.to_vec()),
                ..Emit::default()
            };
            fx.pa.emit(j.t, a, "motes", &motes(4));
            fx.pa.emit(j.t, b, "motes", &motes(3));
            if j.hit != 0 {
                let d = [j.p1[0] - j.p0[0], j.p1[1] - j.p0[1]];
                // mid = (p0 + p1) / 2
                let mid = [(j.p0[0] + j.p1[0]) / 2.0, (j.p0[1] + j.p1[1]) / 2.0];
                fx.mk.add(
                    j.t,
                    Mark {
                        size: py_min(1.4, 0.35 * np::hypot(d[0], d[1])),
                        angle: dmath::atan2(d[1], d[0]),
                        col: [150.0, 130.0, 210.0],
                        life: 2.2,
                        n: 1,
                        // seed=int(tj * 100)
                        seed: np::f64_to_i64(j.t * 100.0).max(0) as u64,
                        strength: 0.55,
                        ..Mark::new(MarkKind::Claw, mid[0], mid[1])
                    },
                );
            }
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let t = e.t;
        if e.kind == "hit" {
            let att = owner(e);
            let pos = up(e.f(5), e.f(6), R);
            match how(e) {
                "cut" => {
                    // the zip line cuts: lavender sparks
                    let strength = if sub(e) == Some(0) { 0.7 } else { 1.1 };
                    fx.bu.add(t, pos, p.col[att], strength, None, true);
                    fx.fl.add(t, pos, p.col[att], 1.4, 0.6, 0.03, 0.22, 0.0);
                    let sparks = Emit {
                        n: Some(8),
                        col: Some(LAV_SPARK.to_vec()),
                        ..Emit::default()
                    };
                    fx.pa.emit(t, pos, "sparks", &sparks);
                }
                // SQUEAK lands
                "stun" => fx.bu.add(t, pos, [255.0, 250.0, 255.0], 0.8, None, true),
                "wave" => {
                    fx.bu.add(t, pos, p.col[att], 1.5, None, true);
                    fx.fl.add(t, pos, p.col[att], 2.2, 1.0, 0.03, 0.4, 0.0);
                }
                _ => {}
            }
        } else if e.kind == "ability" && matches!(e.fields.get(2), Some(Value::Str(s)) if s == "stun") {
            // SQUEAK: a shrill flare at the clarinet
            let at = p.run.p(p.sim_i(t), owner(e));
            let white = [255.0, 250.0, 255.0];
            fx.fl.add(t, up(at[0], at[1], R), white, 1.6, 0.6, 0.03, 0.2, 0.0);
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let chips = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(KO_CHIPS.to_vec()),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &chips);
    }

    // ---------------------------------------------------------------- canvas, effects over the ring
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        // the zip lines' lit paths
        let tint = p.col[self.k].map(|v| v / 255.0 + 0.2);
        for z in self.live_zips(p, c.t) {
            let d = [z.p1[0] - z.p0[0], z.p1[1] - z.p0[1]];
            let ln = np::hypot(d[0], d[1]);
            if ln > 1e-6 {
                let lane = reeds::beam_lane(c.sxp, c.szp, z.p0, dmath::atan2(d[1], d[0]), ln, 0.04, c.ys, c.xs);
                for (light, lane) in ic.iter_mut().zip(lane) {
                    // (lane * 0.45 * (1 - age / 0.34) * hot)[:, None] * (P.COL[self.k] / 255.0 + 0.2)[None]
                    let v = lane * 0.45 * (1.0 - z.age / 0.34) * z.hot;
                    for ch in 0..3 {
                        light[ch] += v * tint[ch];
                    }
                }
            }
        }
    }

    fn canvas_wave(&self, _p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "stun" {
            return;
        }
        // SQUEAK's shrill ring
        let bayer = bayer_full();
        let tint = [1.05, 1.0, 1.15];
        for (j, light) in ic.iter_mut().enumerate() {
            let band = squeak_light(c.sxp[j], c.szp[j], w, c.t);
            // band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
            let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
            let v = band * 1.1;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let col = p.col[self.k];
        for z in self.live_zips(p, f.t) {
            draw_zip(arr, f.cam, &z, col, R);
            // seed=abs(int(p0[0] * 1000)) + 1, seed=abs(int(p1[1] * 1000)) + 2
            let seed = |v: f64, n: u64| np::f64_to_i64(v * 1000.0).unsigned_abs() + n;
            let vanish = Poof {
                life: 0.22,
                r0: 0.25,
                r1: 0.95,
                seed: seed(z.p0[0], 1),
            };
            draw_poof(arr, f.cam, z.p0, z.age, col, R, vanish);
            let appear = Poof {
                life: 0.2,
                r0: 0.9,
                r1: 0.3,
                seed: seed(z.p1[1], 2),
            };
            draw_poof(arr, f.cam, z.p1, z.age - 0.02, col, R, appear);
        }
        if f.in_fight {
            for w in p.run.waves(f.i) {
                if w.kind == "stun" && w.owner == self.k {
                    draw_squeak(arr, f.cam, w, p.col[w.owner]);
                }
            }
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
        reeds::draw_woodwind_shard(arr, cam, kind, pos, rot, size);
    }

    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        let c = to_world(w.cx, w.cz, 0.05);
        let (width, amp) = if w.kind == "wave" { (0.4, 3.0) } else { (0.25, 2.0) };
        img.shimmer_world(cam, c, w.r, width, amp, t, 1.0 - w.r / w.rmax, w.owner as f64);
    }

    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        // the stunned enemy's stars
        let col = p.col[self.k];
        for d in drawn {
            let k = f.fs[d.k].extra.get("stun").copied().unwrap_or(0.0);
            match img {
                Pic::F32(a) => stun_stars(a, d, f.t, k, col),
                Pic::F64(a) => stun_stars(a, d, f.t, k, col),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(times: &[f64]) -> ClarinetLook {
        ClarinetLook {
            k: 0,
            jumps: times
                .iter()
                .map(|&t| Jump {
                    t,
                    p0: [0.0, 0.0],
                    p1: [1.0, 0.0],
                    hit: 0,
                    kind: 0,
                })
                .collect(),
            stuns: vec![(5.0, 1, 0.5)],
        }
    }

    #[test]
    fn the_gap_to_the_next_jump_is_a_second_after_the_last() {
        let l = look(&[4.0, 4.5, 4.75]);
        assert_eq!((l.jump_gap(0), l.jump_gap(1), l.jump_gap(2)), (0.5, 0.25, 1.0));
    }

    #[test]
    fn the_ball_shrinks_out_before_a_jump_and_grows_in_after() {
        let l = look(&[4.0, 5.0]);
        // well between two jumps: the whole ball
        assert_eq!(l.jump_phase(60.0, 4.5), (1.0, None));
        // one frame before the jump: half way out (the window is two frames)
        let (sc, ph) = l.jump_phase(60.0, 5.0 - 1.0 / 30.0);
        assert_eq!(ph, Some(false));
        assert!((sc - 0.75).abs() < 1e-9, "{sc}");
        // at the jump: back in from half size
        assert_eq!(l.jump_phase(60.0, 5.0), (0.5, Some(true)));
        // after the K.O. nothing
        assert_eq!(l.jump_phase(4.9, 5.0), (1.0, None));
        assert_eq!(look(&[]).jump_phase(60.0, 5.0), (1.0, None));
    }

    #[test]
    fn the_stars_fade_in_and_out() {
        let l = look(&[]);
        assert_eq!(l.stun_at(60.0, 1, 5.25), 1.0);
        assert_eq!(l.stun_at(60.0, 0, 5.25), 0.0);
        assert_eq!(l.stun_at(60.0, 1, 5.5), 0.0);
        assert!((l.stun_at(60.0, 1, 5.025) - 0.5).abs() < 1e-9);
        assert!((l.stun_at(60.0, 1, 5.44) - 0.5).abs() < 1e-9);
        // a stun that starts after the K.O. shows nothing
        assert_eq!(l.stun_at(4.0, 1, 5.25), 0.0);
    }

    #[test]
    fn the_path_is_a_leap_of_fourteen_points() {
        let path = zip_path([0.0, 0.0], [3.0, 4.0], 0.6);
        assert_eq!(path.len(), 14);
        assert_eq!(path[0], [0.0, 0.0, 0.6]);
        assert_eq!((path[13][0], path[13][1]), (3.0, 4.0));
        // hop = 0.18 + 0.09 * 5: the top of the leap is near the middle
        let top = path.iter().map(|q| q[2]).fold(0.0, f64::max);
        assert!((top - (0.6 + 0.63)).abs() < 0.01, "{top}");
    }

    #[test]
    fn a_mixed_pixel_follows_numpys_dtypes() {
        // float32 pixel times a Python float in float32, plus a float64 term, stored as float32
        let mut px = [100.5f32, 0.1, 255.0];
        mix(&mut px, 0.3, [7.0, 0.25, 1e-9]);
        let want = |v: f32, a: f64| (f64::from(v * 0.3f32) + a) as f32;
        assert_eq!(px, [want(100.5, 7.0), want(0.1, 0.25), want(255.0, 1e-9)]);
        let mut px = [100.5f64, 0.1, 255.0];
        mix(&mut px, 0.3, [7.0, 0.25, 1e-9]);
        assert_eq!(px, [100.5 * 0.3 + 7.0, 0.1 * 0.3 + 0.25, 255.0 * 0.3 + 1e-9]);
    }
}
