//! BASS CLARINET's look: the `Look` half of `kit/fighters/bass_clarinet.py` and its module helpers (`whirl_light`,
//! `peg_light`, `draw_arm`, `draw_ghost_ball`, `draw_slap`, `shadow_pool`, `erupt_light`), from #38's `styles.py`
//! and #40's `render.py`.
//!
//! The silver endpin rod from the ball down to the peg (black collar, a glint where it bites), the swept trail of
//! the whirl on the canvas and the peg's dent, MULTIPHONICS' translucent twin and its ghost rod, SLAP TONGUE's cane
//! paddle, LIGHTS OUT's shadow pool, the sinking ball (the renderer's `sink`), the eased arena dim with soft keep
//! circles round both balls, and the cracked eruption; grenadilla, key, neck and bell shards.
//!
//! Its roster numbers come from the `cfg` [`Look::setup_as`] is given (`self.cfg`): SLAP TONGUE's `reach`, LIGHTS
//! OUT's `radius` and whether the weapon is an anchor.
//!
//! `st['anc']` (the whirl at a frame: the peg, the angle, the radius, the twin, the age, the turn) is a tuple in
//! the kit; here [`Look::state`] keeps its numbers in the ball's `extra` (`anc`, `anc_cx`, ...), so the canvas, the
//! effects over the ring and the focus read the whirl of the frame they draw, as the kit's hooks read `st['anc']`.
//!
//! The kit's `after_grade` also keeps lit whatever another look asks with a `dim_keep(F)` hook. No kit look has one
//! and the renderer's `Look` has no such hook, so that loop is not here (it would change nothing in any kit fight).
//!
//! numpy's dtypes, kept: the frame of `over_ring` is `float32`, every mask `float32`, every alpha a Python float;
//! the keep circles of the dim are `float32` (`np.zeros((H, W), np.float32)`), so the dim factor is a `float32`
//! product with the Python-float depth. A pixel mixed in place (the peg's glint) is [`mix`].

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{FPS, H, W, bayer_full, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Fx, Look, Phase, Pic, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::{R_BALL as R, to_world};

use super::state::{self, Anch};
use crate::fighters::shared::reeds::{self, CANE, INK, Num, Px, WHITE};

/// `BLACK`: grenadilla.
const BLACK: V3 = [30.0, 26.0, 32.0];
/// `NICKEL` of #38's styles (the endpin rod).
const NICKEL: V3 = [214.0, 216.0, 228.0];
/// `WOOD`: grenadilla, the body's shards.
const WOOD: V3 = [34.0, 26.0, 30.0];
/// `SHARD_NICKEL` (#40's render `NICKEL`: keys, neck, bell).
const SHARD_NICKEL: V3 = [214.0, 216.0, 226.0];
/// `SLATE_SPARK`.
const SLATE_SPARK: [V3; 3] = [[196.0, 210.0, 236.0], [110.0, 132.0, 170.0], [240.0, 244.0, 255.0]];
/// `CHIPS`: the K.O. sparks.
const CHIPS: [V3; 3] = [[40.0, 34.0, 38.0], [214.0, 216.0, 226.0], [24.0, 20.0, 24.0]];
/// `DUST_PEG`.
const DUST_PEG: [V3; 2] = [[170.0, 160.0, 190.0], [130.0, 122.0, 150.0]];

/// `math.radians(200)`: CPython's `x * (pi / 180)`.
fn radians(x: f64) -> f64 {
    x * (dmath::PI / 180.0)
}

/// `np.linspace(0, b, n)`: `i * step`, the last value `b` itself.
fn linspace0(b: f64, n: usize) -> Vec<f64> {
    let step = b / (n - 1) as f64;
    (0..n).map(|i| if i == n - 1 { b } else { i as f64 * step }).collect()
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// `e[8]` of a hit, its sub-number (an anchor smash: 0 for the arm, 1 for the twin's).
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

/// `img[y, x] = img[y, x] * keep + add` on one pixel: `keep` a Python float (a `float32` pixel is multiplied in
/// `float32`), `add` a `float64` colour term, the sum stored in the picture's dtype.
fn mix<T: Px>(px: &mut [T], keep: f64, add: V3) {
    for (v, a) in px.iter_mut().zip(add) {
        *v = T::set(v.times(keep) + a);
    }
}

/// `wp(cam, x, z, y)`: ring-frame points to the screen (`cam.wp(S.to_world(x, z, y))`).
fn wp(cam: &Cam, pts: &[V3]) -> Vec<(f64, f64)> {
    let world: Vec<V3> = pts.iter().map(|q| to_world(q[0], q[1], q[2])).collect();
    cam.wp(&world)
}

/// `np.clip(col * a + b, 0, 255)`.
fn tone(col: V3, a: f64, b: f64) -> V3 {
    col.map(|c| np::clip(c * a + b, 0.0, 255.0))
}

// ---------------------------------------------------------------- #38 styles.py, the canvas decals (one pixel)
/// `whirl_light(sxp, szp, c, r, th, om, R, age)` at one canvas pixel: the swept trail behind the whirling arm,
/// brightest just behind it and fading back over 200 degrees, and a bright ring on the ball's path (0 to 1).
#[allow(clippy::too_many_arguments)] // the Python signature
fn whirl_light(sx: f64, sz: f64, c: [f64; 2], r: f64, th: f64, om: f64, age: f64) -> f64 {
    let (dx, dz) = (sx - c[0], sz - c[1]);
    let d = np::hypot(dx, dz);
    let a = dmath::atan2(dz, dx);
    let sgn = if om >= 0.0 { 1.0 } else { -1.0 };
    // behind = ((th - a) * sgn) % (2 * math.pi): 0 at the arm, growing backward
    let behind = py::fmod_py((th - a) * sgn, 2.0 * dmath::PI);
    let trail = dmath::pow(np::clip(1.0 - behind / radians(200.0), 0.0, 1.0), 1.6);
    let near = if d < r + R + 0.12 { 1.0 } else { 0.0 };
    let disc = np::clip(1.0 - (d - (r + R * 0.9)) / 0.12, 0.0, 1.0) * near;
    let path = np::clip(1.0 - (d - r).abs() / (R * 0.75), 0.0, 1.0);
    let k = py_min(1.0, age / 0.12);
    (0.45 * disc * trail + 0.95 * path * trail) * k
}

/// `peg_light(sxp, szp, c, age)` at one canvas pixel: the spot where the peg went in, a dark dent and a bright ring
/// fresh for 0.3 s: `(dent, ring)`.
fn peg_light(sx: f64, sz: f64, c: [f64; 2], age: f64) -> (f64, f64) {
    let d = np::hypot(sx - c[0], sz - c[1]);
    let dent = np::clip(1.0 - d / 0.13, 0.0, 1.0);
    let ring =
        np::clip(1.0 - (d - (0.14 + 0.9 * py_min(age, 0.25))).abs() / 0.05, 0.0, 1.0) * py_max(0.0, 1.0 - age / 0.3);
    (dent, ring)
}

/// `shadow_pool(sxp, szp, p, R, k)` at one canvas pixel: the dark pool it slides in while sunk into shadow.
fn shadow_pool(sx: f64, sz: f64, p: [f64; 2], k: f64) -> f64 {
    let d = np::hypot(sx - p[0], sz - p[1]);
    dmath::pow(np::clip(1.0 - d / (R * 1.5), 0.0, 1.0), 0.7) * k
}

/// `erupt_light(sxp, szp, c, age, radius)` at one canvas pixel (`age` in 0..0.6): the eruption, a cracked ring
/// racing out to the radius and a hot centre.
fn erupt_light(sx: f64, sz: f64, c: [f64; 2], age: f64, radius: f64) -> f64 {
    let d = np::hypot(sx - c[0], sz - c[1]);
    let r = radius * dmath::pow(py_min(1.0, age / 0.22), 0.6);
    let ring = np::clip(1.0 - (d - r).abs() / 0.16, 0.0, 1.0);
    let core = np::clip(1.0 - d / 0.6, 0.0, 1.0) * py_max(0.0, 1.0 - age / 0.25);
    (ring + core) * (1.0 - age / 0.6)
}

// ---------------------------------------------------------------- #38 styles.py, over the ring
/// `draw_arm(img, cam, c, p, R, col, ghost=False, k=1.0)`: the endpin: a bright silver rod out of the ball's rim
/// down to the peg at `c` (a slate rim light, the black rubber collar at the canvas and a white glint where it
/// bites in); `ghost`: MULTIPHONICS' twin, pale and translucent.
fn draw_arm(img: &mut Img<f32>, cam: &Cam, c: [f64; 2], p: [f64; 2], col: V3, ghost: bool, k: f64) {
    let d = [c[0] - p[0], c[1] - p[1]];
    let l = np::hypot(d[0], d[1]);
    let u = if l > 1e-6 { [d[0] / l, d[1] / l] } else { [1.0, 0.0] };
    // a = p + u_ * R * 0.62: from the glass's edge ...
    let a = [p[0] + u[0] * R * 0.62, p[1] + u[1] * R * 0.62];
    let w = linspace0(1.0, 7);
    let pts: Vec<V3> = w
        .iter()
        .map(|&w| {
            [
                a[0] + (c[0] - a[0]) * w,
                a[1] + (c[1] - a[1]) * w,
                R * 0.55 * (1.0 - w) + 0.02,
            ]
        })
        .collect();
    let sp = wp(cam, &pts);
    let zc = cam.depth(&[to_world(p[0], p[1], R)])[0];
    // width = max(2, int(round(0.075 * cam.f / max(1e-6, zc))))
    let width = py_max(2.0, py::round_half_even(0.075 * cam.f / py_max(1e-6, zc))) as u32;
    let body = reeds::mask(&[], std::slice::from_ref(&sp), width + 2);
    let hi_line: Vec<(f64, f64)> = sp.iter().map(|&(x, y)| (x, y - 1.0)).collect();
    let hi = reeds::mask(&[], &[hi_line], 1);
    if ghost {
        reeds::blend(img, &body, tone(col, 0.4, 150.0), Num::Py(0.45 * k));
        reeds::blend(img, &hi, WHITE, Num::Py(0.5 * k));
        return;
    }
    let core = reeds::mask(&[], std::slice::from_ref(&sp), width);
    // the dark outline, the silver rod, its slate-lit top edge
    reeds::blend(img, &body, INK, Num::Py(0.9 * k));
    reeds::blend(img, &core, NICKEL, Num::Py(0.95 * k));
    reeds::blend(img, &hi, tone(col, 0.4, 170.0), Num::Py(0.9 * k));
    let collar = wp(cam, &pts[5..7]);
    reeds::blend(img, &reeds::mask(&[], &[collar], width + 2), BLACK, Num::Py(0.95 * k));
    let base = wp(cam, &[[c[0], c[1], 0.0]])[0];
    // xi, yi = int(base_[0]), int(base_[1])
    let (xi, yi) = (base.0 as i64, base.1 as i64);
    for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1)] {
        let (x, y) = (xi + dx, yi + dy);
        if 0 <= x && x < W as i64 && 0 <= y && y < H as i64 {
            mix(img.px_mut(x as usize, y as usize), 1.0 - k, WHITE.map(|v| v * k));
        }
    }
}

/// `draw_ghost_ball(img, cam, p, R, col, k)`: MULTIPHONICS: the twin's ball, a translucent outline on the far side
/// of the peg.
fn draw_ghost_ball(img: &mut Img<f32>, cam: &Cam, p: [f64; 2], col: V3, k: f64) {
    let s = wp(cam, &[[p[0], p[1], R]])[0];
    let rp = R * cam.f / py_max(1e-6, cam.depth(&[to_world(p[0], p[1], R)])[0]);
    let ring: Vec<(f64, f64)> = linspace0(2.0 * dmath::PI, 25)
        .into_iter()
        .map(|a| (s.0 + rp * dmath::cos(a), s.1 + rp * dmath::sin(a)))
        .collect();
    let disc = reeds::mask(std::slice::from_ref(&ring), &[], 1);
    let edge = reeds::mask(&[], &[ring], 1);
    reeds::blend(img, &disc, tone(col, 0.35, 150.0), Num::Py(0.22 * k));
    reeds::blend(img, &edge, tone(col, 0.5, 130.0), Num::Py(0.75 * k));
}

/// SLAP TONGUE's paddle at a frame: `(aim, ext, wind-up, slap flash)`.
type Slap = (f64, f64, f64, f64);

/// `draw_slap(img, cam, p, aim, R, ext, open_k, hit_k, col)`: the reed as a paddle: a broad, flat cane blade on a
/// black mouthpiece shoots out of the ball at the enemy and slaps (`open_k`: the wind-up, the paddle cocked back;
/// `ext`: its reach beyond the surface; `hit_k`: the slap's flash).
fn draw_slap(img: &mut Img<f32>, cam: &Cam, p: [f64; 2], sn: Slap, col: V3) {
    let (aim, ext, open_k, hit_k) = sn;
    let d = [dmath::cos(aim), dmath::sin(aim)];
    let nrm = [-d[1], d[0]];
    let base = [p[0] + d[0] * (R * 0.85), p[1] + d[1] * (R * 0.85)];
    let tip = [p[0] + d[0] * (R + ext), p[1] + d[1] * (R + ext)];
    // cock = nrm * 0.18 * open_k: wound back sideways before it slaps
    let cock = [nrm[0] * 0.18 * open_k, nrm[1] * 0.18 * open_k];
    let mid = [
        base[0] + (tip[0] - base[0]) * 0.35 + cock[0] * 0.5,
        base[1] + (tip[1] - base[1]) * 0.35 + cock[1] * 0.5,
    ];
    let (hw0, hw1) = (0.05, 0.16);
    let at = |o: [f64; 2], n: f64| [o[0] + nrm[0] * n, o[1] + nrm[1] * n];
    let tc = [tip[0] + cock[0], tip[1] + cock[1]];
    // q = [base_ + nrm * hw0, mid + nrm * hw0 * 1.2, tip + cock + nrm * hw1, tip + cock - nrm * hw1,
    //      mid - nrm * hw0 * 1.2, base_ - nrm * hw0]
    let q = [
        at(base, hw0),
        [mid[0] + nrm[0] * hw0 * 1.2, mid[1] + nrm[1] * hw0 * 1.2],
        at(tc, hw1),
        [tc[0] - nrm[0] * hw1, tc[1] - nrm[1] * hw1],
        [mid[0] - nrm[0] * hw0 * 1.2, mid[1] - nrm[1] * hw0 * 1.2],
        [base[0] - nrm[0] * hw0, base[1] - nrm[1] * hw0],
    ];
    let pts: Vec<V3> = q.iter().map(|a| [a[0], a[1], R]).collect();
    let sp = wp(cam, &pts);
    let m = reeds::mask(std::slice::from_ref(&sp), &[], 1);
    reeds::blend(img, &m, CANE, Num::Py(0.95));
    let stem = wp(cam, &[[base[0], base[1], R], [mid[0], mid[1], R]]);
    reeds::blend(img, &reeds::mask(&[], &[stem], 2), BLACK, Num::Py(0.95));
    let mut closed = sp.clone();
    closed.push(sp[0]);
    reeds::blend(img, &reeds::mask(&[], &[closed], 1), tone(col, 0.6, 60.0), Num::Py(0.8));
    if hit_k > 0.0 {
        reeds::blend(img, &m, WHITE, Num::Py(0.7 * hit_k));
    }
}

// ---------------------------------------------------------------- the look
/// The whirl at a frame (`anc_at`): `(peg, th, r, twin, age, om)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Whirl {
    c: [f64; 2],
    th: f64,
    r: f64,
    tw: f64,
    age: f64,
    om: f64,
}

/// The `extra` keys [`Look::state`] keeps the whirl in.
const ANC_KEYS: [&str; 7] = ["anc_cx", "anc_cz", "anc_th", "anc_r", "anc_tw", "anc_age", "anc_om"];

impl Whirl {
    /// `st['anc'] = ...`: into the ball's `extra`.
    fn put(&self, st: &mut BallState) {
        st.extra.insert("anc".to_string(), 1.0);
        let v = [self.c[0], self.c[1], self.th, self.r, self.tw, self.age, self.om];
        for (key, v) in ANC_KEYS.iter().zip(v) {
            st.extra.insert((*key).to_string(), v);
        }
    }

    /// `st.get('anc')`: the whirl a ball's state holds, if any.
    fn of(st: &BallState) -> Option<Whirl> {
        st.extra.get("anc")?;
        let v = |key: &str| st.extra.get(key).copied().unwrap_or(f64::NAN);
        Some(Whirl {
            c: [v("anc_cx"), v("anc_cz")],
            th: v("anc_th"),
            r: v("anc_r"),
            tw: v("anc_tw"),
            age: v("anc_age"),
            om: v("anc_om"),
        })
    }

    /// `st['anc'] = None`.
    fn clear(st: &mut BallState) {
        st.extra.remove("anc");
        for key in ANC_KEYS {
            st.extra.remove(key);
        }
    }
}

/// The BASS CLARINET's look: `Look` of `kit/fighters/bass_clarinet.py`.
#[derive(Clone, Debug, Default)]
pub struct BassClarinetLook {
    /// Its side.
    k: usize,
    /// `ANCH`: its pegs.
    anch: Vec<Anch>,
    /// `LOUT`: `(t, t_end)` of its LIGHTS OUTs.
    lout: Vec<(f64, f64)>,
    /// `ERUPTS`: `(t, where)` of its eruptions.
    erupts: Vec<(f64, [f64; 2])>,
    /// `TWINS`: `(t, until)` of its twins.
    twins: Vec<(f64, f64)>,
    /// `SNIP_OPEN`: `(t, t_snap, aim)` of its slaps.
    snip_open: Vec<(f64, f64, f64)>,
    /// `self.SLAP_AB['reach']` (`None` for an ability without one: Python raises where it reads it).
    slap_reach: Option<f64>,
    /// `self.LOW['radius']`.
    low_radius: Option<f64>,
    /// `self.cfg['weapon']['kind']`.
    weapon_kind: String,
    /// `self.anchored`: its weapon is an anchor and the run recorded `ANC`.
    anchored: bool,
}

impl BassClarinetLook {
    /// `sink_at(t)`: LIGHTS OUT: how far it has sunk into shadow (0 to 1): down over 0.3 s, popping up at the
    /// eruption.
    fn sink_at(&self, t_ko: f64, t: f64) -> f64 {
        for &(t0, te) in &self.lout {
            if t0 <= t && t < te + 0.16 && t0 < t_ko {
                if t < te {
                    return ss(t0, t0 + 0.3, t);
                }
                return 1.0 - ss(te, te + 0.12, t);
            }
        }
        0.0
    }

    /// `dim_at(t)`: LIGHTS OUT's arena dim: in over 0.35 s, held, back over 0.35 s after the eruption (at most 0.40).
    fn dim_at(&self, t_ko: f64, t: f64) -> f64 {
        let mut d = 0.0;
        for &(t0, te) in &self.lout {
            if t0 < t_ko && t0 <= t && t < te + 0.4 {
                d = py_max(d, 0.40 * ss(t0, t0 + 0.35, t) * (1.0 - ss(te + 0.05, te + 0.4, t)));
            }
        }
        d
    }

    /// `twin_at(t)`: how much of MULTIPHONICS' twin shows (in over 0.15 s, out over its last 0.15 s).
    fn twin_at(&self, t_ko: f64, t: f64) -> f64 {
        for &(t0, until) in &self.twins {
            if t0 <= t && t < until && t0 < t_ko {
                return py_min(1.0, (t - t0) / 0.15) * (1.0 - ss(until - 0.15, until, t));
            }
        }
        0.0
    }

    /// `anc_at(t)`: the whirl at `t` from the recorded `ANC`, or `None`.
    fn anc_at(&self, p: &PictureCtx, t: f64) -> Option<Whirl> {
        let t_fight = p.show.grid.t_fight;
        if !(t_fight <= t && t < p.t_ko) || !self.anchored {
            return None;
        }
        let i0 = p.sim_i(t);
        let row = p.run.row("ANC", i0, self.k)?;
        if row[0].is_nan() {
            return None;
        }
        // t0 = max([a[0] for a in self.ANCH if a[0] <= t] + [self.T.T_FIGHT])
        let t0 = self
            .anch
            .iter()
            .filter(|a| a.t <= t)
            .map(|a| a.t)
            .chain([t_fight])
            .fold(f64::NEG_INFINITY, py_max);
        let th_prev = p
            .run
            .row("ANC", i0.saturating_sub(1), self.k)
            .map_or(f64::NAN, |r| r[3]);
        let om = if th_prev.is_nan() {
            1.0
        } else {
            (row[3] - th_prev) * f64::from(FPS)
        };
        Some(Whirl {
            c: [row[1], row[2]],
            th: row[3],
            r: row[4],
            tw: row[5],
            age: t - t0,
            om,
        })
    }

    /// `snip_state(t)`: SLAP TONGUE's paddle at `t`, or `None`.
    fn snip_state(&self, p: &PictureCtx, t: f64) -> Option<Slap> {
        for &(t0, t_snap, aim) in &self.snip_open {
            if t0 <= t && t < t_snap + 0.22 && t0 < p.t_ko {
                let i = p.sim_i(py_min(t, p.t_ko));
                // a = float(row[1]) if not np.isnan(row[1]) else aim
                let a = match p.run.row("SNIP", i, self.k) {
                    Some(row) if !row[1].is_nan() => row[1],
                    _ => aim,
                };
                if t < t_snap {
                    let u = (t - t0) / py_max(1e-6, t_snap - t0);
                    return Some((a, 0.15 + 0.1 * u, u, 0.0));
                }
                let u = (t - t_snap) / 0.22;
                let reach = self
                    .slap_reach
                    .expect("SLAP TONGUE's paddle reads the ability's reach (Python raises KeyError)");
                let ext = reach
                    * if u < 0.6 {
                        dmath::sin(dmath::PI * py_min(1.0, u * 1.6))
                    } else {
                        (1.0 - u) / 0.4 * 0.6
                    };
                return Some((a, py_max(0.12, ext), 0.0, py_max(0.0, 1.0 - u / 0.4)));
            }
        }
        None
    }
}

impl Look for BassClarinetLook {
    fn shards(&self) -> Vec<String> {
        // ['wood'] * 6 + ['key'] * 8 + ['neck'] * 2 + ['bell'] * 2
        [("wood", 6), ("key", 8), ("neck", 2), ("bell", 2)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["snip", "erupt"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["erupt", "snip", "anchor"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["snip", "erupt", "anchor"]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `setup_as` took from `cfg`, by the keys `bass_clarinet.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.low=radius {:?}; abilities.unlock1=reach {:?}; weapon.kind={}",
            self.low_radius, self.slap_reach, self.weapon_kind
        ))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the roster numbers from `cfg`, the events of side `k`.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        let mine = |e: &&Event, kind: &str| e.kind == kind && e.who() == Some(k);
        self.anch = state::anch(ev, k);
        // (e[0], e[2], e[3]) for 'lights_out'; (e[0], e[2], [e[3], e[4]], e[5]) for 'erupt'
        self.lout = ev
            .iter()
            .filter(|e| mine(e, "lights_out"))
            .map(|e| (e.t, e.f(3)))
            .collect();
        self.erupts = ev
            .iter()
            .filter(|e| mine(e, "erupt"))
            .map(|e| (e.t, [e.f(3), e.f(4)]))
            .collect();
        self.twins = ev.iter().filter(|e| mine(e, "twin")).map(|e| (e.t, e.f(3))).collect();
        self.snip_open = ev
            .iter()
            .filter(|e| mine(e, "snip_open"))
            .map(|e| (e.t, e.f(3), e.f(4)))
            .collect();
        self.slap_reach = cfg.unlock1.get("reach");
        self.low_radius = cfg.low.get("radius");
        self.weapon_kind = cfg.weapon.kind().to_string();
        self.anchored = self.weapon_kind == "anchor" && p.run.has("ANC");
    }

    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k {
            return;
        }
        // st['sink'], st['twin'], st['anc'] = 0.0, 0.0, None
        st.sink = 0.0;
        st.extra.insert("twin".to_string(), 0.0);
        Whirl::clear(st);
        if phase != Phase::Fight {
            return;
        }
        st.sink = self.sink_at(p.t_ko, t);
        st.extra.insert("twin".to_string(), self.twin_at(p.t_ko, t));
        if let Some(w) = self.anc_at(p, t) {
            st.glow = py_max(st.glow, 0.35 * py_max(0.0, 1.0 - w.age / 0.4));
            w.put(st);
        }
    }

    // ---------------------------------------------------------------- events -> camera, ropes, sparks
    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "hit" {
            return match how(e) {
                "anchor" => Some(if sub(e) == Some(0) { 1.1 } else { 0.7 }),
                "snip" => Some(1.3),
                "erupt" => Some(2.4),
                _ => None,
            };
        }
        // the peg going in
        if e.kind == "anchor" { Some(0.5) } else { None }
    }

    fn rope(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "erupt" { Some(1.0) } else { None }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[owner(e)].map(|c| c * 0.6 + 100.0);
        let what = how(e);
        if self.spark_big().contains(&what) {
            (col, 16, 3.2, 0.3)
        } else if what == "anchor" {
            (col, 10, 2.6, 0.24)
        } else {
            (col, 7, 2.2, 0.22)
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "snip" | "erupt" => Some((14.0, 0.35)),
            "anchor" => Some((8.0, 0.2)),
            _ => None,
        }
    }

    fn fringe(&self, _p: &PictureCtx, e: &Event) -> bool {
        matches!(how(e), "snip" | "erupt") || (how(e) == "anchor" && sub(e) == Some(0))
    }

    // ---------------------------------------------------------------- effects (#40's _fx_build, in its order)
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.col[self.k];
        // ENDPIN ANCHOR: the peg goes in
        for a in &self.anch {
            if a.t > p.t_ko {
                break;
            }
            fx.fl.add(a.t, up(a.c[0], a.c[1], 0.3), col, 1.8, 0.5, 0.03, 0.3, 0.0);
            let dust = Emit {
                n: Some(8),
                col: Some(DUST_PEG.to_vec()),
                ..Emit::default()
            };
            fx.pa.emit(a.t, up(a.c[0], a.c[1], 0.05), "dust", &dust);
            fx.mk.add(
                a.t,
                Mark {
                    size: 0.32,
                    angle: a.t * 3.0,
                    col: [56.0, 48.0, 72.0],
                    life: 3.5,
                    n: 3,
                    // seed=int(t_ * 10)
                    seed: np::f64_to_i64(a.t * 10.0).max(0) as u64,
                    strength: 0.7,
                    ..Mark::new(MarkKind::Crack, a.c[0], a.c[1])
                },
            );
        }
        // LIGHTS OUT erupts
        for &(t, at) in &self.erupts {
            if t > p.t_ko {
                break;
            }
            let pos = up(at[0], at[1], 0.2);
            fx.bu.add(t, pos, col.map(|c| c * 0.6 + 100.0), 1.8, None, true);
            fx.fl.add(t, pos, col, 2.6, 1.2, 0.03, 0.45, 0.0);
            let dust = Emit {
                n: Some(22),
                spread: Some(180.0),
                col: Some(vec![[170.0, 160.0, 190.0], [110.0, 100.0, 130.0]]),
                speed: Some((1.5, 3.2)),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "dust", &dust);
            let chips = Emit {
                n: Some(10),
                col: Some(vec![[60.0, 52.0, 76.0], [110.0, 132.0, 170.0]]),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "chips", &chips);
            fx.mk.add(
                t,
                Mark {
                    size: 1.2,
                    angle: 0.4,
                    col: [40.0, 34.0, 56.0],
                    life: 6.0,
                    n: 6,
                    seed: np::f64_to_i64(t * 10.0).max(0) as u64,
                    strength: 0.85,
                    ..Mark::new(MarkKind::Crack, at[0], at[1])
                },
            );
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind != "hit" {
            return;
        }
        let (t, att) = (e.t, owner(e));
        let pos = up(e.f(5), e.f(6), R);
        match how(e) {
            "anchor" => {
                // the whirling body smashes in
                let strength = if sub(e) == Some(0) { 1.1 } else { 0.8 };
                fx.bu
                    .add(t, pos, p.col[att].map(|c| c * 0.6 + 100.0), strength, None, true);
                fx.fl.add(t, pos, p.col[att], 1.6, 0.7, 0.03, 0.25, 0.0);
                let sparks = Emit {
                    n: Some(10),
                    col: Some(SLATE_SPARK.to_vec()),
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "sparks", &sparks);
            }
            "snip" => {
                // SLAP TONGUE: a cane smack
                fx.bu.add(t, pos, [240.0, 220.0, 160.0], 1.3, None, true);
                fx.fl.add(t, pos, p.col[att], 1.8, 0.9, 0.03, 0.3, 0.0);
                let chips = Emit {
                    n: Some(10),
                    col: Some(vec![[232.0, 210.0, 152.0], [196.0, 168.0, 110.0]]),
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "chips", &chips);
            }
            "erupt" => fx.bu.add(t, pos, [240.0, 236.0, 255.0], 1.4, None, true),
            _ => {}
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let chips = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(CHIPS.to_vec()),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &chips);
    }

    // ---------------------------------------------------------------- the canvas
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let col = p.col[self.k];
        let bayer = bayer_full();
        if let Some(w) = Whirl::of(&c.fs[self.k]) {
            // the whirl's swept trail (and the twin's) and the peg's bright ring
            let lit = col.map(|v| v / 255.0 * 1.2 + 0.25);
            let ring_lit = col.map(|v| v / 255.0 + 0.3);
            for (j, light) in ic.iter_mut().enumerate() {
                let (sx, sz) = (c.sxp[j], c.szp[j]);
                let mut v = whirl_light(sx, sz, w.c, w.r, w.th, w.om, w.age);
                if w.tw > 0.5 {
                    v += 0.6 * whirl_light(sx, sz, w.c, w.r, w.th + dmath::PI, w.om, w.age);
                }
                let v = (v * 4.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 4.0;
                for ch in 0..3 {
                    light[ch] += v * 0.9 * lit[ch];
                }
                let (_, ring) = peg_light(sx, sz, w.c, w.age);
                for ch in 0..3 {
                    light[ch] += ring * 0.8 * ring_lit[ch];
                }
            }
        }
        // LIGHTS OUT's eruption
        let lit = col.map(|v| v / 255.0 * 1.2 + 0.3);
        for &(t, at) in &self.erupts {
            if t <= c.t && c.t < t + 0.6 {
                let radius = self
                    .low_radius
                    .expect("the eruption reads LIGHTS OUT's radius (Python raises KeyError)");
                let age = c.t - t;
                for (j, light) in ic.iter_mut().enumerate() {
                    let v = if (0.0..0.6).contains(&age) {
                        erupt_light(c.sxp[j], c.szp[j], at, age, radius)
                    } else {
                        0.0
                    };
                    let v = (v * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
                    for ch in 0..3 {
                        light[ch] += v * 1.2 * lit[ch];
                    }
                }
            }
        }
    }

    fn canvas_after(&self, _p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        // the peg's dent and LIGHTS OUT's shadow pool darken the canvas last
        let st = &c.fs[self.k];
        let whirl = Whirl::of(st);
        let sunk = st.sink > 0.02;
        if whirl.is_none() && !sunk {
            return;
        }
        for (j, light) in ic.iter_mut().enumerate() {
            let (sx, sz) = (c.sxp[j], c.szp[j]);
            let mut shade = whirl.map(|w| peg_light(sx, sz, w.c, w.age).0 * 0.6);
            if sunk {
                let pool = shadow_pool(sx, sz, st.p, st.sink) * 0.75;
                shade = Some(shade.map_or(pool, |s| np::maximum(s, pool)));
            }
            if let Some(s) = shade {
                for v in light.iter_mut() {
                    *v *= 1.0 - s;
                }
            }
        }
    }

    // ---------------------------------------------------------------- over the ring: the endpin, the twin, the paddle
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        let st = &f.fs[self.k];
        let col = p.col[self.k];
        if let Some(w) = Whirl::of(st).filter(|_| st.vis) {
            let k_in = py_min(1.0, w.age / 0.06);
            let twin = st.extra.get("twin").copied().unwrap_or(0.0);
            if w.tw > 0.5 {
                // MULTIPHONICS: the twin, opposite the peg
                let q = [
                    w.c[0] + w.r * dmath::cos(w.th + dmath::PI),
                    w.c[1] + w.r * dmath::sin(w.th + dmath::PI),
                ];
                draw_ghost_ball(arr, f.cam, q, col, twin * k_in);
                draw_arm(arr, f.cam, w.c, q, col, true, k_in * twin);
            }
            draw_arm(arr, f.cam, w.c, st.p, col, false, k_in);
        }
        if let Some(sn) = self.snip_state(p, f.t).filter(|_| st.vis) {
            draw_slap(arr, f.cam, st.p, sn, col);
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
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let at = |q: [f64; 2]| to_world(q[0], q[1], pos[1]);
        if kind == "wood" || kind == "neck" {
            // a stick of the body / the neck
            let (ln, wd) = if kind == "wood" { (0.30, 0.04) } else { (0.16, 0.03) };
            let q4: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(a1, a2)| {
                    at([
                        sx + d[0] * ln * a1 + nn[0] * wd * a2,
                        sz + d[1] * ln * a1 + nn[1] * wd * a2,
                    ])
                })
                .collect();
            let sp = ring::screen(cam, &q4);
            ring::layer_blend(arr, &[sp], if kind == "wood" { WOOD } else { SHARD_NICKEL }, 1.0, None);
            return;
        }
        // a key (a chip), the bell (a big curved chip)
        let sz_ = size * if kind == "key" { 0.7 } else { 1.6 };
        let tri: Vec<V3> = [
            [sx + d[0] * sz_, sz + d[1] * sz_],
            [
                sx - d[0] * sz_ * 0.7 + nn[0] * sz_ * 0.8,
                sz - d[1] * sz_ * 0.7 + nn[1] * sz_ * 0.8,
            ],
            [
                sx - d[0] * sz_ * 0.4 - nn[0] * sz_ * 0.7,
                sz - d[1] * sz_ * 0.4 - nn[1] * sz_ * 0.7,
            ],
        ]
        .iter()
        .map(|&q| at(q))
        .collect();
        let sp = ring::screen(cam, &tri);
        let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
        ring::layer_blend(
            arr,
            &[sp],
            SHARD_NICKEL.map(|c| np::clip(c * shade_k, 0.0, 255.0)),
            1.0,
            None,
        );
    }

    // ---------------------------------------------------------------- lens, LIGHTS OUT's dim
    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        // the eruption's heat ripple
        for &(t0, at) in &self.erupts {
            let u = t - t0;
            if 0.0 <= u && u < 0.5 && t0 < p.t_ko {
                img.shimmer_world(
                    cam,
                    to_world(at[0], at[1], 0.05),
                    0.4 + 2.2 * u / 0.5,
                    0.4,
                    3.0,
                    t,
                    1.0 - u / 0.5,
                    self.k as f64,
                );
            }
        }
    }

    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        // LIGHTS OUT: the whole arena eases darker (at most 40 %); both balls stay readable (soft keep circles)
        let d = self.dim_at(p.t_ko, f.t);
        if d <= 0.0 {
            return;
        }
        let mut keep = vec![0.0f32; W * H];
        for b in drawn {
            let ((sx, sy), rp) = (b.s, b.rp);
            let r = rp * 1.8;
            // x0, x1 = int(max(0, s_[0] - r)), int(min(W, s_[0] + r + 1))
            let x0 = py_max(0.0, sx - r) as i64;
            let x1 = py_min(W as f64, sx + r + 1.0) as i64;
            let y0 = py_max(0.0, sy - r) as i64;
            let y1 = py_min(H as f64, sy + r + 1.0) as i64;
            if x1 > x0 && y1 > y0 {
                for y in y0..y1 {
                    for x in x0..x1 {
                        let dd = np::hypot(x as f64 - sx, y as f64 - sy) / r;
                        let v = np::clip(1.4 - dd * 1.4, 0.0, 1.0);
                        let at = y as usize * W + x as usize;
                        keep[at] = np::maximum(f64::from(keep[at]), v) as f32;
                    }
                }
            }
        }
        // img * (1 - d * (1 - keep))[..., None]: a float32 factor (d is a Python float)
        let df = d as f32;
        let factor: Vec<f32> = keep.iter().map(|&k| 1.0f32 - df * (1.0f32 - k)).collect();
        match img {
            Pic::F32(a) => {
                for (px, &g) in a.data.as_chunks_mut::<3>().0.iter_mut().zip(&factor) {
                    for v in px.iter_mut() {
                        *v *= g;
                    }
                }
            }
            Pic::F64(a) => {
                for (px, &g) in a.data.as_chunks_mut::<3>().0.iter_mut().zip(&factor) {
                    for v in px.iter_mut() {
                        *v *= f64::from(g);
                    }
                }
            }
        }
    }

    fn focus_points(&self, _p: &PictureCtx, _t: f64, st: &BallState) -> Vec<(V3, String)> {
        match Whirl::of(st) {
            Some(w) => vec![(to_world(w.c[0], w.c[1], 0.0), "peg".to_string())],
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> BassClarinetLook {
        BassClarinetLook {
            lout: vec![(10.0, 12.0)],
            twins: vec![(20.0, 24.0)],
            ..BassClarinetLook::default()
        }
    }

    #[test]
    fn it_sinks_into_shadow_and_pops_up_at_the_eruption() {
        let l = look();
        assert_eq!(l.sink_at(60.0, 9.9), 0.0);
        assert_eq!(l.sink_at(60.0, 10.15), 0.5);
        assert_eq!(l.sink_at(60.0, 11.0), 1.0);
        assert_eq!(l.sink_at(60.0, 12.06), 0.5);
        assert_eq!(l.sink_at(60.0, 12.2), 0.0);
        // a LIGHTS OUT after the K.O. shows nothing
        assert_eq!(l.sink_at(9.0, 11.0), 0.0);
    }

    #[test]
    fn the_arena_dims_to_forty_percent_and_back() {
        let l = look();
        assert_eq!(l.dim_at(60.0, 9.0), 0.0);
        assert_eq!(l.dim_at(60.0, 11.0), 0.40);
        assert_eq!(l.dim_at(60.0, 12.4), 0.0);
        assert!(l.dim_at(60.0, 12.2) > 0.0 && l.dim_at(60.0, 12.2) < 0.40);
    }

    #[test]
    fn the_twin_fades_in_and_out() {
        let l = look();
        assert_eq!(l.twin_at(60.0, 19.0), 0.0);
        assert!((l.twin_at(60.0, 20.075) - 0.5).abs() < 1e-9);
        assert_eq!(l.twin_at(60.0, 22.0), 1.0);
        assert_eq!(l.twin_at(60.0, 24.0), 0.0);
    }

    #[test]
    fn a_whirl_rides_in_the_balls_extra() {
        let mut st = BallState::default();
        assert_eq!(Whirl::of(&st), None);
        let w = Whirl {
            c: [0.5, -0.25],
            th: 7.0,
            r: 0.95,
            tw: 1.0,
            age: 0.1,
            om: -3.0,
        };
        w.put(&mut st);
        assert_eq!(Whirl::of(&st), Some(w));
        Whirl::clear(&mut st);
        assert_eq!(Whirl::of(&st), None);
        assert!(st.extra.is_empty());
    }

    #[test]
    fn the_trail_is_brightest_just_behind_the_arm() {
        // the arm at angle 0, turning forward: a pixel on the path just behind it is lit, one just ahead is not
        let c = [0.0, 0.0];
        let behind = whirl_light(dmath::cos(-0.1), dmath::sin(-0.1), c, 1.0, 0.0, 1.0, 1.0);
        let ahead = whirl_light(dmath::cos(0.1), dmath::sin(0.1), c, 1.0, 0.0, 1.0, 1.0);
        assert!(behind > 0.8 && ahead < 0.05, "{behind} {ahead}");
        // the other way round when it turns backward
        let ahead_back = whirl_light(dmath::cos(0.1), dmath::sin(0.1), c, 1.0, 0.0, -1.0, 1.0);
        assert!(ahead_back > 0.8);
        // a fresh whirl lights in over 0.12 s
        assert_eq!(whirl_light(1.0, 0.0, c, 1.0, 0.0, 1.0, 0.0), 0.0);
    }

    #[test]
    fn a_mixed_pixel_follows_numpys_dtypes() {
        let mut px = [100.5f32, 0.1, 255.0];
        mix(&mut px, 0.3, [7.0, 0.25, 1e-9]);
        let want = |v: f32, a: f64| (f64::from(v * 0.3f32) + a) as f32;
        assert_eq!(px, [want(100.5, 7.0), want(0.1, 0.25), want(255.0, 1e-9)]);
    }
}
