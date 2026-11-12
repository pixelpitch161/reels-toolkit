//! The TRUMPET's look: the port of `Look` in `projects/ballfight/kit/fighters/trumpet.py` and of the module's draw
//! helpers, hook by hook.
//!
//! What it draws: the side-on trumpet turning inside its ball so its valves face the punches (and, for HIGH C, its
//! bell swings down the line); VALVE COMBO's pistons (a brass collar, the silver rod, a pearl button rimmed in its
//! colour, a motion streak) with steam from the valve, scorch marks, brass sparks and a burst where a punch lands;
//! FANFARE's light ring, banners and shimmer; HIGH C's dashed sight and lit lane while it charges, then the blast
//! (a soft halo, the body and a white-hot core, each blurred with `scipy.ndimage.gaussian_filter`, ripples, a
//! muzzle star, a streak of light on the canvas, scorch marks); the bell mouth's glow; the winner's solo notes
//! rising from the bell; tubing, valve caps and brass chips when it loses; and the VS card's panel (the bell's
//! flare and three piston valves).
//!
//! The kit keeps an atlas of the trumpet's sprites (radii 12 to 19, every 10 degrees of roll) in a file; here a
//! sprite is drawn when first asked for and kept on the look, which is the same picture.
//!
//! The arithmetic keeps Python's order and dtypes: the masks are `float32`, every blend's alpha a Python float, the
//! canvas light `float64`. The sines, cosines, arctangents, exponentials and powers are `bf_core::dmath`'s.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::{FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W, bayer_full, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::{Img, RgbF};
use bf_picture::look::{CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use bf_picture::ndimage::gaussian_filter;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::shadows::proj_point;
use bf_picture::sprite::{Mesh, render_sprite};
use bf_picture::title::layout;
use bf_picture::{np, pil};

use crate::fighters::shared::reeds::{Num, blend_into, mask_of};

/// `BRASS`.
const BRASS: V3 = [232.0, 184.0, 72.0];
/// `BRASS_HI`.
const BRASS_HI: V3 = [255.0, 236.0, 160.0];
/// `BRASS_DK`.
const BRASS_DK: V3 = [150.0, 104.0, 34.0];
/// `PEARL`.
const PEARL: V3 = [238.0, 236.0, 228.0];
/// `PISTON`: the nickel-silver piston.
const PISTON: V3 = [212.0, 218.0, 230.0];
/// `COLLAR`: the brass valve-casing collar.
const COLLAR: V3 = [214.0, 168.0, 62.0];
/// `BANNER`.
const BANNER: V3 = [255.0, 214.0, 72.0];
/// `BANNER_TRIM`.
const BANNER_TRIM: V3 = [196.0, 40.0, 52.0];
/// `BELL_IN`: the trumpet sprite's bell mouth sits this far (x R) from its ball's centre, along the aim.
const BELL_IN: f64 = 0.76;
/// `FADE`: seconds a blast takes to fade once its hold is over.
const FADE: f64 = 0.22;
/// `ROLL_STEP`: the sprite turns in steps of this many degrees.
const ROLL_STEP: i64 = 10;
/// The kit's `6.28` (its circles of `np.linspace(0, 6.28, n)` stop a little short of a full turn; not 2 pi).
#[allow(clippy::approx_constant)]
const TURN: f64 = 6.28;
/// `SPR_RADII`: the sprites span radii 12 to 19.
const SPR_RADII: (u32, u32) = (12, 19);

type P2 = [f64; 2];
type Poly = Vec<(f64, f64)>;

// ---------------------------------------------------------------- small helpers, as numpy computes them
/// `np.array([math.cos(a), math.sin(a)])`.
fn unit(a: f64) -> P2 {
    [dmath::cos(a), dmath::sin(a)]
}

/// `np.linspace(a, b, n)`: `i * step + a`, the last value `b` itself.
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    let step = (b - a) / (n - 1) as f64;
    (0..n)
        .map(|i| if i == n - 1 { b } else { i as f64 * step + a })
        .collect()
}

/// `np.clip(col, 0, 255)`.
fn clip_col(c: V3) -> V3 {
    c.map(|v| np::clip(v, 0.0, 255.0))
}

/// `S.to_world(q[0], q[1], y)`.
fn world(q: P2, y: f64) -> V3 {
    scene::to_world(q[0], q[1], y)
}

/// `np.searchsorted(a, x)` (the left side).
fn search_left(a: &[f64], x: f64) -> usize {
    a.partition_point(|&v| v < x)
}

/// `np.searchsorted(a, x, 'right')`.
fn search_right(a: &[f64], x: f64) -> usize {
    a.partition_point(|&v| v <= x)
}

/// A number of a recording's value.
fn value_num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// `e[i]` as a number, when the event has one there.
fn num(e: &Event, i: usize) -> Option<f64> {
    value_num(e.fields.get(i.checked_sub(2)?)?)
}

/// `e[i]` as a whole number, when the event has one there.
fn int(e: &Event, i: usize) -> Option<i64> {
    match e.fields.get(i.checked_sub(2)?)? {
        Value::Int(n) => Some(*n),
        _ => None,
    }
}

/// `e[i]` as text, or `""`.
fn text(e: &Event, i: usize) -> &str {
    match i.checked_sub(2).and_then(|j| e.fields.get(j)) {
        Some(Value::Str(s)) => s,
        _ => "",
    }
}

/// `e[7]` of a hit (what hit), or `""`.
fn how(e: &Event) -> &str {
    if e.kind == "hit" { text(e, 7) } else { "" }
}

/// `n=...` alone as the options of an `emit`.
fn count(n: i64) -> Emit {
    Emit {
        n: Some(n),
        ..Emit::default()
    }
}

/// `_wdir(d)`: a ring-frame direction in the world.
fn wdir(d: P2) -> V3 {
    scene::to_world(d[0], d[1], 0.0)
}

/// The TRUMPET's own model, compiled in: what a look that was never set up draws.
fn own_mesh() -> &'static Mesh {
    static MESH: OnceLock<Mesh> = OnceLock::new();
    MESH.get_or_init(|| {
        let json = super::MESH.expect("the TRUMPET's model is compiled in");
        Mesh::from_json(json).expect("the TRUMPET's model reads")
    })
}

// ---------------------------------------------------------------- HIGH C: the charge sight and the blast
/// `(extension 0..1, alpha, tail 0..1, age)` of a blast.
type BeamState = (f64, f64, f64, f64);

/// `draw_sight(arr, cam, p, a, length, k, t, col)`: the locked aim while the bell charges: a dashed line marching
/// outward, brighter as the charge fills.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_sight(arr: &mut Img<f32>, cam: &Cam, p: P2, a: f64, length: f64, k: f64, t: f64, col: V3) {
    let d = unit(a);
    let o = [p[0] + d[0] * (R + 0.08), p[1] + d[1] * (R + 0.08)];
    // n = max(2, int(length / 0.22))
    let n = ((length / 0.22) as i64).max(2);
    let off = py::fmod_py(t * 3.0, 1.0);
    let mut segs: Vec<Poly> = Vec::new();
    for j in 0..n {
        let u0 = (j as f64 + off) / n as f64;
        let u1 = (j as f64 + off + 0.45) / n as f64;
        if u1 > 1.0 {
            continue;
        }
        let a0 = [o[0] + d[0] * length * u0, o[1] + d[1] * length * u0];
        let a1 = [o[0] + d[0] * length * u1, o[1] + d[1] * length * u1];
        segs.push(cam.wp(&[world(a0, R), world(a1, R)]));
    }
    let m = mask_of(&segs, Some(1));
    blend_into(arr, &m, clip_col(col.map(|c| c * 0.7 + 90.0)), Num::Py(0.25 + 0.6 * k));
}

/// `beam_state(t, t_fire, hold)`: `(extension 0..1, alpha, tail 0..1, age)` of a blast fired at `t_fire`: it shoots
/// out of the bell in 50 ms, holds, then fades as its back end lets go of the bell and races after the tip.
fn beam_state(t: f64, t_fire: f64, hold: f64) -> Option<BeamState> {
    let u = t - t_fire;
    if u < 0.0 || u > hold + FADE {
        return None;
    }
    let e = 1.0 - py_min(1.0, u / 0.05);
    let ext = 1.0 - e * e;
    if u <= hold {
        return Some((ext, 1.0, 0.0, u));
    }
    let k = (u - hold) / FADE;
    Some((ext, 1.0 - k, dmath::pow(k, 1.4), u))
}

/// `_beam_outline(p, a, s0, s1, width, y, n=18, flare_len=0.32)`: a blast's outline in the world: narrow at the
/// bell mouth (s = 0), opening to full width over `flare_len`, a rounded tip; `s0..s1` along the aim from the
/// ball's centre + `BELL_IN` R.
fn beam_outline(p: P2, a: f64, s0: f64, s1: f64, width: f64, y: f64) -> Option<Vec<V3>> {
    let (n, flare_len) = (18, 0.32);
    let d = unit(a);
    let nrm = [-d[1], d[0]];
    let o = [p[0] + d[0] * BELL_IN * R, p[1] + d[1] * BELL_IN * R];
    if s1 - s0 < 1e-3 {
        return None;
    }
    let ss_ = linspace(s0, s1, n);
    let w: Vec<f64> = ss_
        .iter()
        .map(|&s| {
            let k = np::clip(s / flare_len, 0.0, 1.0);
            width / 2.0 * (0.32 + 0.68 * (k * k * (3.0 - 2.0 * k)))
        })
        .collect();
    // left = [o + d * sv + nrm * wv ...]; right = [o + d * sv - nrm * wv ...]
    let left: Vec<P2> = ss_
        .iter()
        .zip(&w)
        .map(|(&sv, &wv)| [o[0] + d[0] * sv + nrm[0] * wv, o[1] + d[1] * sv + nrm[1] * wv])
        .collect();
    let right: Vec<P2> = ss_
        .iter()
        .zip(&w)
        .map(|(&sv, &wv)| [o[0] + d[0] * sv - nrm[0] * wv, o[1] + d[1] * sv - nrm[1] * wv])
        .collect();
    let wl = w[n - 1];
    // tip = [o + d * s1 + w[-1] * (math.cos(q) * d + math.sin(q) * nrm) for q in np.linspace(pi / 2, -pi / 2, 7)]
    let tip: Vec<P2> = linspace(dmath::PI / 2.0, -dmath::PI / 2.0, 7)
        .iter()
        .map(|&q| {
            let (c, s) = (dmath::cos(q), dmath::sin(q));
            [
                o[0] + d[0] * s1 + wl * (c * d[0] + s * nrm[0]),
                o[1] + d[1] * s1 + wl * (c * d[1] + s * nrm[1]),
            ]
        })
        .collect();
    // pts = left + tip[1:-1] + right[::-1]
    let pts = left.iter().chain(&tip[1..tip.len() - 1]).chain(right.iter().rev());
    Some(pts.map(|&q| world(q, y)).collect())
}

/// `draw_beam(arr, cam, p, a, length, width, st, col, t, y=R)`: the blast as light out of the trumpet's bell: a soft
/// wide halo, the body, a white-hot core (all with blurred, not hard, edges), sound-wave ripples racing along it
/// while it holds. (Python's `t` is not used.)
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_beam(arr: &mut Img<f32>, cam: &Cam, p: P2, a: f64, length: f64, width: f64, st: BeamState, col: V3) {
    let y = R;
    let (ext, alpha, tail, u) = st;
    // the engine's beam starts just outside the glass; the light, at the bell
    let lv = length + R + 0.04 - BELL_IN * R;
    let s1 = lv * ext;
    let s0 = (s1 - 0.05) * tail;
    let thin = 1.0 - 0.55 * tail;
    let layers: [(f64, V3, f64, f64); 3] = [
        (3.2, col.map(|c| c * 0.7 + 60.0), 0.42, 2.4),
        (1.25, col.map(|c| c * 0.85 + 45.0), 0.9, 0.9),
        (0.42, [255.0, 252.0, 236.0], 1.0, 0.5),
    ];
    for (wk, c, mut ak, sig) in layers {
        // a wide blast keeps a narrow halo
        let ww = if wk < 3.0 {
            width * wk
        } else {
            width + py_min(width * (wk - 1.0), 0.5)
        };
        if wk > 3.0 && width > 0.6 {
            ak *= 0.6;
        }
        let Some(outline) = beam_outline(p, a, s0, s1, py_max(0.03, ww * thin), y) else {
            return;
        };
        let mut m = gaussian_filter(&mask_of(&[cam.wp(&outline)], None), sig);
        // np.clip(m * 1.15, 0, 1): float32 all the way
        for v in &mut m.data {
            *v = np::clip_f32(*v * 1.15f32, 0.0, 1.0);
        }
        blend_into(arr, &m, clip_col(c), Num::Py(ak * alpha));
    }
    if tail == 0.0 && s1 > 0.4 {
        // ripples: little wavefronts bowed forward, moving out at 8 units/s
        let d = unit(a);
        let nrm = [-d[1], d[0]];
        let o = [p[0] + d[0] * BELL_IN * R, p[1] + d[1] * BELL_IN * R];
        let qs = linspace(-1.0, 1.0, 7);
        let mut lines: Vec<Poly> = Vec::new();
        for j in 0..6 {
            let sj = py::fmod_py(0.35 + f64::from(j) * 0.62 + 8.0 * u, py_max(0.5, s1 - 0.25));
            if sj < 0.3 {
                continue;
            }
            let hw = width * 1.05;
            let arc: Vec<V3> = qs
                .iter()
                .map(|&q| {
                    let along = sj - 0.06 * (1.0 - q * q);
                    world(
                        [
                            o[0] + d[0] * along + nrm[0] * hw * q,
                            o[1] + d[1] * along + nrm[1] * hw * q,
                        ],
                        y,
                    )
                })
                .collect();
            lines.push(cam.wp(&arc));
        }
        if !lines.is_empty() {
            let m = mask_of(&lines, Some(1));
            blend_into(arr, &m, clip_col(col.map(|c| c * 0.4 + 165.0)), Num::Py(0.55 * alpha));
        }
    }
}

/// `draw_flash(arr, cam, p, a, k, col, y=R)`: the muzzle flash at the bell mouth as a blast leaves: a short
/// four-point star.
fn draw_flash(arr: &mut Img<f32>, cam: &Cam, p: P2, a: f64, k: f64, col: V3) {
    if k <= 0.02 {
        return;
    }
    let o = bell_mouth(p, a);
    let s = cam.wp(&[world(o, R)])[0];
    let l = 5.0 + 7.0 * k;
    let lines = vec![
        vec![(s.0 - l, s.1), (s.0 + l, s.1)],
        vec![(s.0, s.1 - l * 0.8), (s.0, s.1 + l * 0.8)],
    ];
    blend_into(arr, &mask_of(&lines, Some(1)), [255.0, 250.0, 230.0], Num::Py(0.85 * k));
    let qs = linspace(0.0, TURN, 9);
    let star: Poly = qs[..8]
        .iter()
        .map(|&q| (s.0 + 2.2 * dmath::cos(q), s.1 + 2.2 * dmath::sin(q)))
        .collect();
    blend_into(
        arr,
        &mask_of(&[star], None),
        clip_col(col.map(|c| c * 0.3 + 190.0)),
        Num::Py(k),
    );
}

/// `bell_mouth(p, a)`: the trumpet's bell mouth in the ring frame (inside the glass, along the aim).
fn bell_mouth(p: P2, a: f64) -> P2 {
    let d = unit(a);
    [p[0] + d[0] * BELL_IN * R, p[1] + d[1] * BELL_IN * R]
}

/// `draw_note_rise(arr, cam, x, z, y, col, a)`: a solo note floating up from the champion's horn (an eighth note,
/// fading).
fn draw_note_rise(arr: &mut Img<f32>, cam: &Cam, x: f64, z: f64, y: f64, col: V3, a: f64) {
    if a <= 0.0 {
        return;
    }
    let at = scene::to_world(x, z, y);
    let s = cam.wp(&[at])[0];
    let k = cam.f / cam.depth(&[at])[0];
    let hr = 0.07 * k;
    let qs = linspace(0.0, 2.0 * dmath::PI, 11);
    let head: Poly = qs[..10]
        .iter()
        .map(|&q| {
            (
                s.0 + hr * 1.25 * dmath::cos(q) - hr * 0.3 * dmath::sin(q),
                s.1 + hr * 0.85 * dmath::sin(q),
            )
        })
        .collect();
    let c = clip_col(col.map(|v| v * 0.5 + 128.0));
    blend_into(arr, &mask_of(&[head], None), c, Num::Py(a));
    let (sx, sy) = (s.0 + hr * 1.1, s.1);
    let top = sy - 0.28 * k;
    let stem = vec![
        vec![(sx, sy), (sx, top)],
        vec![(sx, top), (sx + 0.09 * k, top + 0.09 * k)],
    ];
    blend_into(arr, &mask_of(&stem, Some(1)), c, Num::Py(a));
}

// ---------------------------------------------------------------- VALVE COMBO: three pistons punching out of the ball
/// What the look reads from `self.W` (the roster's `weapon`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Valves {
    t_out: f64,
    t_hold: f64,
    t_back: f64,
    punches: i64,
    spread: f64,
}

impl Valves {
    /// `(i - (w['punches'] - 1) / 2) * w['spread']`: piston `i`'s place across the aim.
    fn off(&self, i: i64) -> f64 {
        (i as f64 - (self.punches - 1) as f64 / 2.0) * self.spread
    }
}

/// `piston_ext(u, w, reach)`: a piston's length beyond the ball's surface at age `u` (seconds, already divided by
/// the haste time scale): out (ease-out), hold, back (smoothstep); `None` once it is home.
fn piston_ext(u: f64, w: &Valves, reach: f64) -> Option<f64> {
    let (t_out, t_hold, t_back) = (w.t_out, w.t_hold, w.t_back);
    if u < 0.0 {
        return None;
    }
    if u < t_out {
        let e = u / t_out;
        return Some(reach * (1.0 - (1.0 - e) * (1.0 - e)));
    }
    if u < t_out + t_hold {
        return Some(reach);
    }
    if u < t_out + t_hold + t_back {
        let e = (u - t_out - t_hold) / t_back;
        return Some(reach * (1.0 - e * e * (3.0 - 2.0 * e)));
    }
    None
}

/// `piston_geom(p, th, off, ext)`: `(base, tip, d)` in the ring frame: the piston leaves the glass at its valve's
/// lateral offset.
fn piston_geom(p: P2, th: f64, off: f64, ext: f64) -> (P2, P2, P2) {
    let d = unit(th);
    let a0 = [p[0] + d[0] * R * 0.8 + -d[1] * off, p[1] + d[1] * R * 0.8 + d[0] * off];
    let reach = R * 0.2 + ext;
    (a0, [a0[0] + d[0] * reach, a0[1] + d[1] * reach], d)
}

/// A piston out of the ball: `(owner, index, aim, off, ext, stretch)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Piston {
    k: usize,
    a: f64,
    off: f64,
    ext: f64,
    stretch: f64,
}

/// `draw_piston(arr, cam, p, th, off, ext, stretch, col, y=R)`: a brass collar at the glass, the silver piston
/// rod, a pearl finger button on the end (its brass rim in the fighter's colour); `stretch` 0..1 adds a motion
/// streak while it shoots out.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_piston(arr: &mut Img<f32>, cam: &Cam, p: P2, th: f64, off: f64, ext: f64, stretch: f64, col: V3) {
    let y = R;
    let (a0, tip, d) = piston_geom(p, th, off, ext);
    let n = [-d[1], d[0]];
    let on_screen = |q: &[P2]| -> Poly { cam.wp(&q.iter().map(|&q| world(q, y)).collect::<Vec<V3>>()) };
    let plus = |c: P2, k: f64| -> P2 { [c[0] + n[0] * k, c[1] + n[1] * k] };
    let less = |c: P2, k: f64| -> P2 { [c[0] - n[0] * k, c[1] - n[1] * k] };
    if stretch > 0.05 {
        let streak = [less(a0, 0.12), less(tip, 0.15), plus(tip, 0.15), plus(a0, 0.12)];
        blend_into(
            arr,
            &mask_of(&[on_screen(&streak)], None),
            clip_col(col.map(|c| c * 0.5 + 120.0)),
            Num::Py(0.35 * stretch),
        );
    }
    let rod = [less(a0, 0.05), less(tip, 0.05), plus(tip, 0.05), plus(a0, 0.05)];
    blend_into(
        arr,
        &mask_of(&[on_screen(&rod)], None),
        PISTON.map(|c| c * 0.82),
        Num::Py(1.0),
    );
    let glint = [plus(a0, 0.018), plus(tip, 0.018)];
    blend_into(
        arr,
        &mask_of(&[on_screen(&glint)], Some(1)),
        [250.0, 252.0, 255.0],
        Num::Py(0.9),
    );
    let qs = linspace(0.0, TURN, 13);
    let discs = [
        (a0, 0.095, COLLAR),
        (tip, 0.16, clip_col(col.map(|c| c * 0.8 + 30.0))),
        ([tip[0] + d[0] * 0.015, tip[1] + d[1] * 0.015], 0.125, PEARL),
    ];
    for (c, rr, colr) in discs {
        let ring: Vec<V3> = qs[..12]
            .iter()
            .map(|&q| scene::to_world(c[0] + rr * dmath::cos(q), c[1] + rr * dmath::sin(q), y + 0.01))
            .collect();
        blend_into(arr, &mask_of(&[cam.wp(&ring)], None), colr, Num::Py(1.0));
    }
}

// ---------------------------------------------------------------- FANFARE: a ring of golden banners snapping outward
/// `draw_banners(arr, cam, c, r, k, t, n=10, y0=0.04, h=0.7)`: ten banners on the expanding ring: a pole, a pennant
/// streaming outward that snaps (flutters) as it goes.
fn draw_banners(arr: &mut Img<f32>, cam: &Cam, c: P2, r: f64, k: f64, t: f64) {
    let (n, y0, h) = (10, 0.04, 0.7);
    if k <= 0.02 {
        return;
    }
    let (mut poles, mut flags, mut trims): (Vec<Poly>, Vec<Poly>, Vec<Poly>) = (Vec::new(), Vec::new(), Vec::new());
    for j in 0..n {
        let a = 2.0 * dmath::PI * f64::from(j) / f64::from(n) + 0.3;
        let d = unit(a);
        let b = [c[0] + d[0] * r, c[1] + d[1] * r];
        let top = world(b, y0 + h);
        poles.push(cam.wp(&[world(b, y0), top]));
        let fl = 0.06 * dmath::sin(t * 34.0 + f64::from(j) * 1.7);
        let tipp = [b[0] + d[0] * 0.34, b[1] + d[1] * 0.34];
        let out = world(tipp, y0 + h - 0.08 + fl);
        flags.push(cam.wp(&[top, out, world(b, y0 + h - 0.2)]));
        trims.push(cam.wp(&[top, out]));
    }
    blend_into(arr, &mask_of(&poles, Some(1)), [240.0, 230.0, 200.0], Num::Py(k));
    blend_into(arr, &mask_of(&flags, None), BANNER, Num::Py(k));
    blend_into(arr, &mask_of(&trims, Some(1)), BANNER_TRIM, Num::Py(k));
}

// ---------------------------------------------------------------- the VS card panel
/// `trumpet_valves(base)`: the VS card panel: the bell's flare seen mouth-on (rings round the panel's far corner, a
/// bright rim) and three piston valves along the divider (caps, pearl buttons, stems). `base` is the panel's
/// gradient, changed in place.
pub fn trumpet_valves(base: &mut RgbF) {
    let dd = layout::dd();
    let side = layout::side();
    let (da, db) = (layout::DA, layout::DB);
    // a_mid = float(np.mean([TT.DA @ TT.DD, TT.DB @ TT.DD]))
    let a_mid = ((da[0] * dd[0] + da[1] * dd[1]) + (db[0] * dd[0] + db[1] * dd[1])) / 2.0;
    // np.where(mask, out * keep + col * k, out) for one pixel
    let mix = |px: &mut [f64], keep: f64, col: V3, k: f64| {
        for (v, c) in px.iter_mut().zip(col) {
            *v = *v * keep + c * k;
        }
    };
    for y in 0..base.h {
        for x in 0..base.w {
            let (xx, yy) = (x as f64, y as f64);
            let sd = side.px(x, y)[0];
            // the bell sits in the panel's far corner
            let (cx, cy) = if sd > 0.0 {
                (-40.0, -40.0)
            } else {
                (W as f64 + 40.0, H as f64 + 40.0)
            };
            let r = np::hypot(xx - cx, yy - cy);
            let q = py::fmod_py(r / 22.0, 1.0);
            let gain = if q < 0.18 {
                1.10
            } else if q > 0.86 {
                0.88
            } else {
                1.0
            };
            let px = base.px_mut(x, y);
            for v in px.iter_mut() {
                // out = base_ * 1.0, then out * gain
                *v = *v * 1.0 * gain;
            }
            if (r - 250.0).abs() < 3.0 {
                mix(px, 0.5, [255.0, 236.0, 160.0], 0.5);
            }
            let along = xx * dd[0] + yy * dd[1];
            let u = sd.abs();
            for j in 0..3 {
                let a0 = a_mid - 52.0 + 52.0 * f64::from(j);
                let rr = np::hypot(along - a0, u - 58.0);
                let cap = rr < 15.0;
                let ring = (rr - 15.0).abs() < 1.8;
                let btn = rr < 7.0;
                let stem = (along - a0).abs() < 3.0 && u > 73.0 && u < 118.0;
                if stem {
                    mix(px, 0.55, [212.0, 218.0, 230.0], 0.45);
                }
                if cap {
                    mix(px, 0.7, [255.0, 214.0, 120.0], 0.3);
                }
                if ring {
                    mix(px, 0.45, [255.0, 236.0, 170.0], 0.55);
                }
                if btn {
                    mix(px, 0.4, [250.0, 244.0, 226.0], 0.6);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- the look
/// A charge of `CHARGES`: `(t0, owner, aim, t_fire, length, width)` (the owner is the look's side).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Charge {
    t0: f64,
    aim: f64,
    t_fire: f64,
    ln: f64,
}

/// A blast of `BEAMS`: `(t_fire, owner, ox, oz, aim, length, width)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Blast {
    tf: f64,
    k: usize,
    ox: f64,
    oz: f64,
    a: f64,
    ln: f64,
    wd: f64,
}

/// A blast visible now: `(k, ox, oz, a, clipped length, width, ext, alpha, state)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Live {
    k: usize,
    ox: f64,
    oz: f64,
    a: f64,
    ln: f64,
    wd: f64,
    ext: f64,
    al: f64,
    st: BeamState,
}

/// A punch of `PUNCHES`: `(t0, owner, index, aim, reach, combo no., haste time scale)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Punch {
    t0: f64,
    k: usize,
    i: i64,
    a: f64,
    reach: f64,
    ks: f64,
}

/// An open charge's sight: `(k, aim, length, how far the charge has come)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sight {
    k: usize,
    a: f64,
    ln: f64,
    ck: f64,
}

/// The TRUMPET's look: `Look` of `kit/fighters/trumpet.py`.
#[derive(Debug, Default)]
pub struct TrumpetLook {
    /// Its side.
    k: usize,
    /// `CHARGES`.
    charges: Vec<Charge>,
    /// `BEAMS`.
    beams: Vec<Blast>,
    /// `BEAM_T`.
    beam_t: Vec<f64>,
    /// `HOLD`: `self.cfg['abilities']['low'].get('hold', 0.12)`.
    hold: f64,
    /// `self.cfg['abilities']['unlock2'].get('mult', 1.0)`.
    mult: f64,
    /// `PUNCHES`.
    punches: Vec<Punch>,
    /// `PUNCH_T`.
    punch_t: Vec<f64>,
    /// `self.W`: the weapon's numbers, `None` when `cfg`'s weapon is not a combo (Python raises when it reads one).
    w: Option<Valves>,
    /// `self.cfg['view'][0]`: the view's yaw.
    view_yaw: f64,
    /// `self.cfg['view'][2]`: the view's roll.
    view_roll: f64,
    /// `self.cfg['pp']`.
    pp: String,
    /// The model `sprites.render_sprite(self.cfg['pp'], ...)` draws: the model of the roster entry the look was set
    /// up with (its own, or another fighter's when it is made with that fighter's entry). `None` for an entry whose
    /// model this app does not have: asking such a look for a sprite panics, saying so.
    mesh: Option<Mesh>,
    /// `self.T.SOLO.get(self.name, [])`.
    solo: Vec<(f64, i32, f64)>,
    /// `_ATLAS`: the sprites drawn so far, by `(radius, roll)`.
    atlas: Mutex<BTreeMap<(u32, i64), Img<u8>>>,
}

impl TrumpetLook {
    /// `punches_live(t)`: the pistons out at `t` (they freeze with the hit-stop).
    fn punches_live(&self, p: &PictureCtx, t: f64) -> Vec<Piston> {
        if self.punch_t.is_empty() || t < p.show.grid.t_fight {
            return Vec::new();
        }
        let Some(w) = &self.w else { return Vec::new() };
        let tt = py_min(t, p.t_ko);
        let (j0, j1) = (search_left(&self.punch_t, tt - 0.6), search_right(&self.punch_t, tt));
        let mut out = Vec::new();
        for q in self.punches.get(j0..j1).unwrap_or(&[]) {
            let u = (tt - q.t0) / q.ks;
            if let Some(ext) = piston_ext(u, w, q.reach) {
                let stretch = if u < w.t_out {
                    py_max(0.0, 1.0 - u / w.t_out)
                } else {
                    0.0
                };
                out.push(Piston {
                    k: q.k,
                    a: q.a,
                    off: w.off(q.i),
                    ext,
                    stretch,
                });
            }
        }
        // ... then they pull home after the hit-stop
        if p.t_ko <= t && t < p.t_hs + 0.2 && t >= p.t_hs {
            for q in &mut out {
                q.ext *= py_max(0.0, 1.0 - (t - p.t_hs) / 0.2);
                q.stretch = 0.0;
            }
        }
        out
    }

    /// `hc_mode(t)`: 0..1: how far the trumpet has turned from "valves at the enemy" to "bell down the HIGH C
    /// line" (eased in as the charge starts, out after the blast).
    fn hc_mode(&self, t: f64) -> f64 {
        for c in &self.charges {
            let t_end = c.t_fire + self.hold + FADE;
            if c.t0 - 0.02 <= t && t < t_end + 0.2 {
                return ss(c.t0 - 0.02, c.t0 + 0.13, t) * (1.0 - ss(t_end, t_end + 0.2, t));
            }
        }
        0.0
    }

    /// `hc_aim(t)`: the HIGH C line's aim (the latest charge at or before `t`).
    fn hc_aim(&self, t: f64) -> Option<f64> {
        self.charges.iter().rfind(|c| c.t0 - 0.02 <= t).map(|c| c.aim)
    }

    /// `charge_at(t)`: the open charge at `t`.
    fn charge_at(&self, p: &PictureCtx, t: f64) -> Option<Charge> {
        self.charges
            .iter()
            .find(|c| c.t0 <= t && t < c.t_fire && t < p.t_ko)
            .copied()
    }

    /// `beams_live(t)`: the blasts visible at `t` (they freeze with the hit-stop, then fade).
    fn beams_live(&self, p: &PictureCtx, t: f64) -> Vec<Live> {
        if self.beam_t.is_empty() {
            return Vec::new();
        }
        let (j0, j1) = (
            search_left(&self.beam_t, t - 0.4),
            search_right(&self.beam_t, py_min(t, p.t_ko)),
        );
        let mut out = Vec::new();
        for b in self.beams.get(j0..j1).unwrap_or(&[]) {
            let tt = if t < p.t_hs {
                py_min(t, p.t_ko)
            } else {
                t - PictureCtx::HITSTOP
            };
            if let Some(st) = beam_state(tt, b.tf, self.hold) {
                out.push(Live {
                    k: b.k,
                    ox: b.ox,
                    oz: b.oz,
                    a: b.a,
                    ln: clip_len(b.ox, b.oz, b.a, b.ln),
                    wd: b.wd,
                    ext: st.0,
                    al: st.1,
                    st,
                });
            }
        }
        out
    }

    /// `solo_notes(t)`: the winner's solo: one floating note per solo note, from the bell: `(age, midi note)`.
    fn solo_notes(&self, p: &PictureCtx, t: f64) -> Vec<(f64, i32)> {
        p.solo_t
            .iter()
            .zip(&self.solo)
            .filter_map(|(&st, &(_, m, _))| {
                let u = t - st;
                (0.0..1.4).contains(&u).then_some((u, m))
            })
            .collect()
    }

    /// The sights of `now(F)`: the open charge's, with how far the charge has come (`fs[k]['charge']`).
    fn sights(&self, p: &PictureCtx, t: f64, fs: &[BallState; 2]) -> Vec<Sight> {
        if !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return Vec::new();
        }
        match self.charge_at(p, py_min(t, p.t_ko)) {
            Some(ch) => vec![Sight {
                k: self.k,
                a: ch.aim,
                ln: ch.ln,
                ck: fs[self.k].extra.get("charge").copied().unwrap_or(0.0),
            }],
            None => Vec::new(),
        }
    }

    /// `screen_angle(cam, p, th)`: the ring-frame direction `th` at `p`, as an angle on the screen (degrees).
    fn screen_angle(cam: &Cam, at: P2, th: f64) -> f64 {
        let s0 = cam.wp(&[world(at, R)])[0];
        let d = unit(th);
        let s1 = cam.wp(&[world([at[0] + d[0], at[1] + d[1]], R)])[0];
        // math.degrees
        dmath::atan2(s1.1 - s0.1, s1.0 - s0.0) * (180.0 / dmath::PI)
    }

    /// `trumpet_alpha(cam, st, t)`: the side-on trumpet's screen rotation (degrees, y down): valves up turned to
    /// the punch aim, blended toward "bell down the HIGH C line" by the HIGH C mode.
    fn trumpet_alpha(&self, cam: &Cam, st: &BallState, t: f64) -> f64 {
        let a_valves = Self::screen_angle(cam, st.p, st.th) + 90.0;
        let hc = st.extra.get("hc").copied().unwrap_or(0.0);
        if hc <= 0.0 {
            return a_valves;
        }
        // (a HIGH C mode above 0 means a charge at or before t; Python raises without one)
        let Some(aim) = self.hc_aim(t) else { return a_valves };
        let a_bell = Self::screen_angle(cam, st.p, aim);
        a_valves + (py::fmod_py(a_bell - a_valves + 180.0, 360.0) - 180.0) * hc
    }

    /// `int(round(self.cfg['view'][0] / 6.0)) * 6 % 360`: the yaw every sprite is drawn at.
    fn yaw_key(&self) -> i64 {
        py::imod_py(py::round_half_even(self.view_yaw / 6.0) as i64 * 6, 360)
    }
}

/// `clip_len(ox, oz, a, ln, lim=3.25)`: the visible blast ends at the ropes (it stays inside the ring, and inside
/// the lens's sharp band).
fn clip_len(ox: f64, oz: f64, a: f64, ln: f64) -> f64 {
    let lim = 3.25;
    let d = unit(a);
    let mut s_max = ln;
    for (o, dv) in [(ox, d[0]), (oz, d[1])] {
        if dv.abs() > 1e-9 {
            s_max = py_min(s_max, ((if dv > 0.0 { lim } else { -lim }) - o) / dv);
        }
    }
    py_max(0.2, s_max)
}

/// `fire_centre(ox, oz, a)`: the trumpet ball's centre when it fired (the engine's beam origin sits R + 0.04 out
/// along the aim).
fn fire_centre(ox: f64, oz: f64, a: f64) -> P2 {
    let d = unit(a);
    [ox - (R + 0.04) * d[0], oz - (R + 0.04) * d[1]]
}

impl Look for TrumpetLook {
    fn shards(&self) -> Vec<String> {
        // ['tube'] * 4 + ['valve'] * 3 + ['brass'] * 6
        [("tube", 4), ("valve", 3), ("brass", 6)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "beam"]
    }

    fn spark_flash(&self) -> &[&str] {
        &["beam"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "beam"]
    }

    fn title_pattern(&self, base: &mut RgbF) {
        trumpet_valves(base);
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `setup_as` took from `cfg`, by the keys `trumpet.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.low.hold={:?}; abilities.unlock2.mult={:?}; pp={}; view.0={:?}; view.2={:?}; weapon={:?}",
            self.hold, self.mult, self.pp, self.view_yaw, self.view_roll, self.w
        ))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: HIGH C's `hold`, DOUBLE TONGUE's `mult`, the weapon's
    /// numbers and the view come from `cfg`, the rest from side `k`.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        let mine = |e: &&Event| e.who() == Some(k);
        // beams: charges (t0, owner, aim, t_fire, length, width) and blasts (t_fire, owner, ox, oz, aim, length,
        // width)
        self.charges = ev
            .iter()
            .filter(|e| e.kind == "charge")
            .filter(mine)
            .filter_map(|e| {
                Some(Charge {
                    t0: e.t,
                    aim: num(e, 3)?,
                    t_fire: e.t + num(e, 4)?,
                    ln: num(e, 5)?,
                })
            })
            .collect();
        self.beams = ev
            .iter()
            .filter(|e| e.kind == "beam")
            .filter(mine)
            .filter_map(|e| {
                Some(Blast {
                    tf: e.t,
                    k,
                    ox: num(e, 3)?,
                    oz: num(e, 4)?,
                    a: num(e, 5)?,
                    ln: num(e, 6)?,
                    wd: num(e, 7)?,
                })
            })
            .collect();
        self.beam_t = self.beams.iter().map(|b| b.tf).collect();
        // HIGH C (an ability beam) holds longer
        self.hold = cfg.low.get("hold").unwrap_or(0.12);
        // VALVE COMBO: every punch (t0, owner, index, aim, reach, combo no., haste time scale)
        self.mult = cfg.unlock2.get("mult").unwrap_or(1.0);
        let mult = self.mult;
        self.punches = ev
            .iter()
            .filter(|e| e.kind == "punch")
            .filter(mine)
            .filter_map(|e| {
                // HA[min(len(HA) - 1, max(0, int(round((e[0] - T.T_FIGHT) * FPS)))), e[2]]
                let hasted = p.run.f("HA", p.sim_i(e.t), k).unwrap_or(0.0) > 0.0;
                Some(Punch {
                    t0: e.t,
                    k,
                    i: int(e, 3)?,
                    a: num(e, 4)?,
                    reach: num(e, 5)?,
                    ks: 1.0 / if hasted { mult } else { 1.0 },
                })
            })
            .collect();
        self.punch_t = self.punches.iter().map(|q| q.t0).collect();
        self.w = match &cfg.weapon {
            WeaponCfg::Combo(w) => Some(Valves {
                t_out: w.t_out,
                t_hold: w.t_hold,
                t_back: w.t_back,
                punches: w.punches,
                spread: w.spread,
            }),
            _ => None,
        };
        let view = |i: usize| -> f64 {
            cfg.raw
                .get("view")
                .and_then(|v| v.get(i))
                .and_then(|r| r.as_f64())
                .expect("a roster entry the picture accepted has a view (yaw, swing, roll)")
        };
        self.view_yaw = view(0);
        self.view_roll = view(2);
        self.pp = cfg.pp.clone();
        // the kit's models are named by `pp`; the app's by the fighter whose roster entry holds that `pp`
        self.mesh = crate::picture::mesh(&cfg.name);
        // (the kit's atlas is one module-level table for every TRUMPET look of the process: a look set up again
        // starts its own)
        self.atlas.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.solo = p.solos[k].clone();
    }

    // ------------------------------------------------------------ the ball
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, gl: f64) {
        if kk != self.k {
            return;
        }
        let (mut charge, mut hc) = (0.0, 0.0);
        if phase == Phase::Fight {
            // HIGH C: the bell glows as it charges, flares as it fires; every punch thumps the ball
            let tt = py_min(t, p.t_ko);
            hc = self.hc_mode(tt);
            if let Some(ch) = self.charge_at(p, tt) {
                charge = (tt - ch.t0) / py_max(1e-3, ch.t_fire - ch.t0);
            }
            let fire = match search_right(&self.beam_t, tt).checked_sub(1) {
                Some(j) => dmath::exp(-(tt - self.beam_t[j]) / 0.18),
                None => 0.0,
            };
            let thump = match search_right(&self.punch_t, tt).checked_sub(1) {
                Some(j) => dmath::exp(-(tt - self.punch_t[j]) / 0.08),
                None => 0.0,
            };
            // st['glow'] = max(0.5 * st['charge'] ** 2, 0.8 * fire, 0.3 * thump)
            st.glow = py_max(py_max(0.5 * (charge * charge), 0.8 * fire), 0.3 * thump);
            charge = py_max(dmath::pow(charge, 1.5), fire);
        } else if phase == Phase::Tail && kk == p.win {
            // the bell glows on every solo note
            charge = p.solo_env(t) * (1.0 - gl);
        }
        st.extra.insert("charge".to_string(), charge);
        st.extra.insert("hc".to_string(), hc);
    }

    /// The trumpet turns its valves toward the punches (the pistons come out of them); for HIGH C it swings its
    /// bell down the line (any roll an enemy's debuff adds rides on top).
    fn ball(&self, p: &PictureCtx, kk: usize, st: &BallState, t: f64, glass: [f64; 3], roll: f64) -> ([f64; 3], f64) {
        if kk != self.k {
            return (glass, roll);
        }
        let extra = roll - self.view_roll;
        let alpha = self.trumpet_alpha(&p.camera(t), st, t);
        // rl = int(round((-alpha + extra) / ROLL_STEP)) * ROLL_STEP; (rl + 180) % 360 - 180
        let rl = py::round_half_even((-alpha + extra) / ROLL_STEP as f64) as i64 * ROLL_STEP;
        (glass, (py::imod_py(rl + 180, 360) - 180) as f64)
    }

    /// Its own in-ball sprite: always side-on at its view's yaw (no sway), at the roll `ball()` set.
    fn sprite(&self, r: u32, _yaw: f64, roll: f64) -> Option<Img<u8>> {
        let key = (r.clamp(SPR_RADII.0, SPR_RADII.1), roll as i64);
        let mut atlas = self.atlas.lock().unwrap_or_else(|e| e.into_inner());
        // a look not set up yet (no roster entry given) draws the TRUMPET's own model, at its view's yaw 0
        let mesh = match &self.mesh {
            Some(mesh) => mesh,
            None if self.pp.is_empty() => own_mesh(),
            None => panic!("the TRUMPET's look has no model to draw for {:?}", self.pp),
        };
        let sprite = atlas
            .entry(key)
            .or_insert_with(|| render_sprite(mesh, key.0, self.yaw_key() as f64, key.1 as f64));
        Some(sprite.clone())
    }

    // ------------------------------------------------------------ events
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "hit" {
            return match how(e) {
                "piston" => (self.w.is_some_and(|w| int(e, 8) == Some(w.punches - 1))).then_some(0.9),
                "beam" | "slam" => Some(2.5),
                "shockwave" | "wave" => Some(1.5),
                _ => None,
            };
        }
        if e.kind == "beam" {
            return Some(1.2);
        }
        DefaultLook.shake(p, e)
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let what = text(e, 7);
        let (n, sp, life) = if self.spark_big().contains(&what) {
            (16, 3.2, 0.3)
        } else if what == "piston" {
            (9, 2.6, 0.22)
        } else {
            (5, 1.8, 0.18)
        };
        let col = p.col[e.who().filter(|&k| k < 2).unwrap_or(0)];
        (col.map(|c| c * 0.6 + 100.0), n, sp, life)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match text(e, 7) {
            "beam" => Some((8.0, 0.22)),
            "slam" | "wave" | "shockwave" => Some((14.0, 0.35)),
            _ => None,
        }
    }

    // ------------------------------------------------------------ effects
    /// VALVE COMBO: steam bursts from the valve as each piston fires.
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let Some(w) = &self.w else { return };
        for q in &self.punches {
            let at = p.run.p(p.sim_i(q.t0), q.k);
            let (a0, _tip, d) = piston_geom(at, q.a, w.off(q.i), 0.0);
            let emit = Emit {
                n: Some(3),
                dir: [0.0, 1.0, 0.0],
                vel: wdir([d[0] * 0.6, d[1] * 0.6]),
                ..Emit::default()
            };
            fx.pa.emit(q.t0, world(a0, R * 1.15), "steam", &emit);
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let t = e.t;
        let Some(att) = e.who().filter(|&k| k < 2) else { return };
        if e.kind == "hit" {
            let (Some(x), Some(z)) = (num(e, 5), num(e, 6)) else {
                return;
            };
            let pos = up(x, z, R);
            match text(e, 7) {
                "piston" => {
                    let big = self.w.is_some_and(|w| int(e, 8) == Some(w.punches - 1));
                    fx.bu.add(t, pos, p.col[att], if big { 1.2 } else { 0.7 }, None, big);
                    fx.fl.add(
                        t,
                        pos,
                        p.fl_col[att],
                        1.4,
                        if big { 0.85 } else { 0.55 },
                        0.03,
                        0.2,
                        0.0,
                    );
                    fx.pa.emit(t, pos, "brass", &count(if big { 8 } else { 4 }));
                    fx.pa.emit(t, pos, "steam", &count(2));
                    fx.mk.add(
                        t,
                        Mark {
                            size: if big { 0.22 } else { 0.14 },
                            col: [150.0, 110.0, 30.0],
                            life: 2.5,
                            // seed=int(e[0] * 100)
                            seed: (t * 100.0) as u64,
                            strength: 0.6,
                            ..Mark::new(MarkKind::Scorch, x, z)
                        },
                    );
                }
                "beam" => {
                    // HIGH C lands
                    fx.bu.add(t, pos, p.col[att], 2.0, None, true);
                    fx.fl.add(t, pos, p.fl_col[att], 2.4, 1.2, 0.03, 0.4, 0.0);
                    let emit = Emit {
                        n: Some(18),
                        speed: Some((2.0, 4.5)),
                        ..Emit::default()
                    };
                    fx.pa.emit(t, pos, "brass", &emit);
                }
                "slam" => fx.bu.add(t, pos, p.col[att], 2.0, None, true),
                "shockwave" | "wave" => fx.bu.add(t, pos, p.col[att], 1.4, None, true),
                _ => {}
            }
        } else if e.kind == "beam" {
            // HIGH C fires: the bell flares, the line scorches
            let (Some(ox), Some(oz), Some(a), Some(ln), Some(wd)) =
                (num(e, 3), num(e, 4), num(e, 5), num(e, 6), num(e, 7))
            else {
                return;
            };
            let d = unit(a);
            for u in [0.15, 0.45, 0.75] {
                let q = [ox + d[0] * ln * u, oz + d[1] * ln * u];
                fx.fl.add(t, world(q, R), p.fl_col[att], 2.2, 0.7, 0.03, 0.35, 0.15);
                fx.mk.add(
                    t,
                    Mark {
                        size: 0.5 * wd + 0.2,
                        angle: a,
                        col: [170.0, 120.0, 30.0],
                        life: 6.0,
                        // seed=int(tf * 10 + u * 7)
                        seed: (t * 10.0 + u * 7.0) as u64,
                        strength: 0.8,
                        ..Mark::new(MarkKind::Scorch, q[0], q[1])
                    },
                );
            }
            let emit = Emit {
                n: Some(14),
                dir: wdir(d),
                spread: Some(40.0),
                speed: Some((2.0, 4.0)),
                ..Emit::default()
            };
            fx.pa.emit(t, up(ox, oz, R), "brass", &emit);
        } else if e.kind == "ability" && text(e, 4) == "shockwave" {
            // FANFARE: the ball flares gold
            let at = p.run.p(p.sim_i(t), att);
            fx.fl
                .add(t, up(at[0], at[1], 1.0), p.fl_col[att], 2.6, 0.9, 0.03, 0.45, 0.0);
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let brass = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((1.0, 3.0)),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "brass", &brass);
        let steam = Emit {
            n: Some(14),
            spread: Some(180.0),
            speed: Some((0.4, 1.2)),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "steam", &steam);
    }

    /// The champion's solo: a puff of steam from the horn on every note (effects clock).
    fn fx_end(&self, p: &PictureCtx, fx: &mut Fx) {
        if self.k != p.win {
            return;
        }
        for &t0 in &p.solo_t {
            if t0 < p.show.grid.t_reassemble {
                let c = p.fighter(p.win, t0).p;
                let at = t0 - PictureCtx::HITSTOP;
                fx.pa.emit(at, up(c[0], c[1], R * 1.6), "steam", &count(2));
                fx.fl
                    .add(at, up(c[0], c[1], 1.0), p.fl_col[p.win], 1.6, 0.5, 0.03, 0.2, 0.0);
            }
        }
    }

    // ------------------------------------------------------------ the canvas
    /// Each blast throws a streak of light on the canvas; the charging bell lights a faint lane.
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let bayer = bayer_full();
        for b in self.beams_live(p, c.t) {
            let d = unit(b.a);
            let bm = bell_mouth(fire_centre(b.ox, b.oz, b.a), b.a);
            let lv = (b.ln + R + 0.04 - BELL_IN * R) * b.ext;
            let lo = lv * b.st.2;
            let span = 0.35 + b.wd * 1.5;
            let tint = p.col[b.k].map(|v| v / 255.0 * 0.9 + 0.3);
            for (j, light) in ic.iter_mut().enumerate() {
                let (rx, rz) = (c.sxp[j] - bm[0], c.szp[j] - bm[1]);
                let along = np::clip(rx * d[0] + rz * d[1], lo, lv);
                let off = np::hypot(rx - along * d[0], rz - along * d[1]);
                let band = dmath::pow(np::clip(1.0 - off / span, 0.0, 1.0), 1.5);
                let band = (band * 4.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 4.0;
                let v = band * b.al * 0.75;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
        for s in self.sights(p, c.t, c.fs) {
            let st = &c.fs[s.k];
            let d = unit(s.a);
            let tint = p.col[s.k].map(|v| v / 255.0);
            for (j, light) in ic.iter_mut().enumerate() {
                let (rx, rz) = (c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                let along = rx * d[0] + rz * d[1];
                let off = np::hypot(rx - along * d[0], rz - along * d[1]);
                let lane = along > R && along < R + s.ln && off < 0.06;
                let v = if lane { 1.0 } else { 0.0 } * 0.18 * s.ck;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
    }

    /// FANFARE's ring of light on the canvas (a shockwave band; a wave would be jagged and shivering).
    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind == "pulse" {
            return;
        }
        let shock = w.kind == "shockwave";
        let wave = w.kind == "wave";
        let wdt = if shock { 0.16 } else { 0.32 };
        let k_ = (1.0 - w.r / w.rmax) * if shock { 1.0 } else { 1.3 };
        let tint = p.col[w.owner].map(|v| v / 255.0 * 1.4 + if wave { 0.35 } else { 0.15 });
        let bayer = bayer_full();
        for (j, light) in ic.iter_mut().enumerate() {
            let (dx, dz) = (c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let mut rd = np::hypot(dx, dz);
            if wave {
                let ang = dmath::atan2(dz, dx);
                rd = rd
                    + 0.10 * (2.0 * py::fmod_py(7.0 * ang / dmath::PI + 6.0 * c.t, 1.0) - 1.0)
                    + 0.04 * dmath::sin(23.0 * ang - 50.0 * c.t);
            }
            let band = np::clip(1.0 - (rd - w.r).abs() / wdt, 0.0, 1.0);
            let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
            let v = band * k_ * 0.9;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    /// The pistons' shadows from the spot.
    fn shadows_free(&self, p: &PictureCtx, f: &FrameCtx<'_>, spot: V3) -> Vec<Poly> {
        let mut polys = Vec::new();
        for q in self.punches_live(p, f.t) {
            let st = &f.fs[q.k];
            if st.vis && !st.shattered {
                let (a0, tip, d) = piston_geom(st.p, q.a, q.off, q.ext);
                let n = [-d[1] * 0.05, d[0] * 0.05];
                let quad = [
                    [a0[0] - n[0], a0[1] - n[1]],
                    [tip[0] - n[0], tip[1] - n[1]],
                    [tip[0] + n[0], tip[1] + n[1]],
                    [a0[0] + n[0], a0[1] + n[1]],
                ]
                .map(|c| world(c, R));
                polys.push(proj_point(&quad, spot, 0.93));
            }
        }
        polys
    }

    // ------------------------------------------------------------ the painter pass, effects over the ring
    /// VALVE COMBO: the pistons out right now, each at its own depth.
    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let mut out: Vec<(f64, Payload)> = Vec::new();
        for q in self.punches_live(p, f.t) {
            let st = &f.fs[q.k];
            if st.vis && !st.shattered {
                let (a0, tip, _) = piston_geom(st.p, q.a, q.off, q.ext);
                let mid = [(a0[0] + tip[0]) / 2.0, (a0[1] + tip[1]) / 2.0];
                let piston: Payload = Box::new(q);
                out.push((f.cam.depth(&[world(mid, R)])[0], piston));
            }
        }
        out
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(q) = payload.downcast_ref::<Piston>() else {
            return;
        };
        let (mut arr, depth) = (fr.array(), fr.depth());
        draw_piston(&mut arr, f.cam, f.fs[q.k].p, q.a, q.off, q.ext, q.stretch, p.col[q.k]);
        fr.set_array(&arr, Some(&depth));
    }

    /// FANFARE's banners, the HIGH C sight and blasts, the champion's solo notes rising from the bell.
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let (cam, t) = (f.cam, f.t);
        if f.in_fight {
            for w in p.run.waves(f.i) {
                if w.kind == "shockwave" && w.owner == self.k {
                    let k = py_min(1.0, (1.0 - w.r / w.rmax) * 1.6);
                    draw_banners(arr, cam, [w.cx, w.cz], w.r, k, t);
                }
            }
        }
        for s in self.sights(p, t, f.fs) {
            draw_sight(arr, cam, f.fs[s.k].p, s.a, s.ln, s.ck, t, p.col[s.k]);
        }
        for b in self.beams_live(p, t) {
            let pc = fire_centre(b.ox, b.oz, b.a);
            draw_beam(arr, cam, pc, b.a, b.ln, b.wd, b.st, p.col[b.k]);
            draw_flash(arr, cam, pc, b.a, py_max(0.0, 1.0 - b.st.3 / 0.10), p.col[b.k]);
        }
        if self.k == p.win {
            let st = &f.fs[p.win];
            for (u, m) in self.solo_notes(p, t) {
                let org = bell_mouth(st.p, st.th);
                let side = 0.25 * dmath::sin(f64::from(m) * 1.7);
                draw_note_rise(
                    arr,
                    cam,
                    org[0] + side * u,
                    org[1] - side * u,
                    R + 0.4 + 1.3 * u,
                    p.col[p.win],
                    1.0 - u / 1.4,
                );
            }
        }
    }

    // ------------------------------------------------------------ shards: tubing, valve caps, brass chips
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
        let on_screen = |pts: &[P2]| -> Poly {
            let w: Vec<V3> = pts.iter().map(|&q| world(q, pos[1])).collect();
            screen(cam, &w)
        };
        match kind {
            "tube" => {
                // a bent length of brass tubing: a short arc, 2 px, brass with a bright edge
                let pts: Vec<P2> = (0..6)
                    .map(|j| {
                        let a = f64::from(j) * 0.5 * curl + 1.0;
                        let (c, s) = (dmath::cos(a), dmath::sin(a));
                        [
                            sx + d[0] * 0.22 * c + nn[0] * 0.22 * s,
                            sz + d[1] * 0.22 * c + nn[1] * 0.22 * s,
                        ]
                    })
                    .collect();
                let sp = on_screen(&pts);
                // im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8)); the lines; back as float32
                let bytes = arr.data.iter().map(|&v| np::clip_u8_f32(v)).collect();
                let mut im: Img<u8> = Img::from_vec(arr.w, arr.h, arr.c, bytes);
                let ink = |c: V3| [c[0] as u8, c[1] as u8, c[2] as u8];
                pil::line(&mut im, &sp, &ink(BRASS_DK), 2);
                pil::line(&mut im, &sp, &ink(BRASS_HI), 1);
                for (v, &b) in arr.data.iter_mut().zip(&im.data) {
                    *v = f32::from(b);
                }
            }
            "valve" => {
                // a valve cap: a small pearl-topped disc
                let rr = 0.09;
                let cen = scene::to_world(sx, sz, pos[1]);
                let disc = |n: usize, r: f64, dy: f64| -> Vec<V3> {
                    let us = linspace(0.0, 2.0 * dmath::PI, n + 1);
                    us[..n]
                        .iter()
                        .map(|&u| [cen[0] + r * dmath::cos(u), cen[1] + dy, cen[2] + r * dmath::sin(u)])
                        .collect()
                };
                layer_blend(arr, &[screen(cam, &disc(8, rr, 0.0))], BRASS, 1.0, None);
                layer_blend(arr, &[screen(cam, &disc(6, rr * 0.6, 0.01))], PEARL, 1.0, None);
            }
            _ => {
                // brass: tumbling flat chips of the bell / body, banded as they turn
                let s = size * 1.1;
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
                let shade_k = 0.7 + 0.45 * py_max(0.0, dmath::cos(rot * 2.0));
                layer_blend(arr, &[on_screen(&tri)], clip_col(BRASS.map(|c| c * shade_k)), 1.0, None);
            }
        }
    }

    // ------------------------------------------------------------ lens, glows
    /// Heat shimmer on FANFARE's ring.
    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        let c = scene::to_world(w.cx, w.cz, 0.05);
        img.shimmer_world(cam, c, w.r, 0.35, 2.6, t, 1.0 - w.r / w.rmax, w.owner as f64);
    }

    /// HIGH C: the air shivers along the blast.
    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        for b in self.beams_live(p, t) {
            let d = unit(b.a);
            for u in [0.25, 0.55, 0.85] {
                let q = [b.ox + d[0] * b.ln * b.ext * u, b.oz + d[1] * b.ln * b.ext * u];
                // seed=int(u * 10)
                let seed = (u * 10.0) as i64 as f64;
                img.shimmer_world(
                    cam,
                    world(q, R),
                    0.35 + 0.6 * b.wd * (1.0 + b.st.3),
                    0.3,
                    2.4,
                    t,
                    b.al,
                    seed,
                );
            }
        }
    }

    /// The bell mouth glows as it charges, fires and solos; glow along the blast.
    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _drawn: &[DrawnBall]) {
        let cam = f.cam;
        let st = &f.fs[self.k];
        if st.vis && !st.shattered {
            let ck = st.extra.get("charge").copied().unwrap_or(0.0);
            if ck > 0.05 {
                let hc = st.extra.get("hc").copied().unwrap_or(0.0);
                let a = if hc > 0.5 {
                    self.hc_aim(f.t).unwrap_or(st.th)
                } else {
                    st.th
                };
                let sm = cam.wp(&[world(bell_mouth(st.p, a), R)])[0];
                fr.glow(
                    sm.0,
                    sm.1,
                    6.0 + 8.0 * ck,
                    p.col[self.k].map(|c| c * 0.6 + 100.0),
                    0.35 + 0.55 * ck,
                );
            }
        }
        for b in self.beams_live(p, f.t) {
            let d = unit(b.a);
            for u in [0.15, 0.5, 0.85] {
                // q = np.array([ox, oz]) + u * ln * ext * np.array([cos, sin])
                let k = u * b.ln * b.ext;
                let s = cam.wp(&[world([b.ox + k * d[0], b.oz + k * d[1]], R)])[0];
                let inten = if b.wd < 0.6 { 0.35 } else { 0.22 } * b.al;
                fr.glow(s.0, s.1, 10.0 + 8.0 * b.wd, p.col[b.k].map(|c| c * 0.7 + 70.0), inten);
            }
        }
    }

    // ------------------------------------------------------------ checks
    /// The bell mouth, every beam tip, every piston head.
    fn focus_points(&self, p: &PictureCtx, t: f64, st: &BallState) -> Vec<(V3, String)> {
        let mut out = Vec::new();
        out.push((world(bell_mouth(st.p, st.th), R), "bell".to_string()));
        for b in self.beams_live(p, t) {
            let at = [
                b.ox + b.ln * b.ext * dmath::cos(b.a),
                b.oz + b.ln * b.ext * dmath::sin(b.a),
            ];
            out.push((world(at, R), "beam tip".to_string()));
        }
        for q in self.punches_live(p, t) {
            let (_a0, tip, _d) = piston_geom(st.p, q.a, q.off, q.ext);
            out.push((world(tip, R), "piston".to_string()));
        }
        out
    }
}
