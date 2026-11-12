//! BASSOON's look: the port of `Look` in `projects/ballfight/kit/fighters/bassoon.py` and of the module's draw code
//! (#39's `styles.py`), hook by hook.
//!
//! What it draws: the ball crouching, springing up in a tall arc (stretched as it rises and falls) and landing flat
//! (a squash, then a little bounce back), a stomp ring on the canvas with maple dust, a print and a green flare, a
//! faint dotted arc of each hop; a backstab's crossed green slashes, bigger double ring and claw mark; BOCAL HOOK's
//! silver crook over both balls and the hooked enemy whipping round (a swirl after the grade); MOUNTAIN KING's
//! circle of landing spots round the enemy and a green light that keeps up the dance. It bobs on its toes in the
//! corner; as champion it hops a little on every solo note. It shatters into maple sticks, nickel keys and cane
//! blades.
//!
//! Its VS card panel is the card's own (`_bassoon`, `bf_picture::title::patterns::bassoon`).
//!
//! The masks and blends are the module's own copies of the reeds' (`_mask`, `blend`, `add`:
//! `fighters::shared::reeds`); its `layer_blend` is its own arithmetic (`arr * (1 - mk) + col * mk`) and is here.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{self, H, W, ease, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::BallState;
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::{np, pil};

use crate::fighters::shared::reeds::{self, Num};

/// `INK`.
const INK: V3 = [18.0, 10.0, 26.0];
/// `WHITE`.
const WHITE: V3 = [255.0, 252.0, 240.0];
/// `NICKEL`: the crook.
const NICKEL: V3 = [216.0, 220.0, 232.0];
/// `THREAD`: the reed's thread wrap, a nub at the crook's root.
const THREAD: V3 = [200.0, 46.0, 50.0];
/// `WOOD`: #39's shatter maple.
const WOOD: V3 = [120.0, 56.0, 52.0];
/// `NICKEL_SH`: a key chip.
const NICKEL_SH: V3 = [214.0, 216.0, 226.0];
/// `CANE_SH`: a cane blade.
const CANE_SH: V3 = [232.0, 208.0, 150.0];
/// `MAPLE_DUST`.
const MAPLE_DUST: [V3; 3] = [[214.0, 176.0, 140.0], [186.0, 140.0, 110.0], [240.0, 214.0, 186.0]];
/// `GREEN_SP`.
const GREEN_SP: [V3; 3] = [[150.0, 240.0, 180.0], [60.0, 190.0, 120.0], [240.0, 255.0, 240.0]];

/// `np.array([math.cos(th), math.sin(th)])`.
fn unit(th: f64) -> [f64; 2] {
    [dmath::cos(th), dmath::sin(th)]
}

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

/// `e[2]` of a hit: the attacker.
///
/// # Panics
/// When the event names no fighter (Python then indexes `P.COL` with whatever is there).
fn attacker(e: &Event) -> usize {
    e.who()
        .filter(|&k| k < 2)
        .unwrap_or_else(|| panic!("the {} event at {} s names no fighter", e.kind, e.t))
}

/// `e[8]` of a hit, its sub-number (a stomp's: 1 for a backstab); 0 for an event without one.
fn sub(e: &Event) -> i64 {
    match e.fields.get(6) {
        Some(Value::Int(n)) => *n,
        _ => 0,
    }
}

/// `int(round(x))` of a screen coordinate: half to even.
fn round_px(x: f64) -> i64 {
    np::f64_to_i64(py::round_half_even(x))
}

/// `int(x)` of a time that is never negative, as a seed.
fn seed_of(x: f64) -> u64 {
    np::f64_to_i64(x).max(0) as u64
}

/// `np.clip(col * a + b, 0, 255)`.
fn tinted(col: V3, a: f64, b: f64) -> V3 {
    col.map(|c| np::clip(c * a + b, 0.0, 255.0))
}

/// The rows or columns `lo:hi` of an axis `len` long, as numpy slices them (`lo` is never negative here; a negative
/// `hi` counts from the end).
fn py_slice(lo: i64, hi: i64, len: usize) -> std::ops::Range<usize> {
    let len = len as i64;
    let hi = if hi < 0 { (hi + len).max(0) } else { hi.min(len) };
    let lo = lo.clamp(0, len);
    lo as usize..(hi.max(lo)) as usize
}

// ---------------------------------------------------------------- #39 styles.py
/// `hop_height(L)`: the peak height of a hop of length `l` (ring units): a tall, exaggerated tiptoe even for short
/// hops.
fn hop_height(l: f64) -> f64 {
    0.42 + 0.16 * py_min(2.5, l)
}

/// `hop_state(u, L)`: `(height, squash-x, squash-y)` of a hopping ball at hop progress `u` (0 to 1): it crouches as
/// it leaves, stretches tall while it rises and falls fast, round at the top.
fn hop_state(u: f64, l: f64) -> (f64, f64, f64) {
    let hgt = hop_height(l) * 4.0 * u * (1.0 - u);
    // vertical speed (0 at the top)
    let v = (1.0 - 2.0 * u).abs();
    let sy = 1.0 + 0.16 * v;
    (hgt, 1.0 / sy.sqrt(), sy)
}

/// `land_squash(age)`: the squash on landing: flat and wide for a couple of frames, then a little bounce back.
fn land_squash(age: f64) -> (f64, f64) {
    if !(0.0..=0.2).contains(&age) {
        return (1.0, 1.0);
    }
    if age < 0.07 {
        let k = 1.0 - age / 0.07 * 0.3;
        return (1.0 + 0.24 * k, 1.0 - 0.26 * k);
    }
    let u = (age - 0.07) / 0.13;
    let s = 0.06 * dmath::sin(dmath::PI * u) * (1.0 - u);
    (1.0 - s, 1.0 + s)
}

/// `stomp_light(sxp, szp, x, z, age, radius, back)` for one canvas pixel at distance `rd` from the stomp, with
/// `u = age / life`: a quick expanding band (a backstab's is bigger and double).
fn stomp_band(rd: f64, u: f64, radius: f64, back: bool) -> f64 {
    let b = if back { 1.0 } else { 0.0 };
    // r = 0.35 + (radius + 0.5 + 0.5 * back) * (1 - (1 - u) ** 2)
    let r = 0.35 + (radius + 0.5 + 0.5 * b) * (1.0 - (1.0 - u) * (1.0 - u));
    let mut band = np::clip(1.0 - (rd - r).abs() / (0.07 + 0.05 * b), 0.0, 1.0) * (1.0 - u);
    if back {
        let inner = np::clip(1.0 - (rd - r * 0.62).abs() / 0.05, 0.0, 1.0) * (1.0 - u) * 0.8;
        band = np::maximum(band, inner);
    }
    band
}

/// The frame's pixel `(xi, yi)` times `1 - k` (a Python float beside a `float32` picture: the product stays
/// `float32`) plus `c * k` (`float64`), stored back as `float32`: `img[yi, xi] = img[yi, xi] * (1 - k) + c * k`.
fn mix_px(arr: &mut Img<f32>, xi: usize, yi: usize, c: V3, k: f64) {
    let keep = (1.0 - k) as f32;
    for (v, &cc) in arr.px_mut(xi, yi).iter_mut().zip(&c) {
        *v = (f64::from(*v * keep) + cc * k) as f32;
    }
}

/// `draw_hop_arc(img, cam, p0, p1, age_k, col, y0, peak)`: a faint dotted arc of the hop's path, fading out.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_hop_arc(arr: &mut Img<f32>, cam: &Cam, p0: [f64; 2], p1: [f64; 2], age_k: f64, col: V3, y0: f64, peak: f64) {
    if age_k <= 0.02 {
        return;
    }
    let world: Vec<V3> = linspace(0.0, 1.0, 9)
        .iter()
        .map(|&u| {
            // P = p0 + (p1 - p0) * u;  hy = y0 + peak * 4 * u * (1 - u)
            scene::to_world(
                p0[0] + (p1[0] - p0[0]) * u,
                p0[1] + (p1[1] - p0[1]) * u,
                y0 + peak * 4.0 * u * (1.0 - u),
            )
        })
        .collect();
    let sp = cam.wp(&world);
    let c = tinted(col, 0.7, 90.0);
    for &(x, yy) in &sp[1..sp.len() - 1] {
        let (xi, yi) = (round_px(x), round_px(yy));
        if 1 <= xi && xi < W as i64 - 1 && 1 <= yi && yi < H as i64 - 1 {
            mix_px(arr, xi as usize, yi as usize, c, age_k);
        }
    }
}

/// `backstab_slash(img, cam, x, z, aim, age, col, y, life=0.28)`: two crossed slashes in the bassoon's green, inked,
/// flicking across the enemy's back.
#[allow(clippy::too_many_arguments)] // the Python signature
fn backstab_slash(arr: &mut Img<f32>, cam: &Cam, x: f64, z: f64, aim: f64, age: f64, col: V3, y: f64) {
    let life = 0.28;
    if age < 0.0 || age >= life {
        return;
    }
    let k = 1.0 - age / life;
    let d = unit(aim);
    let nrm = [-d[1], d[0]];
    let grow = py_min(1.0, age / 0.06);
    for sgn in [1.0, -1.0] {
        // a = [x, z] - d * 0.22 + nrm * 0.62 * sgn;  b = [x, z] + d * 0.22 - nrm * 0.62 * sgn;  b = a + (b - a) * grow
        let a = [
            x - d[0] * 0.22 + nrm[0] * 0.62 * sgn,
            z - d[1] * 0.22 + nrm[1] * 0.62 * sgn,
        ];
        let b = [
            x + d[0] * 0.22 - nrm[0] * 0.62 * sgn,
            z + d[1] * 0.22 - nrm[1] * 0.62 * sgn,
        ];
        let b = [a[0] + (b[0] - a[0]) * grow, a[1] + (b[1] - a[1]) * grow];
        let sp = cam.wp(&[
            scene::to_world(a[0], a[1], y + 0.3),
            scene::to_world(b[0], b[1], y - 0.1),
        ]);
        let lines = [sp];
        reeds::blend(arr, &reeds::mask(&[], &lines, 5), INK, Num::Py(0.75 * k));
        reeds::blend(
            arr,
            &reeds::mask(&[], &lines, 3),
            tinted(col, 0.7, 90.0),
            Num::Py(1.0 * k),
        );
        reeds::blend(arr, &reeds::mask(&[], &lines, 1), WHITE, Num::Py(0.9 * k));
    }
}

/// `crook_points(p, aim, ext, R_, curl, y)`: the bocal as a hook: out of the ball's rim along the aim for `ext`
/// (ring units), then curling round by `curl` (rad) at the end; ring-frame points `[x, z, height]`.
fn crook_points(p: [f64; 2], aim: f64, ext: f64, r: f64, curl: f64, y: f64) -> Vec<V3> {
    let d = unit(aim);
    let nrm = [-d[1], d[0]];
    // base_ = p + d * R_ * 0.9
    let base = [p[0] + d[0] * r * 0.9, p[1] + d[1] * r * 0.9];
    let s = linspace(0.0, 1.0, 10);
    let mut pts: Vec<V3> = s
        .iter()
        .map(|&s| {
            let arch = dmath::sin(dmath::PI * s);
            // shaft = base_ + d * (ext * s) + nrm * (0.18 * np.sin(np.pi * s) * ext / 1.6)
            let side = 0.18 * arch * ext / 1.6;
            // hy = y + 0.10 + 0.25 * np.sin(np.pi * s) * min(1.0, ext / 1.2)
            [
                base[0] + d[0] * (ext * s) + nrm[0] * side,
                base[1] + d[1] * (ext * s) + nrm[1] * side,
                y + 0.10 + 0.25 * arch * py_min(1.0, ext / 1.2),
            ]
        })
        .collect();
    if curl > 0.05 {
        // the hook at the end, curling back round
        let rc = 0.22;
        let end = pts[pts.len() - 1];
        let c = [end[0] + nrm[0] * rc, end[1] + nrm[1] * rc];
        let a0 = dmath::atan2(-nrm[1], -nrm[0]);
        for &da in &linspace(0.0, curl, 8)[1..] {
            let aa = a0 + da;
            pts.push([c[0] + rc * dmath::cos(aa), c[1] + rc * dmath::sin(aa), end[2]]);
        }
    }
    pts
}

/// `draw_crook(img, cam, p, aim, ext, R_, curl, y, col, glow)`: BOCAL HOOK: a silver crook swinging out of the ball
/// (inked, a thread-red nub at the root, lit green while it hooks).
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_crook(arr: &mut Img<f32>, cam: &Cam, p: [f64; 2], aim: f64, ext: f64, curl: f64, y: f64, col: V3, glow: f64) {
    let pts = crook_points(p, aim, ext, R, curl, y);
    let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], q[2])).collect();
    let sp = cam.wp(&world);
    let whole = [sp.clone()];
    reeds::blend(arr, &reeds::mask(&[], &whole, 7), INK, Num::Py(0.85));
    let silver: V3 = std::array::from_fn(|c| NICKEL[c] * 0.82 + col[c] * 0.18);
    reeds::blend(arr, &reeds::mask(&[], &whole, 4), silver, Num::Py(1.0));
    // sp[: max(2, len(sp) * 2 // 3)]
    let lit = [sp[..(sp.len() * 2 / 3).max(2).min(sp.len())].to_vec()];
    reeds::blend(arr, &reeds::mask(&[], &lit, 1), WHITE, Num::Py(0.9));
    if glow > 0.02 {
        reeds::add(
            arr,
            &reeds::mask(&[], &whole, 7),
            col.map(|c| c * 0.7 + 60.0),
            0.35 * glow,
        );
    }
    // img[max(0, yi - 1):yi + 2, max(0, xi - 1):xi + 2] = THREAD
    let (xi, yi) = (round_px(sp[0].0), round_px(sp[0].1));
    let thread = THREAD.map(|c| c as f32);
    for yy in py_slice((yi - 1).max(0), yi + 2, arr.h) {
        for xx in py_slice((xi - 1).max(0), xi + 2, arr.w) {
            arr.px_mut(xx, yy).copy_from_slice(&thread);
        }
    }
}

/// `swirl(img, x, y, rp, k, t, col)`: a hooked enemy spun round: two curved speed arcs whipping round the ball.
fn swirl(img: &mut Pic, x: f64, y: f64, rp: f64, k: f64, t: f64, col: V3) {
    if k <= 0.02 {
        return;
    }
    let c = tinted(col, 0.6, 100.0);
    for j in 0..2 {
        let a0 = t * 18.0 + f64::from(j) * dmath::PI;
        let pts: Vec<(f64, f64)> = linspace(0.0, 1.6, 9)
            .iter()
            .map(|&da| {
                let aa = a0 + da;
                (x + rp * 1.25 * dmath::cos(aa), y + rp * 1.25 * dmath::sin(aa))
            })
            .collect();
        let lines = [pts];
        reeds::blend_pic(img, &reeds::mask(&[], &lines, 3), INK, Num::Py(0.5 * k));
        reeds::blend_pic(img, &reeds::mask(&[], &lines, 1), c, Num::Py(1.0 * k));
    }
}

/// `mk_trail(cam, centre, ang0, step, n_done, R_, gap, y=0.06)`: MOUNTAIN KING's dance: the circle of landing spots
/// round the enemy (the ones so far), as screen points.
fn mk_trail(cam: &Cam, centre: [f64; 2], ang0: f64, step: f64, n_done: usize, gap: f64) -> Vec<(f64, f64)> {
    let rr = 2.0 * R + gap;
    let world: Vec<V3> = (0..n_done)
        .map(|j| {
            // a = ang0 + math.radians(step) * np.arange(n_done)
            let a = ang0 + step * (dmath::PI / 180.0) * j as f64;
            scene::to_world(centre[0] + rr * dmath::cos(a), centre[1] + rr * dmath::sin(a), 0.06)
        })
        .collect();
    cam.wp(&world)
}

/// The module's own `layer_blend(arr, polys, col, alpha)`: every polygon filled into an `L` mask, then
/// `arr[:] = arr * (1 - mk) + col * mk` with `mk = float32(mask) / 255.0 * alpha`: the picture's share is a
/// `float32` product, the colour's a `float64` one, the sum stored as `float32`.
fn layer_blend(arr: &mut Img<f32>, polys: &[Vec<(f64, f64)>], col: V3, alpha: f64) {
    let mut m: Img<u8> = Img::new(W, H, 1);
    for pts in polys {
        pil::polygon(&mut m, pts, &[255]);
    }
    let alpha = alpha as f32;
    for (i, &b) in m.data.iter().enumerate() {
        if b == 0 {
            continue; // arr * 1 + col * 0 is arr
        }
        let mk = f32::from(b) / 255.0f32 * alpha;
        for (v, &c) in arr.data[i * 3..i * 3 + 3].iter_mut().zip(&col) {
            *v = (f64::from(*v * (1.0f32 - mk)) + c * f64::from(mk)) as f32;
        }
    }
}

// ---------------------------------------------------------------- the look
/// A hop, from its `hop` event: `(t, k, p0, p1, t_land, kind)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hop {
    t: f64,
    p0: [f64; 2],
    p1: [f64; 2],
    t_land: f64,
    kind: i64,
}

/// A stomp, from its `stomp` event: `(t, k, x, z, hit, back, radius, kind)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stomp {
    t: f64,
    x: f64,
    z: f64,
    back: bool,
    rad: f64,
    kind: i64,
}

/// The painter item of this look: `'crook'`.
struct Crook;

/// BASSOON's look: `Look` of `kit/fighters/bassoon.py`.
#[derive(Clone, Debug, Default)]
pub struct BassoonLook {
    /// Its side.
    k: usize,
    /// `HOPS`, in time order (`HOP_T` is their times).
    hops: Vec<Hop>,
    /// `STOMPS`, in time order (`STOMP_T` is their times).
    stomps: Vec<Stomp>,
    /// `HOOK_OPEN`: `(t, t_hook, aim)`.
    hook_open: Vec<(f64, f64, f64)>,
    /// `HOOKS`: `(t, hit, aim)`.
    hooks: Vec<(f64, bool, f64)>,
    /// `MKS`: `(t0, t1)` of each MOUNTAIN KING.
    mks: Vec<(f64, f64)>,
    /// `cfg['abilities']['unlock2']['reach']`: how far the crook swings out (`None`: an entry without one, where
    /// Python raises when a crook is drawn).
    hook_reach: Option<f64>,
    /// `cfg['abilities']['low']['step']`: degrees between MOUNTAIN KING's landing spots.
    mk_step: Option<f64>,
    /// `cfg['abilities']['low']['gap']`: the landing spots' distance beyond contact.
    mk_gap: Option<f64>,
}

impl BassoonLook {
    /// `hop_at(t)`: the hop in the air at `t` and its progress, 0 to 1.
    fn hop_at(&self, p: &PictureCtx, t: f64) -> Option<(&Hop, f64)> {
        if self.hops.is_empty() || t >= p.t_ko || t < p.show.grid.t_fight {
            return None;
        }
        // j = int(np.searchsorted(ts, t, 'right')) - 1
        let j = self.hops.partition_point(|h| h.t <= t).checked_sub(1)?;
        let h = &self.hops[j];
        if h.t <= t && t < h.t_land {
            return Some((h, (t - h.t) / py_max(1e-6, h.t_land - h.t)));
        }
        None
    }

    /// `last_stomp(t)`: seconds since its last landing (99 before the first).
    fn last_stomp(&self, t: f64) -> f64 {
        match self.stomps.partition_point(|s| s.t <= t).checked_sub(1) {
            Some(j) => t - self.stomps[j].t,
            None => 99.0,
        }
    }

    /// `hooked_k(t)`: how hard the enemy is whipping round after being hooked (0 to 1).
    fn hooked_k(&self, p: &PictureCtx, t: f64) -> f64 {
        for &(th, hit, _) in &self.hooks {
            if hit && th <= t && t < th + 0.4 && th < p.t_ko {
                return 1.0 - (t - th) / 0.4;
            }
        }
        0.0
    }

    /// `crook_state(t)`: `(aim, ext, curl, glow)` of BOCAL HOOK's crook at `t`, or `None`.
    fn crook_state(&self, p: &PictureCtx, t: f64) -> Option<(f64, f64, f64, f64)> {
        if !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return None;
        }
        for &(t0, t_hook, aim) in &self.hook_open {
            if !(t0 <= t && t < t_hook + 0.42) {
                continue;
            }
            let reach = self
                .hook_reach
                .expect("BOCAL HOOK's crook: abilities.unlock2 has no reach (Python raises KeyError)");
            let i = p.sim_i(py_min(t, p.t_ko));
            // row = self.HOOK_REC[i, k];  a = float(row[1]) if row is not None and not np.isnan(row[1]) else aim
            let mut a = match p.run.row("HOOK", i, self.k) {
                Some(row) if row.len() > 1 && !row[1].is_nan() => row[1],
                _ => aim,
            };
            // res = [q for q in self.HOOKS if abs(q[0] - t_hook) < 0.02]
            let res = self.hooks.iter().find(|q| (q.0 - t_hook).abs() < 0.02);
            let hit = res.is_some_and(|q| q.1);
            if let Some(q) = res
                && t >= t_hook
            {
                a = q.2;
            }
            let full = R * 0.4 + reach + R * 0.6;
            if t < t_hook {
                let u = ease((t - t0) / py_max(1e-6, t_hook - t0));
                return Some((a, 0.35 + (full - 0.35) * u, 0.0, 0.3 * u));
            }
            let v = (t - t_hook) / 0.42;
            let curl = if hit { 2.4 } else { 0.6 } * py_min(1.0, v * 4.0);
            return Some((
                a,
                full * (1.0 - ease(py_max(0.0, v - 0.45) / 0.55)) + 0.2,
                curl * (1.0 - ease(py_max(0.0, v - 0.6) / 0.4)),
                if hit { 1.0 } else { 0.4 } * (1.0 - v),
            ));
        }
        None
    }

    /// `live_hops(t)`: `(p0, p1, fade, peak)` of the hops whose dotted arcs show at `t` (an arc fades over the hop
    /// and 0.2 s more).
    fn live_hops(&self, p: &PictureCtx, t: f64) -> Vec<([f64; 2], [f64; 2], f64, f64)> {
        let mut out = Vec::new();
        if !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return out;
        }
        let j = self.hops.partition_point(|h| h.t <= t);
        for h in &self.hops[j.saturating_sub(2)..j] {
            let age = t - h.t;
            let life = (h.t_land - h.t) + 0.2;
            if 0.0 <= age && age < life {
                let l = np::hypot(h.p1[0] - h.p0[0], h.p1[1] - h.p0[1]);
                out.push((h.p0, h.p1, 0.55 * (1.0 - age / life), hop_height(l)));
            }
        }
        out
    }

    /// `live_stomps(t)`: the stomps whose rings are on the canvas at `t`, each with its age.
    fn live_stomps(&self, p: &PictureCtx, t: f64) -> Vec<(Stomp, f64)> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return Vec::new();
        }
        let tc = py_min(t, p.t_ko);
        self.stomps
            .iter()
            .filter(|s| s.t <= tc && tc < s.t + 0.34)
            .map(|s| (*s, tc - s.t))
            .collect()
    }

    /// `live_slashes(t)`: `(x, z, aim, age)` of the backstabs of the last 0.3 s.
    fn live_slashes(&self, p: &PictureCtx, t: f64) -> Vec<(f64, f64, f64, f64)> {
        let mut out = Vec::new();
        if !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return out;
        }
        let tc = py_min(t, p.t_ko);
        for i in p.events_between(tc - 0.3, tc, &["hit"]) {
            let e = p.event(i);
            if e.who() == Some(self.k) && how(e) == "stomp" && sub(e) != 0 {
                let v = victim_v(p, e);
                out.push((e.f(5), e.f(6), dmath::atan2(v[1], v[0]), tc - e.t));
            }
        }
        out
    }

    /// `mk_state(t)`: MOUNTAIN KING at `t`: the enemy's place, the angle of the first landing spot, the hops so
    /// far and the strength (it fades over 0.4 s after the dance).
    fn mk_state(&self, p: &PictureCtx, t: f64) -> Option<([f64; 2], f64, usize, f64)> {
        let enemy = 1 - self.k;
        for &(t0, t1) in &self.mks {
            if t0 <= t && t < py_min(t1 + 0.4, p.t_ko) {
                // hs = [h for h in self.HOPS if t0 - 1e-6 <= h[0] <= t and h[5] == 2]
                let mut hs = self.hops.iter().filter(|h| t0 - 1e-6 <= h.t && h.t <= t && h.kind == 2);
                let first = hs.next()?;
                let n = 1 + hs.count();
                let c = p.run.p(p.sim_i(t), enemy);
                let at = p.run.p(p.sim_i(first.t_land), enemy);
                let d = [first.p1[0] - at[0], first.p1[1] - at[1]];
                let k = if t < t1 { 1.0 } else { 1.0 - (t - t1) / 0.4 };
                return Some((c, dmath::atan2(d[1], d[0]), n, k));
            }
        }
        None
    }

    /// `_air(st)`: an airborne ball is painted and depth-written 0.6 nearer (it clears whatever it hops past), its
    /// painter depth taken at the ball's resting height.
    fn air(st: &mut BallState) {
        if st.lift > 0.02 {
            st.air_bias = 0.6;
            st.paint_h = Some(R);
        }
    }
}

/// `P.RUN['V'][P.sim_i(e[0]), e[3]]`: the victim's heading when the hit `e` landed.
///
/// # Panics
/// If the recording has no `V` (every recording has one).
fn victim_v(p: &PictureCtx, e: &Event) -> Vec<f64> {
    let vic = e.i(3) as usize;
    p.run
        .row("V", p.sim_i(e.t), vic)
        .expect("the recording has no V array of two velocities a frame")
}

impl Look for BassoonLook {
    fn shards(&self) -> Vec<String> {
        // ['wood'] * 6 + ['key'] * 8 + ['reed'] * 3: #39's maple sticks, nickel keys, cane blades
        [("wood", 6), ("key", 8), ("reed", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn flash_hits(&self) -> &[&str] {
        &["hook"]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[]
    }

    /// #39: speed lines measured at the hop's height.
    fn speed_lift(&self) -> bool {
        true
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `bassoon.py` reads from `self.cfg`: MOUNTAIN KING's gap and step, BOCAL HOOK's reach.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.low.gap={:?}; abilities.low.step={:?}; abilities.unlock2.reach={:?}",
            self.mk_gap, self.mk_step, self.hook_reach
        ))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        let mine = |kind: &'static str| {
            p.rec
                .events
                .iter()
                .filter(move |e| e.kind == kind && e.who() == Some(k))
        };
        self.hops = mine("hop")
            .map(|e| Hop {
                t: e.t,
                p0: [e.f(3), e.f(4)],
                p1: [e.f(5), e.f(6)],
                t_land: e.f(7),
                kind: e.i(8),
            })
            .collect();
        self.stomps = mine("stomp")
            .map(|e| Stomp {
                t: e.t,
                x: e.f(3),
                z: e.f(4),
                back: e.i(6) != 0,
                rad: e.f(7),
                kind: e.i(8),
            })
            .collect();
        self.hook_open = mine("hook_open").map(|e| (e.t, e.f(3), e.f(4))).collect();
        self.hooks = mine("hook").map(|e| (e.t, e.i(3) != 0, e.f(4))).collect();
        // ends = [q[0] for q in EV if q[1] == 'mountain_end' and q[2] == k and q[0] >= e[0]];  min(ends + [P.T_KO])
        self.mks = mine("mountain")
            .map(|e| {
                let end = mine("mountain_end")
                    .map(|q| q.t)
                    .filter(|&t| t >= e.t)
                    .fold(p.t_ko, py_min);
                (e.t, end)
            })
            .collect();
        self.hook_reach = cfg.unlock2.get("reach");
        self.mk_step = cfg.low.get("step");
        self.mk_gap = cfg.low.get("gap");
    }

    // ------------------------------------------------------------ the ball
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k {
            return;
        }
        let g = &p.show.grid;
        st.sx.get_or_insert(1.0);
        st.sy.get_or_insert(1.0);
        match phase {
            Phase::Rev => {
                // it bobs on its toes in the corner
                let b = g.beat_pulse(t, 0.1, 0.12);
                st.sx = Some(1.0 + 0.06 * b);
                st.sy = Some(1.0 - 0.07 * b);
            }
            Phase::Fight => {
                if t < p.t_ko {
                    if let Some((hop, u)) = self.hop_at(p, t) {
                        st.p = [
                            hop.p0[0] + (hop.p1[0] - hop.p0[0]) * u,
                            hop.p0[1] + (hop.p1[1] - hop.p0[1]) * u,
                        ];
                        let l = np::hypot(hop.p1[0] - hop.p0[0], hop.p1[1] - hop.p0[1]);
                        let (lift, sx, sy) = hop_state(u, l);
                        st.lift = lift;
                        st.sx = Some(sx);
                        st.sy = Some(sy);
                        st.glow = 0.15 + 0.25 * if hop.kind == 2 { 1.0 } else { 0.0 };
                    } else {
                        let (sx, sy) = land_squash(self.last_stomp(t));
                        st.sx = Some(sx);
                        st.sy = Some(sy);
                    }
                }
                Self::air(st);
            }
            Phase::Tail => {
                // back on its toes in the corner; as champion a little hop on every solo note
                if t >= g.t_rev {
                    let b = g.beat_pulse(t, 0.1, 0.12) * ss(g.t_rev, g.t_rev + 0.3, t);
                    st.sx = Some(1.0 + 0.06 * b);
                    st.sy = Some(1.0 - 0.07 * b);
                }
                if kk == p.win && p.t_solo <= t && t < g.t_reassemble {
                    let e = p.solo_env(t);
                    st.lift = 0.35 * (1.0 - e) * e * 4.0;
                }
                Self::air(st);
            }
        }
    }

    // ------------------------------------------------------------ events -> camera, sparks, glows
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "hit" && e.t <= p.t_ko {
            match how(e) {
                "stomp" => return Some(if sub(e) == 0 { 0.4 } else { 0.9 }),
                "hook" => return Some(1.0),
                _ => {}
            }
        }
        None
    }

    fn rope(&self, _p: &PictureCtx, _e: &Event) -> Option<f64> {
        None
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        // col = self.P.COL[e[2]] * 0.6 + 100
        let col = p.col[attacker(e)].map(|c| c * 0.6 + 100.0);
        let (n, sp, life) = match how(e) {
            // a backstab throws more
            "stomp" if sub(e) != 0 => (12, 2.8, 0.26),
            "stomp" => (5, 1.6, 0.18),
            "hook" => (16, 3.2, 0.3),
            _ => (8, 2.4, 0.22),
        };
        (col, n, sp, life)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        if big(e) { Some((8.0, 0.2)) } else { None }
    }

    fn fringe(&self, _p: &PictureCtx, e: &Event) -> bool {
        big(e)
    }

    // ------------------------------------------------------------ effects (reelkit.fx), in #39's order
    /// TIPTOE: every landing a stomp: maple dust, a print on the canvas, a green flare (#39 built these first).
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        for s in &self.stomps {
            if s.t > p.t_ko {
                break;
            }
            let back = if s.back { 1.0 } else { 0.0 };
            let emit = Emit {
                n: Some(3 + 2 * i64::from(s.back)),
                col: Some(MAPLE_DUST.to_vec()),
                scale: 0.8,
                ..Emit::default()
            };
            fx.pa.emit(s.t, scene::to_world(s.x, s.z, 0.05), "dust", &emit);
            fx.mk.add(
                s.t,
                Mark {
                    size: 0.13,
                    col: [40.0, 30.0, 30.0],
                    life: 2.4,
                    // seed=int(t_ * 100)
                    seed: seed_of(s.t * 100.0),
                    strength: 0.3,
                    ..Mark::new(MarkKind::Scorch, s.x, s.z)
                },
            );
            // radius=1.2 + 0.6 * back, peak=0.35 + 0.45 * back + 0.2 * (kind == 2), decay=0.18
            let peak = 0.35 + 0.45 * back + 0.2 * if s.kind == 2 { 1.0 } else { 0.0 };
            fx.fl.add(
                s.t,
                scene::to_world(s.x, s.z, R * 0.5),
                p.fl_col[k],
                1.2 + 0.6 * back,
                peak,
                0.03,
                0.18,
                0.0,
            );
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind != "hit" {
            return;
        }
        let att = attacker(e);
        let (t, x, z) = (e.t, e.f(5), e.f(6));
        let pos = scene::to_world(x, z, R);
        match how(e) {
            "stomp" if sub(e) != 0 => {
                // a backstab: green slashes, wood chips
                fx.bu.add(t, pos, p.col[att], 1.1, None, true);
                let mut col = GREEN_SP.to_vec();
                col.push(MAPLE_DUST[0]);
                let emit = Emit {
                    n: Some(8),
                    col: Some(col),
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "chips", &emit);
                let v = victim_v(p, e);
                fx.mk.add(
                    t,
                    Mark {
                        size: 0.5,
                        angle: dmath::atan2(v[1], v[0]) + 1.2,
                        col: [20.0, 60.0, 36.0],
                        life: 2.6,
                        n: 2,
                        seed: seed_of(t * 100.0),
                        strength: 0.6,
                        ..Mark::new(MarkKind::Claw, x, z)
                    },
                );
            }
            "stomp" => {
                let emit = Emit {
                    n: Some(4),
                    col: Some(GREEN_SP.to_vec()),
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "sparks", &emit);
            }
            "hook" => {
                // BOCAL HOOK lands
                fx.bu.add(t, pos, [220.0, 226.0, 240.0], 1.0, None, true);
                fx.fl.add(t, pos, p.fl_col[att], 1.6, 0.8, 0.03, 0.3, 0.0);
                let emit = Emit {
                    n: Some(10),
                    col: Some(vec![[226.0, 228.0, 240.0], [255.0, 255.0, 255.0], GREEN_SP[0]]),
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "sparks", &emit);
            }
            _ => {}
        }
    }

    /// MOUNTAIN KING: a green light on the enemy that keeps up the dance (every 8th).
    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        for &(t0, t1) in &self.mks {
            let mut tb = t0;
            while tb < t1 {
                let at = p.run.p(p.sim_i(tb), 1 - k);
                fx.fl.add(
                    tb,
                    scene::to_world(at[0], at[1], R),
                    p.fl_col[k],
                    2.0,
                    0.5,
                    0.03,
                    0.3,
                    0.0,
                );
                tb += p.show.grid.beat / 2.0;
            }
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
        fx.pa.emit(p.t_ko, pos, "chips", &emit);
    }

    // ------------------------------------------------------------ the canvas
    /// The stomp rings (a backstab's bigger and double).
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let col = p.col[self.k];
        let bayer = config::bayer_full();
        for (s, age) in self.live_stomps(p, c.t) {
            // stomp_light: life = 0.32 if back else 0.24; None when age < 0 or age >= life
            let life = if s.back { 0.32 } else { 0.24 };
            if age < 0.0 || age >= life {
                continue;
            }
            let u = age / life;
            // (band * (0.9 + 0.5 * back))[:, None] * (col / 255.0 * 1.2 + 0.25)[None]
            let gain = 0.9 + 0.5 * if s.back { 1.0 } else { 0.0 };
            let tint = col.map(|v| v / 255.0 * 1.2 + 0.25);
            for (j, light) in ic.iter_mut().enumerate() {
                let rd = np::hypot(c.sxp[j] - s.x, c.szp[j] - s.z);
                let band = stomp_band(rd, u, s.rad, s.back);
                // band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
                let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
                let v = band * gain;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
    }

    // ------------------------------------------------------------ painter: BOCAL HOOK's crook over both balls
    fn items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>, _st: &BallState) -> Vec<(f64, Payload)> {
        if f.in_fight && self.crook_state(p, f.t).is_some() {
            let crook: Payload = Box::new(Crook);
            return vec![(-1e9, crook)];
        }
        Vec::new()
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if payload.downcast_ref::<Crook>().is_none() {
            return;
        }
        let st = &f.fs[self.k];
        let (mut arr, depth) = (fr.array(), fr.depth());
        let (aim, ext, curl, glow) = self
            .crook_state(p, f.t)
            .expect("a crook item is queued only while the crook is out");
        draw_crook(&mut arr, f.cam, st.p, aim, ext, curl, R + st.lift, p.col[self.k], glow);
        fr.set_array(&arr, Some(&depth));
    }

    // ------------------------------------------------------------ over the ring: hop arcs, slashes, the dance
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let (cam, t, col) = (f.cam, f.t, p.col[self.k]);
        for (p0, p1, fade, peak) in self.live_hops(p, t) {
            draw_hop_arc(arr, cam, p0, p1, fade, col, R, peak);
        }
        for (x, z, aim, age) in self.live_slashes(p, t) {
            backstab_slash(arr, cam, x, z, aim, age, col, R);
        }
        let mk = if f.in_fight { self.mk_state(p, t) } else { None };
        if let Some((c, a0, n, kk)) = mk {
            let step = self
                .mk_step
                .expect("MOUNTAIN KING's trail: abilities.low has no step (Python raises KeyError)");
            let gap = self
                .mk_gap
                .expect("MOUNTAIN KING's trail: abilities.low has no gap (Python raises KeyError)");
            let pts = mk_trail(cam, c, a0, step, n, gap);
            let ccol = tinted(col, 0.7, 90.0);
            for (j, &(x, y)) in pts.iter().enumerate() {
                let (xi, yi) = (round_px(x), round_px(y));
                if 2 <= xi && xi < W as i64 - 2 && 2 <= yi && yi < H as i64 - 2 {
                    let kj = kk * (0.5 + 0.5 * if j == pts.len() - 1 { 1.0 } else { 0.0 });
                    for yy in yi - 1..yi + 2 {
                        for xx in xi - 1..xi + 2 {
                            mix_px(arr, xx as usize, yy as usize, ccol, kj);
                        }
                    }
                }
            }
        }
    }

    /// #39's: a stick of the maple body / a cane blade, a nickel key chip.
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
        let to = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            cam.wp(&world)
        };
        if kind == "wood" || kind == "reed" {
            let (ln, wd) = if kind == "wood" { (0.26, 0.035) } else { (0.12, 0.03) };
            let q4: Vec<[f64; 2]> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(a1, a2)| {
                    [
                        sx + d[0] * ln * a1 + nn[0] * wd * a2,
                        sz + d[1] * ln * a1 + nn[1] * wd * a2,
                    ]
                })
                .collect();
            layer_blend(arr, &[to(&q4)], if kind == "wood" { WOOD } else { CANE_SH }, 1.0);
            return;
        }
        let key = kind == "key";
        let col = if key { NICKEL_SH } else { WOOD.map(|c| c * 1.3) };
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
        layer_blend(arr, &[to(&tri)], col.map(|c| np::clip(c * shade_k, 0.0, 255.0)), 1.0);
    }

    // ------------------------------------------------------------ after the grade: the hooked enemy whips round
    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let enemy = 1 - self.k;
        if f.in_fight
            && let Some(d) = drawn.iter().find(|d| d.k == enemy)
        {
            swirl(img, d.s.0, d.s.1, d.rp, self.hooked_k(p, f.t), f.t, p.col[self.k]);
        }
    }

    // ------------------------------------------------------------ checks
    fn focus_points(&self, _p: &PictureCtx, _t: f64, st: &BallState) -> Vec<(V3, String)> {
        vec![(scene::to_world(st.p[0], st.p[1], R + st.lift), "bassoon".to_string())]
    }
}

/// `_big(e)`: a hook, or a backstab.
fn big(e: &Event) -> bool {
    how(e) == "hook" || (how(e) == "stomp" && sub(e) == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hop is tall even when short, stretched while it moves fast and round at the top.
    #[test]
    fn a_hop_rises_stretched_and_is_round_at_the_top() {
        assert_eq!(hop_height(0.0), 0.42);
        assert_eq!(hop_height(9.0), 0.42 + 0.16 * 2.5);
        assert_eq!(hop_state(0.0, 1.0), (0.0, 1.0 / 1.16f64.sqrt(), 1.16));
        let (top, sx, sy) = hop_state(0.5, 1.0);
        assert_eq!((top, sx, sy), (hop_height(1.0), 1.0, 1.0));
        assert_eq!(hop_state(1.0, 1.0).0, 0.0);
    }

    /// A landing is flat and wide at first, bounces back a little, and is over after 0.2 s.
    #[test]
    fn a_landing_squashes_then_bounces_back() {
        assert_eq!(land_squash(-0.01), (1.0, 1.0));
        assert_eq!(land_squash(0.21), (1.0, 1.0));
        assert_eq!(land_squash(99.0), (1.0, 1.0));
        assert_eq!(land_squash(0.0), (1.24, 0.74));
        let (sx, sy) = land_squash(0.1);
        assert!(sx < 1.0 && sy > 1.0, "the bounce back is tall and narrow");
        let (sx, sy) = land_squash(0.2);
        assert!((sx - 1.0).abs() < 1e-12 && (sy - 1.0).abs() < 1e-12);
    }

    /// The crook is ten points along the aim; a curl adds seven more round its end.
    #[test]
    fn the_crook_swings_out_and_curls() {
        let straight = crook_points([0.0, 0.0], 0.0, 1.2, R, 0.0, R);
        assert_eq!(straight.len(), 10);
        assert_eq!(straight[0], [R * 0.9, 0.0, R + 0.10]);
        assert!((straight[9][0] - (R * 0.9 + 1.2)).abs() < 1e-12);
        let hooked = crook_points([0.0, 0.0], 0.0, 1.2, R, 2.4, R);
        assert_eq!(hooked.len(), 17);
        for q in &hooked[10..] {
            let c = [straight[9][0], straight[9][1] + 0.22];
            assert!((np::hypot(q[0] - c[0], q[1] - c[1]) - 0.22).abs() < 1e-12);
            assert_eq!(q[2], straight[9][2]);
        }
    }

    /// A stomp's ring grows and fades; a backstab's has a second ring inside it.
    #[test]
    fn a_stomp_ring_grows_and_a_backstab_doubles_it() {
        let r = |u: f64, back: bool| 0.35 + (0.2 + 0.5 + if back { 0.5 } else { 0.0 }) * (1.0 - (1.0 - u) * (1.0 - u));
        assert_eq!(stomp_band(r(0.5, false), 0.5, 0.2, false), 0.5);
        assert!(stomp_band(r(0.5, false) + 0.07, 0.5, 0.2, false) < 1e-12);
        assert_eq!(stomp_band(r(0.5, false) + 0.08, 0.5, 0.2, false), 0.0);
        assert_eq!(stomp_band(r(0.5, true) * 0.62, 0.5, 0.2, true), 0.5 * 0.8);
        assert_eq!(stomp_band(r(0.5, true) * 0.62, 0.5, 0.2, false), 0.0);
    }

    /// numpy's slices of the thread nub's rows: clipped at the frame, a negative end counted from the far edge.
    #[test]
    fn the_nub_is_sliced_as_numpy_slices() {
        assert_eq!(py_slice(4, 7, 640), 4..7);
        assert_eq!(py_slice(0, 2, 640), 0..2);
        assert_eq!(py_slice(638, 641, 640), 638..640);
        assert_eq!(py_slice(700, 703, 640), 640..640);
        assert_eq!(py_slice(0, -1, 640), 0..639);
        assert_eq!(py_slice(0, 0, 640), 0..0);
    }

    /// The module's own layer blend: the picture's share in `float32`, the colour's in `float64`.
    #[test]
    fn the_shard_blend_is_the_modules_own() {
        let mut arr: Img<f32> = Img::filled(W, H, &[100.0, 50.0, 0.1]);
        let quad = vec![(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 20.0)];
        layer_blend(&mut arr, &[quad], WOOD, 1.0);
        assert_eq!(arr.px(15, 15), &[120.0, 56.0, 52.0]);
        assert_eq!(arr.px(5, 5), &[100.0, 50.0, 0.1]);
    }
}
