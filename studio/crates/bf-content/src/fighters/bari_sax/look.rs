//! BARI SAX's look: the port of `Look` in its reference kit module (`tools/studio_export/older/fighters/bari_sax.py`,
//! moved from `projects/ballfight_bassoon_vs_barisax/` (#39 v4) and proven against it by Gate O), and of the
//! episode's `styles.py` functions it draws with, hook by hook.
//!
//! What it draws: the ball wears a leather harness (straps and a steel ring on the side facing the cable); every
//! WRECKING BALL cycle a crane line drops from the spot's rig, its hook block stops over the swing's bottom, the hook
//! bites the harness ring, the ball is hoisted back and swung through a pendulum arc drawn 1.6 x taller (a motion
//! smear and a coral trail under the ball, a coral light at the bottom), smashing what is in front of it; then it
//! unhooks and the block flies back up. Out over the ropes the ropes are pressed down under it and spring back when it
//! lifts off; up over a front rope it is drawn after the front ropes. GROWL: a gravelly ring on the canvas, its inked
//! front, a heat shimmer, grit. CABLE SNAP: hoisted high over the enemy's spot (its shadow and a pulsing danger ring
//! on the canvas), the cable snaps (steel sparks up high, the upper cable whipping away, a stub on the falling ball),
//! the crash (dust, grit, a crack and a scorch, a second jolt of the camera). The HUD holds SKANK lit for its beats and
//! CABLE SNAP until the crash. When a hop of the enemy's sails over the swinging ball, the crane is drawn under the
//! balls. It shatters into lacquer sticks, nickel key chips and bell chips.
//!
//! Its VS card panel is the card's own (`_barisax`, `bf_picture::title::patterns`).

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{self, H, W, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::ropes::RopeEv;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::{self, R_BALL as R};

use super::state::{self, Cycle, Snap, Src, Wreckage};
use crate::fighters::shared::reeds::{self, Num};

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

/// `VERT`: the crane's pendulum drawn 1.6 x taller (the view looks down at 58 degrees: heights read short).
const VERT: f64 = 1.6;
/// `LEAP_H`: a hop that sails over the swinging ball is drawn this high by the hopper.
const LEAP_H: f64 = 1.45;
/// `WOOD`: #39's shatter lacquer.
const WOOD: V3 = [234.0, 188.0, 86.0];
/// `NICKEL`: a key chip.
const NICKEL: V3 = [214.0, 216.0, 226.0];
/// `MAPLE_DUST`.
const MAPLE_DUST: [V3; 3] = [[214.0, 176.0, 140.0], [186.0, 140.0, 110.0], [240.0, 214.0, 186.0]];
/// `CORAL_SP`.
const CORAL_SP: [V3; 3] = [[255.0, 196.0, 170.0], [255.0, 118.0, 92.0], [255.0, 240.0, 220.0]];
/// `GRIT`.
const GRIT: [V3; 3] = [[60.0, 44.0, 40.0], [110.0, 84.0, 70.0], [170.0, 130.0, 110.0]];
/// #39 `_rope_wobbles`: `(axis, sign)` of rope sides 0 to 3.
const SIDES: [(usize, f64); 4] = [(1, 1.0), (0, 1.0), (1, -1.0), (0, -1.0)];

// ---------------------------------------------------------------- #39 styles.py
/// `INK`.
const INK: V3 = [18.0, 10.0, 26.0];
/// `WHITE`.
const WHITE: V3 = [255.0, 252.0, 240.0];
/// `STEEL`.
const STEEL: V3 = [132.0, 138.0, 156.0];
/// `STEEL_HI`.
const STEEL_HI: V3 = [226.0, 230.0, 242.0];
/// `LEATHER`.
const LEATHER: V3 = [62.0, 36.0, 32.0];
/// `Y_RIG`: the spot's truss; the crane line hangs from it.
const Y_RIG: f64 = 9.5;

/// `np.linspace(a, b, n)`: `i * step + a`, the last value `b` itself.
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    let step = (b - a) / (n - 1) as f64;
    (0..n)
        .map(|i| if i == n - 1 { b } else { i as f64 * step + a })
        .collect()
}

/// `np.clip(col * a + b, 0, 255)`.
fn tinted(col: V3, a: f64, b: f64) -> V3 {
    col.map(|c| np::clip(c * a + b, 0.0, 255.0))
}

/// `int(round(x))`: half to even.
fn round_px(x: f64) -> i64 {
    np::f64_to_i64(py::round_half_even(x))
}

/// `int(x)` of a time that is never negative, as a seed.
fn seed_of(x: f64) -> u64 {
    np::f64_to_i64(x).max(0) as u64
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

/// `d / (np.linalg.norm(d) + 1e-9)` of a 2-vector.
fn unit2(d: (f64, f64)) -> (f64, f64) {
    let n = (d.0 * d.0 + d.1 * d.1).sqrt() + 1e-9;
    (d.0 / n, d.1 / n)
}

/// `d / (np.linalg.norm(d) + 1e-9)` of a 3-vector.
fn unit3(d: V3) -> V3 {
    let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() + 1e-9;
    [d[0] / n, d[1] / n, d[2] / n]
}

fn add3(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale3(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// `blend(img, _mask([], lines, width), col, alpha)` of #39.
fn blend_lines(arr: &mut Img<f32>, lines: &[Vec<(f64, f64)>], width: u32, col: V3, alpha: f64) {
    if alpha <= 0.0 {
        return;
    }
    reeds::blend(arr, &reeds::mask(&[], lines, width), col, Num::Py(alpha));
}

/// `line3(img, cam, P, col, width, ink, hi, alpha)`: a world polyline as an inked steel line with a 1-px highlight.
#[allow(clippy::too_many_arguments)] // the Python signature
fn line3(arr: &mut Img<f32>, cam: &Cam, pts: &[V3], col: V3, width: u32, ink: f64, hi: f64, alpha: f64) {
    let sp = vec![cam.wp(pts)];
    if ink > 0.0 {
        blend_lines(arr, &sp, width + 2, INK, ink * alpha);
    }
    blend_lines(arr, &sp, width, col, alpha);
    if hi > 0.0 && width >= 2 {
        blend_lines(arr, &sp, 1, STEEL_HI, hi * alpha);
    }
}

/// `cable(a, b, n, sag, wig, ph)`: world points from `a` to `b`: straight (`sag`: slack, hanging down in the middle;
/// `wig`: a travelling wiggle, a snapped end whipping).
fn cable(a: V3, b: V3, n: usize, sag: f64, wig: f64, ph: f64) -> Vec<V3> {
    let pi = std::f64::consts::PI;
    let u = linspace(0.0, 1.0, n);
    let d = sub3(b, a);
    let mut p: Vec<V3> = u.iter().map(|&s| add3(a, scale3(d, s))).collect();
    for (q, &s) in p.iter_mut().zip(&u) {
        q[1] -= sag * dmath::sin(pi * s);
    }
    if wig != 0.0 {
        let side = unit3([-d[2], 0.0, d[0]]);
        for (q, &s) in p.iter_mut().zip(&u) {
            let w = wig * dmath::sin(pi * s * 3.0 + ph) * s;
            *q = add3(*q, scale3(side, w));
        }
    }
    p
}

/// `hook_block(img, cam, c, size, col)`: the crane's hook block at world point `c`: a small inked steel box.
fn hook_block(arr: &mut Img<f32>, cam: &Cam, c: V3) {
    let size = 0.24;
    let q: Vec<V3> = [
        (-size, -size * 1.2),
        (size, -size * 1.2),
        (size * 0.8, size),
        (-size * 0.8, size),
    ]
    .iter()
    .map(|&(dx, dy)| add3(c, [dx, dy, 0.0]))
    .collect();
    let sp = cam.wp(&q);
    reeds::blend(arr, &reeds::mask(std::slice::from_ref(&sp), &[], 1), INK, Num::Py(0.9));
    // sp.mean(0)
    let m = (
        (sp[0].0 + sp[1].0 + sp[2].0 + sp[3].0) / 4.0,
        (sp[0].1 + sp[1].1 + sp[2].1 + sp[3].1) / 4.0,
    );
    let inner: Vec<(f64, f64)> = sp
        .iter()
        .map(|s| (m.0 + (s.0 - m.0) * 0.7, m.1 + (s.1 - m.1) * 0.7))
        .collect();
    reeds::blend(
        arr,
        &reeds::mask(std::slice::from_ref(&inner), &[], 1),
        STEEL.map(|v| v * 0.85),
        Num::Py(1.0),
    );
    let top = vec![inner[2], inner[3]];
    reeds::blend(arr, &reeds::mask(&[], &[top], 1), STEEL_HI, Num::Py(0.7));
}

/// `hook_shape(img, cam, end, toward, size)`: a steel hook at world point `end` (the harness ring), opening away from
/// the cable's direction `toward`.
fn hook_shape(arr: &mut Img<f32>, cam: &Cam, end: V3, toward: V3) {
    let size = 0.14;
    let s0 = cam.wp(&[end])[0];
    let s1 = cam.wp(&[add3(end, scale3(toward, 0.2))])[0];
    let d = unit2((s1.0 - s0.0, s1.1 - s0.1));
    let n = (-d.1, d.0);
    let r = py_max(2.0, size * 18.0);
    let a = linspace(0.2 * std::f64::consts::PI, 1.6 * std::f64::consts::PI, 9);
    let mut pts = vec![(s0.0 + d.0 * r * 0.8, s0.1 + d.1 * r * 0.8)];
    for &ai in &a {
        // s0 + (cos(a) * n - sin(a) * d) * r * 0.6 - d * r * 0.3
        let (c, s) = (dmath::cos(ai), dmath::sin(ai));
        let x = s0.0 + (c * n.0 - s * d.0) * r * 0.6 - d.0 * r * 0.3;
        let y = s0.1 + (c * n.1 - s * d.1) * r * 0.6 - d.1 * r * 0.3;
        pts.push((x, y));
    }
    let lines = vec![pts];
    blend_lines(arr, &lines, 4, INK, 0.85);
    blend_lines(arr, &lines, 2, STEEL.map(|v| v * 1.1), 1.0);
}

/// `harness(img, s, rp, sx, sy, ring_dir, col)`: the bari's harness on its ball (screen space): two leather straps
/// over the top crossing at a steel ring on the side facing the cable (`ring_dir` a screen unit vector, `None` =
/// straight up), and a belt.
#[allow(clippy::too_many_arguments)] // the Python signature
fn harness(arr: &mut Img<f32>, s: (f64, f64), rp: f64, sx: f64, sy: f64, ring_dir: Option<(f64, f64)>, col: V3) {
    let pi = std::f64::consts::PI;
    let rd = ring_dir.unwrap_or((0.0, -1.0));
    // top = s + ring_dir * rp * [sx, sy] * 0.92
    let top = (s.0 + rd.0 * rp * sx * 0.92, s.1 + rd.1 * rp * sy * 0.92);
    let mut straps: Vec<Vec<(f64, f64)>> = Vec::new();
    let b = linspace(0.0, pi * 0.92, 12);
    for sgn in [-1.0, 1.0] {
        let perp = (-rd.1 * sgn, rd.0 * sgn);
        let pts = b
            .iter()
            .map(|&bi| {
                let (sb, cb) = (dmath::sin(bi), 1.0 - dmath::cos(bi));
                let x = top.0 + (perp.0 * sb * 0.62 - rd.0 * cb * 0.9) * rp * sx;
                let y = top.1 + (perp.1 * sb * 0.62 - rd.1 * cb * 0.9) * rp * sy;
                (x, y)
            })
            .collect();
        straps.push(pts);
    }
    let belt = linspace(0.15 * pi, 0.85 * pi, 12)
        .iter()
        .map(|&a| {
            (
                s.0 + dmath::cos(a) * rp * sx * 0.96,
                s.1 + dmath::sin(a) * rp * sy * 0.30 + rp * 0.08,
            )
        })
        .collect();
    straps.push(belt);
    reeds::blend(arr, &reeds::mask(&[], &straps, 5), INK, Num::Py(0.6));
    let leather = [0, 1, 2].map(|c| LEATHER[c] * 0.7 + col[c] * 0.3);
    reeds::blend(arr, &reeds::mask(&[], &straps, 3), leather, Num::Py(1.0));
    let dots: Vec<Vec<(f64, f64)>> = straps
        .iter()
        .map(|st| st.iter().step_by(3).copied().collect())
        .collect();
    reeds::blend(
        arr,
        &reeds::mask(&[], &dots, 1),
        col.map(|c| c * 0.6 + 90.0),
        Num::Py(0.5),
    );
    let (xi, yi) = (round_px(top.0), round_px(top.1));
    if 2 <= xi && xi < W as i64 - 2 && 2 <= yi && yi < H as i64 - 2 {
        let (xi, yi) = (xi as usize, yi as usize);
        for y in yi - 2..yi + 3 {
            for x in xi - 2..xi + 3 {
                arr.px_mut(x, y).copy_from_slice(&INK.map(|v| v as f32));
            }
        }
        for y in yi - 1..yi + 2 {
            for x in xi - 1..xi + 2 {
                arr.px_mut(x, y).copy_from_slice(&STEEL_HI.map(|v| v as f32));
            }
        }
        arr.px_mut(xi, yi).copy_from_slice(&STEEL.map(|v| v as f32));
    }
}

/// `swing_trail(img, pts, col, k)`: the swing's coral trail behind the ball: a tapering band through the recent
/// screen positions (oldest first).
fn swing_trail(arr: &mut Img<f32>, pts: &[(f64, f64)], col: V3, k: f64) {
    let n = pts.len();
    if n < 2 || k <= 0.02 {
        return;
    }
    let c = tinted(col, 0.8, 70.0);
    for j in 0..n - 1 {
        let u = (j + 1) as f64 / (n - 1) as f64;
        let w = round_px(2.0 + 14.0 * u).max(1) as u32;
        let seg = vec![vec![pts[j], pts[j + 1]]];
        blend_lines(arr, &seg, w, c, 0.65 * k * u);
        if w >= 4 {
            reeds::add(
                arr,
                &reeds::mask(&[], &seg, (w / 3).max(1)),
                WHITE.map(|v| v * 0.5),
                0.35 * k * u,
            );
        }
    }
}

/// `honk_light(...)` at one canvas point: GROWL's gravelly, broken ring (`shockwave`), LOW A's rumble (`quake`), a
/// honk's three ripples.
#[allow(clippy::too_many_arguments)] // the Python signature
fn growl_band(sxp: f64, szp: f64, cx: f64, cz: f64, r: f64, rmax: f64, kind: &str, t: f64) -> f64 {
    let rd = np::hypot(sxp - cx, szp - cz);
    let k = py_max(0.0, 1.0 - r / rmax);
    if kind == "quake" {
        let band = np::clip(1.0 - (rd - r).abs() / 0.45, 0.0, 1.0) * (0.6 + 0.4 * dmath::sin(rd * 9.0 - t * 30.0));
        return band * (0.4 + 0.6 * k);
    }
    if kind == "shockwave" {
        let ang = dmath::atan2(szp - cz, sxp - cx);
        let grit = 0.5 + 0.5 * sign(dmath::sin(ang * 23.0 + rd * 7.0));
        return np::clip(1.0 - (rd - r).abs() / 0.16, 0.0, 1.0) * (0.45 + 0.55 * grit) * k;
    }
    let mut out = 0.0;
    for (off, a) in [(0.0, 1.0), (0.32, 0.6), (0.62, 0.35)] {
        let rr = r - off;
        if rr > 0.0 {
            out = np::maximum(out, np::clip(1.0 - (rd - rr).abs() / 0.07, 0.0, 1.0) * a);
        }
    }
    out * k
}

/// `draw_honk_front(img, cam, cx, cz, r, rmax, col, y, kind)`: the ring's front as an inked coral ring just above
/// the canvas.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_honk_front(arr: &mut Img<f32>, cam: &Cam, cx: f64, cz: f64, r: f64, rmax: f64, col: V3, rattle: bool) {
    let k = py_max(0.0, 1.0 - r / rmax);
    if k <= 0.02 {
        return;
    }
    let a = linspace(0.0, 2.0 * std::f64::consts::PI, 61);
    let world: Vec<V3> = a
        .iter()
        .enumerate()
        .map(|(j, &ai)| {
            let wob = if rattle {
                0.04 * dmath::sin(ai * 11.0)
            } else {
                0.09 * if j % 2 == 1 { 1.0 } else { -0.8 }
            };
            scene::to_world(cx + (r + wob) * dmath::cos(ai), cz + (r + wob) * dmath::sin(ai), 0.3)
        })
        .collect();
    let lines = vec![cam.wp(&world)];
    blend_lines(arr, &lines, 3, INK, 0.45 * k);
    blend_lines(arr, &lines, 1, tinted(col, 0.75, 70.0), 0.9 * k);
}

/// `snap_target(...)` at one canvas point: `(shade, ring)`: the falling ball's shadow (grows and darkens as it comes
/// down, `u` 0 to 1) and a pulsing danger ring at the crater's reach.
#[allow(clippy::too_many_arguments)] // the Python signature
fn snap_target(sxp: f64, szp: f64, x: f64, z: f64, radius: f64, u: f64, t: f64) -> (f64, f64) {
    let rd = np::hypot(sxp - x, szp - z);
    let r_sh = R * (0.45 + 0.6 * u);
    let shade = np::clip(1.0 - (rd - r_sh) / 0.10, 0.0, 1.0) * (0.25 + 0.6 * u);
    let rr = R + radius;
    let pulse = 0.55 + 0.45 * dmath::cos(2.0 * std::f64::consts::PI * 6.0 * t);
    let ring = np::clip(1.0 - (rd - rr).abs() / 0.05, 0.0, 1.0) * pulse * (0.5 + 0.5 * u);
    let on = dmath::sin(dmath::atan2(szp - z, sxp - x) * 2.0).abs() > 0.92;
    let tick = np::clip(1.0 - (rd - rr * 0.55).abs() / 0.035, 0.0, 1.0) * if on { 1.0 } else { 0.0 };
    (shade, np::maximum(ring, tick * 0.8))
}

/// `hop_height(L)` of #39's TIPTOE (a hop's peak).
fn hop_height(l: f64) -> f64 {
    0.42 + 0.16 * py_min(2.5, l)
}

// ---------------------------------------------------------------- the look's state from the run
/// The enemy's hop, from its `hop` event: `(t, k, p0, p1, t_land, kind)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hop {
    t: f64,
    p0: [f64; 2],
    p1: [f64; 2],
    t_land: f64,
}

/// `crane_state(t)`.
#[derive(Clone, Debug, PartialEq)]
struct Crane {
    top: V3,
    block: V3,
    end: Option<V3>,
    attached: bool,
    pieces: Vec<(Vec<V3>, f64)>,
}

/// The painter item of this look: the crane under the balls.
struct CraneItem;

/// The picture's run as the derivations read it.
struct PicSrc<'a>(&'a PictureCtx);

impl Src for PicSrc<'_> {
    fn events(&self) -> &[Event] {
        &self.0.rec.events
    }
    fn t_fight(&self) -> f64 {
        self.0.show.grid.t_fight
    }
    fn t_ko(&self) -> f64 {
        self.0.t_ko
    }
    fn t_hs(&self) -> f64 {
        self.0.t_hs
    }
    fn s16(&self) -> f64 {
        self.0.show.grid.s16
    }
    fn sim_i(&self, t: f64) -> usize {
        self.0.sim_i(t)
    }
    fn p(&self, i: usize, k: usize) -> [f64; 2] {
        self.0.run.p(i, k)
    }
    fn row(&self, key: &str, i: usize, k: usize) -> Option<Vec<f64>> {
        match self.0.run.nested(key, i) {
            Some(frame) => state::nested_row(frame, k),
            None => self.0.run.row(key, i, k),
        }
    }
}

/// `_w(p, y)`.
fn w_at(p: [f64; 2], y: f64) -> V3 {
    scene::to_world(p[0], p[1], y)
}

/// BARI SAX's look: `Look` of `older/fighters/bari_sax.py`.
#[derive(Clone, Debug, Default)]
pub struct BariSaxLook {
    /// The roster numbers its bold moves read, from `setup_as`'s `cfg`.
    nums: bold::Nums,
    /// Its side.
    k: usize,
    /// `cfg['weapon']['kind'] == 'wrecking'`.
    wrecker: bool,
    /// `cfg['weapon']['lift16']` (0 when not a wrecking ball).
    lift16: f64,
    /// `cfg['weapon']['base']`.
    base: f64,
    /// `SMASHES` of its own: `(t, speed)`.
    smashes: Vec<(f64, f64)>,
    /// `E_HOPS`: the enemy's hops.
    e_hops: Vec<Hop>,
    /// `CYCLES`, `SNAPS` and the air pose ([`Wreckage`]).
    w: Wreckage,
    /// `ability_hold`'s holds, per slot (None: the kit's).
    holds: [Option<f64>; 3],
}

impl BariSaxLook {
    /// `cycle_at(t, tail)` ([`Wreckage::cycle_at`]).
    fn cycle_at(&self, t: f64, tail: f64) -> Option<&Cycle> {
        self.w.cycle_at(t, tail)
    }

    /// `wreck_pose(c, t)` ([`state::wreck_pose`]).
    fn wreck_pose(c: Option<&Cycle>, t: f64) -> Option<([f64; 2], f64, bool)> {
        state::wreck_pose(c, t)
    }

    /// `snap_at(t)` ([`Wreckage::snap_at`]).
    fn snap_at(&self, p: &PictureCtx, t: f64) -> Option<(&Snap, bool, f64)> {
        self.w.snap_at(p.t_ko, t)
    }

    /// `air_pose(t)` ([`Wreckage::air_pose`]).
    fn air_pose(&self, p: &PictureCtx, t: f64) -> Option<([f64; 2], f64)> {
        self.w.air_pose(&PicSrc(p), t)
    }

    /// `h_ko()`: its height at the K.O.
    fn h_ko(&self, p: &PictureCtx) -> f64 {
        self.air_pose(p, p.t_ko - 1e-4).map_or(0.0, |a| a.1)
    }

    /// `e_hop_at(t)`: the enemy's hop in the air at `t`.
    fn e_hop_at(&self, p: &PictureCtx, t: f64) -> Option<&Hop> {
        if self.e_hops.is_empty() || t >= p.t_ko || t < p.show.grid.t_fight {
            return None;
        }
        let j = self.e_hops.partition_point(|h| h.t <= t).checked_sub(1)?;
        let h = &self.e_hops[j];
        if h.t <= t && t < h.t_land { Some(h) } else { None }
    }

    /// `leaps(hop)`: does a hop of the enemy's pass over the swinging ball?
    fn leaps(&self, hop: &Hop) -> bool {
        if !self.wrecker {
            return false;
        }
        for u in linspace(0.1, 0.9, 9) {
            let tt = hop.t + (hop.t_land - hop.t) * u;
            if let Some((q, _, true)) = Self::wreck_pose(self.cycle_at(tt, 0.0), tt) {
                let at = [
                    hop.p0[0] + (hop.p1[0] - hop.p0[0]) * u,
                    hop.p0[1] + (hop.p1[1] - hop.p0[1]) * u,
                ];
                if np::hypot(at[0] - q[0], at[1] - q[1]) < 2.0 * R * 1.1 {
                    let l = np::hypot(hop.p1[0] - hop.p0[0], hop.p1[1] - hop.p0[1]);
                    return py_max(1.0, LEAP_H / hop_height(l)) > 1.0;
                }
            }
        }
        false
    }

    /// `crane_first(t)`: a leap over the swing passes in front of the cable: the crane goes under the balls.
    fn crane_first(&self, p: &PictureCtx, t: f64) -> bool {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return false;
        }
        self.e_hop_at(p, t).is_some_and(|h| self.leaps(h))
    }

    /// `bari_over(t)`: the centre and the underside height of the bari while it is up and its footprint reaches the
    /// rope line.
    fn bari_over(&self, p: &PictureCtx, t: f64) -> Option<([f64; 2], f64)> {
        if !self.wrecker {
            return None;
        }
        self.w.over(&PicSrc(p), t)
    }

    /// `cycle_end_ring(c)`: where the hook let go at the end of cycle `c`.
    fn cycle_end_ring(&self, p: &PictureCtx, c: &Cycle) -> V3 {
        let Some((q, h, _)) = Self::wreck_pose(Some(c), c.end - 1e-4) else {
            let at = p.run.p(p.sim_i(c.end), self.k);
            return w_at(at, 2.0 * R);
        };
        let pv = match c.sw {
            Some(sw) => w_at(sw.bot, R + VERT * sw.l),
            None => w_at(q, 2.0 * R + 1.0),
        };
        let cc = w_at(q, R + h);
        add3(cc, scale3(unit3(sub3(pv, cc)), R))
    }

    /// `crane_state(t)`: the crane's world geometry at `t`, frozen through the hit-stop, flying back up after it.
    fn crane_state(&self, p: &PictureCtx, t: f64) -> Option<Crane> {
        let k = self.k;
        if (!self.wrecker && !self.w.snapper) || t < p.show.grid.t_fight {
            return None;
        }
        if t >= p.t_hs {
            let mut st = self.crane_state(p, p.t_ko - 1e-4)?;
            let v = (t - p.t_hs) / 0.6;
            if v >= 1.0 {
                return None;
            }
            let mut b = st.block;
            b[1] += (Y_RIG - b[1]) * v * v;
            st.block = b;
            st.end = Some(sub3(b, [0.0, 0.42, 0.0]));
            st.attached = false;
            st.pieces = st.pieces.into_iter().map(|(q, a)| (q, a * (1.0 - v))).collect();
            return Some(st);
        }
        let tf = py_min(t, p.t_ko - 1e-4);
        // (snap, phase 0 hoist / 1 fall / 2 after the crash, u)
        let mut sn = self
            .snap_at(p, tf)
            .map(|(q, hoist, u)| (*q, if hoist { 0 } else { 1 }, u));
        if sn.is_none() {
            for q in &self.w.snaps {
                if q.t1 <= tf && tf < q.t1 + 0.6 {
                    sn = Some((*q, 2, (tf - q.t1) / 0.6));
                }
            }
        }
        if let Some((q, ph, u)) = sn {
            let i = p.sim_i(tf);
            let at = p.run.p(i, k);
            let h = if ph != 2 {
                PicSrc(p).row("SNAP", i, k).map_or(0.0, |r| r[1])
            } else {
                0.0
            };
            let c = w_at(at, R + h);
            let ring = add3(c, [0.0, R, 0.0]);
            if ph == 0 {
                let mut tgt = add3(ring, [0.0, 1.1, 0.0]);
                let prev = self
                    .w
                    .cycles
                    .iter()
                    .find(|c| c.sw.is_some() && (c.cut - q.t0).abs() < 1e-3);
                if let Some(pc) = prev
                    && u < 0.3
                {
                    // out of a swing: the block slides over from its pivot
                    let sw = pc.sw.expect("a swing");
                    let pv = w_at(sw.bot, R + VERT * sw.l);
                    tgt = add3(pv, scale3(sub3(tgt, pv), ss(0.0, 0.3, u)));
                }
                return Some(Crane {
                    top: [tgt[0], Y_RIG, tgt[2]],
                    block: tgt,
                    end: Some(ring),
                    attached: true,
                    pieces: Vec::new(),
                });
            }
            // snapped: the upper cable whips up and away, a stub flops on the falling ball
            let tsn = tf - q.ts;
            let i_s = p.sim_i(q.ts);
            let ps_ = p.run.p(i_s, k);
            let hs = PicSrc(p).row("SNAP", i_s, k).map_or(0.0, |r| r[1]);
            let b0 = w_at(ps_, R + hs + R + 1.1);
            let blk = add3(b0, [0.35 * tsn, 7.0 * tsn + 6.0 * tsn * tsn, 0.0]);
            let brk = sub3(blk, [0.0, 0.55, 0.0]);
            let up_piece = cable(
                blk,
                add3(brk, [0.2 * dmath::sin(tsn * 30.0), 0.0, 0.0]),
                6,
                0.0,
                0.12 * dmath::exp(-tsn / 0.3),
                tsn * 40.0,
            );
            let stub_top = add3(
                ring,
                [0.15 * dmath::sin(tsn * 22.0), 0.5 * dmath::exp(-tsn / 0.4) + 0.08, 0.1],
            );
            let stub = cable(ring, stub_top, 5, 0.0, 0.06, tsn * 25.0);
            let fade = 1.0 - if ph == 2 { u } else { 0.0 };
            return Some(Crane {
                top: [blk[0], Y_RIG, blk[2]],
                block: blk,
                end: None,
                attached: false,
                pieces: vec![(up_piece, py_max(0.0, 1.0 - tsn / 0.7)), (stub, fade)],
            });
        }
        let c = self.cycle_at(tf, 0.55)?;
        let pv = match c.sw {
            Some(sw) => w_at(sw.bot, R + VERT * sw.l),
            None => w_at(p.run.p(p.sim_i(c.t0), k), R + VERT * self.base),
        };
        let i = p.sim_i(tf);
        let (at, h) = match Self::wreck_pose(Some(c), tf) {
            Some((q, h, _)) => (q, h),
            None => (p.run.p(i, k), 0.0),
        };
        let cc = w_at(at, R + h);
        if tf < c.t_hook && tf < c.end {
            // the drop: the block comes down, the hook reaches
            let u = (tf - c.t0) / py_max(1e-6, c.t_hook - c.t0);
            let y = Y_RIG + (pv[1] - Y_RIG) * (1.0 - dmath::pow(1.0 - u, 3.0));
            let blk = [pv[0], y, pv[2]];
            let ring = add3(cc, [0.0, R, 0.0]);
            let dang = sub3(blk, [0.0, 0.42, 0.0]);
            let end = add3(dang, scale3(sub3(ring, dang), ss(0.55, 1.0, u)));
            return Some(Crane {
                top: [pv[0], Y_RIG, pv[2]],
                block: blk,
                end: Some(end),
                attached: false,
                pieces: Vec::new(),
            });
        }
        if tf < c.end {
            // hooked: hoisted back, the swing
            let ring = add3(cc, scale3(unit3(sub3(pv, cc)), R));
            return Some(Crane {
                top: [pv[0], Y_RIG, pv[2]],
                block: pv,
                end: Some(ring),
                attached: true,
                pieces: Vec::new(),
            });
        }
        // unhooked: the block flies back up
        let v = (tf - c.end) / 0.55;
        let blk = add3(pv, [0.0, (Y_RIG - pv[1]) * v * v, 0.0]);
        let end_c = self.cycle_end_ring(p, c);
        let dang = add3(
            sub3(blk, [0.0, 0.42, 0.0]),
            [0.18 * dmath::sin(v * 9.0) * (1.0 - v), 0.0, 0.0],
        );
        let end = add3(end_c, scale3(sub3(dang, end_c), ss(0.0, 0.3, v)));
        Some(Crane {
            top: [pv[0], Y_RIG, pv[2]],
            block: blk,
            end: Some(end),
            attached: false,
            pieces: Vec::new(),
        })
    }

    /// `draw_crane(arr, cam, cs)`: the crane line from the rig, the hook block, the swing cable and its hook, the
    /// snapped pieces.
    fn draw_crane(arr: &mut Img<f32>, cam: &Cam, cs: Option<&Crane>) {
        let Some(cs) = cs else {
            return;
        };
        let blk = cs.block;
        let line = cable(cs.top, add3(blk, [0.0, 0.17, 0.0]), 6, 0.0, 0.0, 0.0);
        line3(arr, cam, &line, STEEL.map(|v| v * 0.75), 2, 0.8, 0.3, 1.0);
        hook_block(arr, cam, blk);
        if let Some(end) = cs.end {
            let bot = sub3(blk, [0.0, 0.2, 0.0]);
            let slack = if cs.attached { 0.0 } else { 0.02 };
            line3(arr, cam, &cable(bot, end, 8, slack, 0.0, 0.0), STEEL, 2, 0.8, 0.5, 1.0);
            hook_shape(arr, cam, end, unit3(sub3(blk, end)));
        }
        for (q, a) in &cs.pieces {
            if *a > 0.02 {
                line3(arr, cam, q, STEEL, 2, 0.8, 0.5, *a);
            }
        }
    }

    /// `draw_swing_fx(arr, cam, t)`: the swing's motion smear (ghost balls a few ms back) and its coral trail, under
    /// the ball.
    fn draw_swing_fx(&self, p: &PictureCtx, arr: &mut Img<f32>, cam: &Cam, t: f64) {
        let pi = std::f64::consts::PI;
        let tf = py_min(t, p.t_ko - 1e-4);
        let Some(c) = self.cycle_at(tf, 0.0) else {
            return;
        };
        let Some(sw) = c.sw else {
            return;
        };
        if !(sw.tl0 <= tf && tf < sw.ts1 + 0.12) {
            return;
        }
        // v4: a long, fast yank (a turned swing) smears too
        let long_hoist = np::hypot(sw.ps[0] - sw.p0[0], sw.ps[1] - sw.p0[1]) > 1.2;
        let ok = |swinging: bool| swinging || long_hoist;
        let mut pts = Vec::new();
        for j in (0..=12).rev() {
            let tt = tf - j as f64 * 0.018;
            if let Some((q, h, swinging)) = Self::wreck_pose(Some(c), tt)
                && ok(swinging)
            {
                pts.push(cam.wp(&[w_at(q, R + h)])[0]);
            }
        }
        let uu = py_min(1.0, py_max(0.0, (tf - sw.ts0) / py_max(1e-6, sw.ts1 - sw.ts0)));
        let spd = if tf >= sw.ts0 { dmath::sin(pi * uu) } else { 0.8 };
        if tf < sw.ts0 && !long_hoist {
            return;
        }
        let col = p.col[self.k];
        if pts.len() >= 2 {
            swing_trail(arr, &pts, col, 0.35 + 0.65 * spd);
        }
        for j in [4, 3, 2, 1] {
            let Some((q, h, swinging)) = Self::wreck_pose(Some(c), tf - j as f64 * 0.014) else {
                continue;
            };
            if !ok(swinging) {
                continue;
            }
            let pc = cam.to_cam(&[w_at(q, R + h)])[0];
            let s = cam.proj(&[pc])[0];
            let rp = R * cam.f / pc[2];
            let a = linspace(0.0, 2.0 * pi, 19);
            let circ: Vec<(f64, f64)> = a[..18]
                .iter()
                .map(|&ai| (s.0 + rp * dmath::cos(ai), s.1 + rp * dmath::sin(ai)))
                .collect();
            let alpha = 0.24 * (1.0 - j as f64 / 5.0) * (0.3 + 0.7 * spd);
            ring::layer_blend(arr, &[circ], tinted(col, 0.7, 70.0), alpha, None);
        }
    }
}

/// `e[7]` of a hit (what hit), or `""`.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// `e[4]` of an ability event: its kind.
fn ab_kind(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(2)) {
        ("ability", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// `e[2]`: the fighter an event names.
///
/// # Panics
/// When the event names no fighter (Python then indexes `P.COL` with whatever is there).
fn who(e: &Event) -> usize {
    e.who()
        .filter(|&k| k < 2)
        .unwrap_or_else(|| panic!("the {} event at {} s names no fighter", e.kind, e.t))
}

impl Look for BariSaxLook {
    fn shards(&self) -> Vec<String> {
        // ['wood'] * 6 + ['key'] * 8 + ['bell'] * 3: #39's lacquer sticks, nickel key chips, bell chips
        [("wood", 6), ("key", 8), ("bell", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn flash_hits(&self) -> &[&str] {
        &["quake", "sling", "hook", "wreck", "crash"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["quake", "sling", "hook", "shockwave", "wreck", "crash"]
    }

    /// #39: speed lines at the ball's height.
    fn speed_lift(&self) -> bool {
        true
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `bari_sax.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities={:?}; weapon.base={:?}; weapon.kind={:?}; weapon.lift16={:?}",
            (self.w.snapper, self.holds),
            self.base,
            self.wrecker,
            self.lift16
        ))
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.nums = bold::Nums::of(cfg);
        let w = cfg.raw.get("weapon");
        let num = |key: &str| w.and_then(|w| w.get(key)).and_then(|v| v.as_f64());
        self.wrecker = w.and_then(|w| w.get("kind")).and_then(|v| v.as_str()) == Some("wrecking");
        self.lift16 = num("lift16").unwrap_or(0.0);
        self.base = num("base").unwrap_or(0.0);
        let ev = &p.rec.events;
        self.smashes = ev
            .iter()
            .filter(|e| e.kind == "smash" && e.who() == Some(k))
            .map(|e| (e.t, e.f(5)))
            .collect();
        self.e_hops = ev
            .iter()
            .filter(|e| e.kind == "hop" && e.who() == Some(1 - k))
            .map(|e| Hop {
                t: e.t,
                p0: [e.f(3), e.f(4)],
                p1: [e.f(5), e.f(6)],
                t_land: e.f(7),
            })
            .collect();
        self.w = Wreckage::new(&PicSrc(p), k, cfg);
        let beat = p.show.grid.beat;
        self.holds = [&cfg.unlock1, &cfg.unlock2, &cfg.low].map(|a| match a.kind.name() {
            "offbeat" => Some(match a.get("dur_beats") {
                Some(d) => d * beat,
                None => a.get("dur").unwrap_or(0.0),
            }),
            "snap" => a.get("tele16").map(|t| t * beat / 4.0),
            _ => None,
        });
    }

    // ------------------------------------------------------------ the ball: its height on the cable
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if p.bold {
            return self.bold_state(p, kk, st, t, phase);
        }
        if kk != self.k {
            return;
        }
        let lift = |st: &mut BallState, h: f64| {
            st.lift = h;
            if h != 0.0 {
                st.paint_h = Some(R);
            }
            if h > 0.02 {
                st.air_bias = 0.6;
            }
        };
        match phase {
            Phase::Fight => {
                if let Some((q, h)) = self.air_pose(p, py_min(t, p.t_ko - 1e-4)) {
                    st.p = q;
                    lift(st, h);
                }
            }
            Phase::Tail if kk == p.win => lift(st, self.h_ko(p) * (1.0 - ss(p.t_hs, p.t_hs + 0.4, t))),
            _ => {}
        }
    }

    fn ball_under(&self, p: &PictureCtx, kk: usize, _st: &BallState, t: f64, arr: &mut Img<f32>, cam: &Cam) {
        if kk != self.k || !self.wrecker || !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return;
        }
        if p.bold {
            return self.bold_ball_under(p, arr, cam, t);
        }
        self.draw_swing_fx(p, arr, cam, t);
    }

    fn ball_after_sprite(
        &self,
        p: &PictureCtx,
        kk: usize,
        st: &BallState,
        t: f64,
        arr: &mut Img<f32>,
        s: (f64, f64),
        rp: f64,
    ) {
        if kk != self.k || !self.wrecker || st.scale <= 0.5 {
            return;
        }
        // the harness ring's side while hooked (#39 sets st['ring_dir'] right before the ball)
        let mut ring_dir = None;
        if p.show.grid.t_fight <= t
            && t < p.t_hs
            && let Some(cs) = self.crane_state(p, t)
            && cs.attached
        {
            let cam = p.camera(t);
            let a = cam.wp(&[scene::to_world(st.p[0], st.p[1], R + st.lift)])[0];
            let b = cam.wp(&[cs.block])[0];
            ring_dir = Some(unit2((b.0 - a.0, b.1 - a.1)));
        }
        harness(
            arr,
            s,
            rp,
            st.sx.unwrap_or(1.0),
            st.sy.unwrap_or(1.0),
            ring_dir,
            p.col[self.k],
        );
    }

    fn ball_late(&self, _p: &PictureCtx, kk: usize, f: &FrameCtx<'_>, st: &BallState) -> bool {
        kk == self.k
            && self.wrecker
            && f.in_fight
            && st.vis
            && st.lift > 0.02
            && py_max(st.p[0], st.p[1]) > scene::A - R * 1.05
    }

    fn ability_hold(&self, _p: &PictureCtx, slot: &str) -> Option<f64> {
        let s = ["unlock1", "unlock2", "low"].iter().position(|x| *x == slot)?;
        self.holds[s]
    }

    // ------------------------------------------------------------ the ropes
    fn rope_press(&self, p: &PictureCtx, _s: usize, _y: f64, t: f64, pts: &[[f64; 2]], yy: &mut [f64]) {
        if let Some((c, hb)) = self.bari_over(p, t) {
            for (q, h) in pts.iter().zip(yy.iter_mut()) {
                let dd = np::hypot(q[0] - c[0], q[1] - c[1]);
                if dd < R {
                    // the ball's underside above the point
                    let under = hb + R - np::maximum(R * R - dd * dd, 0.0).sqrt() - 0.03;
                    *h = np::minimum(*h, under);
                } else if dd < R * 1.7 {
                    // the rope eases back up either side of it
                    let w = 1.0 - (dd - R) / (R * 0.7);
                    *h = np::minimum(*h, *h - np::maximum(*h - (hb - 0.03), 0.0) * w * w * 0.6);
                }
            }
        }
        for h in yy.iter_mut() {
            *h = np::maximum(*h, 0.05);
        }
    }

    fn rope_events(&self, p: &PictureCtx) -> Vec<RopeEv> {
        let mut out = Vec::new();
        let mut prev: [Option<[f64; 2]>; 4] = [None; 4];
        let (f0, f1) = ((p.show.grid.t_fight * 30.0) as i64, (p.t_ko * 30.0) as i64);
        for f in f0..=f1 {
            let t = f as f64 / 30.0;
            let mut cur: [Option<[f64; 2]>; 4] = [None; 4];
            if let Some((c, hb)) = self.bari_over(p, t) {
                for (s, &(ax, sg)) in SIDES.iter().enumerate() {
                    if c[ax] * sg > scene::A - R * 1.05 && hb < scene::ROPE_Y[2] {
                        cur[s] = Some(c);
                    }
                }
            }
            for s in 0..4 {
                if let (Some(c), None) = (prev[s], cur[s]) {
                    out.push(RopeEv {
                        t,
                        side: s,
                        x: c[0],
                        z: c[1],
                        spd: 3.2,
                    });
                }
            }
            prev = cur;
        }
        out
    }

    // ------------------------------------------------------------ events -> camera, ropes, sparks, glows
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.t > p.t_ko {
            return None;
        }
        match e.kind.as_str() {
            "hit" => match how(e) {
                "shockwave" => Some(1.6),
                "quake" => Some(2.2),
                "sling" => Some(1.4),
                _ => None,
            },
            // a WRECKING BALL smash (harder the faster)
            "smash" if e.who() == Some(self.k) => Some(1.0 + 1.2 * e.f(5)),
            // CABLE SNAP: the crash (its echo: shake_more)
            "crater" if e.who() == Some(self.k) => Some(2.5),
            _ => None,
        }
    }

    fn shake_more(&self, p: &PictureCtx, e: &Event) -> Vec<(f64, f64)> {
        if e.kind == "crater" && e.who() == Some(self.k) && e.t <= p.t_ko {
            return vec![(e.t + 0.09, 1.2)];
        }
        Vec::new()
    }

    fn rope(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match ab_kind(e) {
            "quake" => Some(1.0),
            "shockwave" => Some(0.5),
            _ => None,
        }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[who(e)].map(|c| c * 0.6 + 100.0);
        let (n, sp, life) = match how(e) {
            "quake" | "sling" | "shockwave" | "hook" | "wreck" | "crash" => (16, 3.2, 0.3),
            _ => (8, 2.4, 0.22),
        };
        (col, n, sp, life)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "quake" | "sling" | "shockwave" | "wreck" | "crash" => Some((14.0, 0.35)),
            "rattle" => Some((8.0, 0.2)),
            _ => None,
        }
    }

    // ------------------------------------------------------------ effects (reelkit.fx), in #39's order
    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let k = self.k;
        let t = e.t;
        let emit = |n: i64, col: Vec<V3>| Emit {
            n: Some(n),
            col: Some(col),
            ..Emit::default()
        };
        match e.kind.as_str() {
            "hit" => {
                let att = who(e);
                let (x, z) = (e.f(5), e.f(6));
                let pos = scene::to_world(x, z, R);
                match how(e) {
                    "shockwave" => {
                        // GROWL lands: grit
                        fx.bu.add(t, pos, p.col[att], 1.2, None, true);
                        fx.fl.add(t, pos, p.fl_col[att], 1.8, 0.8, 0.03, 0.35, 0.0);
                        fx.pa.emit(t, pos, "chips", &emit(12, GRIT.to_vec()));
                    }
                    "wreck" => {
                        // WRECKING BALL smash: burst, dust, a skid mark
                        let sp = self
                            .smashes
                            .iter()
                            .find(|m| (m.0 - t).abs() < 1e-4)
                            .map_or(0.7, |m| m.1);
                        fx.bu.add(t, pos, p.col[att], 1.0 + 0.7 * sp, None, true);
                        fx.fl.add(t, pos, p.fl_col[att], 2.0, 0.6 + 0.5 * sp, 0.03, 0.32, 0.0);
                        let dust = Emit {
                            n: Some(6 + np::f64_to_i64(8.0 * sp)),
                            col: Some(MAPLE_DUST.to_vec()),
                            scale: 1.1,
                            ..Emit::default()
                        };
                        fx.pa.emit(t, scene::to_world(x, z, 0.06), "dust", &dust);
                        fx.pa.emit(t, pos, "sparks", &emit(10, CORAL_SP.to_vec()));
                        let mut chips = GRIT.to_vec();
                        chips.push(MAPLE_DUST[0]);
                        fx.pa.emit(t, pos, "chips", &emit(5, chips));
                        fx.mk.add(
                            t,
                            Mark {
                                size: 0.32 + 0.2 * sp,
                                col: [46.0, 30.0, 30.0],
                                life: 3.0,
                                seed: seed_of(t * 100.0),
                                strength: 0.45,
                                ..Mark::new(MarkKind::Scorch, x, z)
                            },
                        );
                    }
                    "crash" => {
                        // CABLE SNAP lands on its target
                        fx.bu.add(t, pos, p.col[att], 1.8, None, true);
                        fx.fl.add(t, pos, p.fl_col[att], 2.6, 1.1, 0.03, 0.45, 0.0);
                    }
                    _ => {}
                }
            }
            "swing" if e.who() == Some(k) => {
                // the swing's bottom: a coral light
                let tb = (e.f(11) + e.f(12)) / 2.0;
                if tb <= p.t_ko {
                    fx.fl.add(
                        tb,
                        scene::to_world(e.f(7), e.f(8), R),
                        p.fl_col[k],
                        2.0,
                        0.45,
                        0.03,
                        0.25,
                        0.0,
                    );
                }
            }
            "cable_snap" if e.who() == Some(k) => {
                // the cable parts: steel sparks up high
                let i = p.sim_i(t);
                let at = p.run.p(i, k);
                let h = PicSrc(p).row("SNAP", i, k).map_or(0.0, |r| r[1]);
                let top = scene::to_world(at[0], at[1], R + h + 2.0 * R + 0.5);
                fx.bu.add(t, top, [230.0, 234.0, 245.0], 0.9, None, false);
                let sparks = Emit {
                    n: Some(14),
                    spread: Some(180.0),
                    col: Some(vec![
                        [255.0, 250.0, 230.0],
                        [226.0, 230.0, 242.0],
                        [255.0, 196.0, 120.0],
                    ]),
                    ..Emit::default()
                };
                fx.pa.emit(t, top, "sparks", &sparks);
                fx.fl.add(t, top, [255.0, 240.0, 220.0], 1.8, 0.6, 0.03, 0.25, 0.0);
            }
            "crater" if e.who() == Some(k) => {
                // the crash: a ring of dust, grit, the crater mark
                let (x, z) = (e.f(3), e.f(4));
                let dust = Emit {
                    n: Some(26),
                    spread: Some(180.0),
                    col: Some(MAPLE_DUST.to_vec()),
                    scale: 1.4,
                    speed: Some((1.2, 3.0)),
                    ..Emit::default()
                };
                fx.pa.emit(t, scene::to_world(x, z, 0.05), "dust", &dust);
                let chips = Emit {
                    n: Some(14),
                    spread: Some(180.0),
                    col: Some(GRIT.to_vec()),
                    speed: Some((1.5, 3.4)),
                    ..Emit::default()
                };
                fx.pa.emit(t, scene::to_world(x, z, R * 0.5), "chips", &chips);
                fx.mk.add(
                    t,
                    Mark {
                        size: 1.5,
                        life: 6.0,
                        n: 7,
                        seed: seed_of(t * 10.0),
                        strength: 0.75,
                        ..Mark::new(MarkKind::Crack, x, z)
                    },
                );
                fx.mk.add(
                    t,
                    Mark {
                        size: 0.95,
                        col: [34.0, 22.0, 24.0],
                        life: 6.0,
                        seed: seed_of(t * 10.0) + 1,
                        strength: 0.7,
                        ..Mark::new(MarkKind::Scorch, x, z)
                    },
                );
            }
            "ability" if ab_kind(e) == "shockwave" => {
                // GROWL leaves the bari: grit
                let at = p.run.p(p.sim_i(t), who(e));
                let chips = Emit {
                    n: Some(14),
                    spread: Some(180.0),
                    col: Some(GRIT.to_vec()),
                    speed: Some((1.5, 3.0)),
                    ..Emit::default()
                };
                fx.pa.emit(t, scene::to_world(at[0], at[1], R * 0.6), "chips", &chips);
            }
            _ => {}
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let emit = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(vec![WOOD, [214.0, 216.0, 226.0], [232.0, 208.0, 150.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "brass", &emit);
    }

    // ------------------------------------------------------------ the canvas
    /// GROWL's gravelly, broken ring.
    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if p.bold {
            return self.bold_canvas_wave(ic, c, w);
        }
        let bayer = config::bayer_full();
        let gain = if w.kind == "quake" { 1.3 } else { 1.0 };
        let tint = p.col[self.k].map(|v| v / 255.0 * 1.3 + 0.2);
        for (j, light) in ic.iter_mut().enumerate() {
            let band = growl_band(c.sxp[j], c.szp[j], w.cx, w.cz, w.r, w.rmax, &w.kind, c.t);
            let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
            let v = band * gain;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    /// CABLE SNAP's target: the falling ball's shadow and a pulsing danger ring.
    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if p.bold {
            return self.bold_canvas_after(p, ic, c);
        }
        let t = c.t;
        let tcn = py_min(t, p.t_ko);
        let bayer = config::bayer_full();
        let tint = p.col[self.k].map(|v| v / 255.0 * 1.3 + 0.2);
        for q in &self.w.snaps {
            if !(q.t0 <= tcn && tcn < q.t1 && t < p.t_hs) {
                continue;
            }
            let u = (tcn - q.t0) / py_max(1e-6, q.t1 - q.t0);
            for (j, light) in ic.iter_mut().enumerate() {
                let (shd, rng) = snap_target(c.sxp[j], c.szp[j], q.spot[0], q.spot[1], q.rad, u, t);
                let dim = 1.0 - 0.8 * shd;
                let rng = (rng * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
                for v in light.iter_mut() {
                    *v *= dim;
                }
                for (v, &c) in light.iter_mut().zip(&tint) {
                    *v += 1.1 * rng * c;
                }
            }
        }
    }

    // ------------------------------------------------------------ the painter pass, over the ring
    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if self.wrecker && self.crane_first(p, f.t) {
            let item: Payload = Box::new(CraneItem);
            return vec![(f64::INFINITY, item)];
        }
        Vec::new()
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if p.bold {
            return self.bold_draw_item(p, f, fr, payload);
        }
        if payload.downcast_ref::<CraneItem>().is_none() {
            return;
        }
        let (mut arr, depth) = (fr.array(), fr.depth());
        Self::draw_crane(&mut arr, f.cam, self.crane_state(p, f.t).as_ref());
        fr.set_array(&arr, Some(&depth));
    }

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        if p.bold {
            return self.bold_over_ring(p, f, arr);
        }
        if self.wrecker && !self.crane_first(p, f.t) {
            Self::draw_crane(arr, f.cam, self.crane_state(p, f.t).as_ref());
        }
        if f.in_fight {
            for w in p.run.waves(f.i) {
                if w.owner == self.k {
                    draw_honk_front(
                        arr,
                        f.cam,
                        w.cx,
                        w.cz,
                        w.r,
                        w.rmax,
                        p.col[self.k],
                        w.kind != "shockwave",
                    );
                }
            }
        }
    }

    /// #39's: a stick of the lacquered body, a nickel key chip, a bell chip.
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
        let to = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            cam.wp(&world)
        };
        if kind == "wood" {
            let (ln, wd) = (0.26, 0.035);
            let q4: Vec<[f64; 2]> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(a1, a2)| {
                    [
                        sx + d[0] * ln * a1 + nn[0] * wd * a2,
                        sz + d[1] * ln * a1 + nn[1] * wd * a2,
                    ]
                })
                .collect();
            ring::layer_blend(arr, &[to(&q4)], WOOD, 1.0, None);
            return;
        }
        let key = kind == "key";
        let col = if key { NICKEL } else { WOOD.map(|c| c * 1.3) };
        let s = size * if key { 0.7 } else { 1.5 };
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
        ring::layer_blend(
            arr,
            &[to(&tri)],
            col.map(|c| np::clip(c * shade_k, 0.0, 255.0)),
            1.0,
            None,
        );
    }

    // ------------------------------------------------------------ lens
    /// Heat shimmer on GROWL's ring.
    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        let big = w.kind == "quake";
        img.shimmer_world(
            cam,
            scene::to_world(w.cx, w.cz, 0.05),
            w.r,
            if big { 0.45 } else { 0.3 },
            if big { 3.0 } else { 2.2 },
            t,
            1.0 - w.r / w.rmax,
            w.owner as f64,
        );
    }

    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[bf_picture::look::DrawnBall]) {
        if p.bold {
            self.bold_after_grade(p, img, f, drawn);
        }
    }

    // ------------------------------------------------------------ checks
    fn focus_points(&self, _p: &PictureCtx, _t: f64, st: &BallState) -> Vec<(V3, String)> {
        vec![
            (scene::to_world(st.p[0], st.p[1], 0.0), "bari sax".to_string()),
            (
                scene::to_world(st.p[0], st.p[1], 2.0 * R + st.lift),
                "bari sax".to_string(),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cable_runs_straight_or_sags() {
        let p = cable([0.0, 0.0, 0.0], [1.0, 2.0, 0.0], 3, 0.0, 0.0, 0.0);
        assert_eq!(p, vec![[0.0, 0.0, 0.0], [0.5, 1.0, 0.0], [1.0, 2.0, 0.0]]);
        let s = cable([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 3, 0.1, 0.0, 0.0);
        assert!((s[1][1] + 0.1).abs() < 1e-15);
    }

    #[test]
    fn growls_ring_is_broken() {
        let a = growl_band(1.0, 0.0, 0.0, 0.0, 1.0, 2.0, "shockwave", 0.0);
        assert!(a > 0.0 && a <= 0.5);
        assert_eq!(growl_band(3.0, 0.0, 0.0, 0.0, 1.0, 2.0, "shockwave", 0.0), 0.0);
    }

    #[test]
    fn the_snap_target_darkens_as_it_falls() {
        let (a, _) = snap_target(0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0);
        let (b, _) = snap_target(0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 0.0);
        assert!(b > a);
    }
}
