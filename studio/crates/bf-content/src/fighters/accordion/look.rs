//! ACCORDION's look: #30's `render.py` ACCORDION layers and the `styles.py` draw code they call (`bite_open`,
//! `draw_jaws`, `clamp_state`, `draw_clamp`, `draw_squeezebox`), ported close enough for the eye (ruling M4-R16).
//!
//! What it draws: BELLOWS BITE's jaws (the two end boards fly apart with the pleats fanned between them, then snap
//! shut) and the mouth's light on the canvas; FULL BELLOWS, a low pleated bellows wall across the ring that slams shut
//! on the enemy (first in the painter pass, so the fighters stay readable); SQUEEZEBOX, the folded collar of pleat
//! rings around the ball with its buttons glowing on the beat; its hits' sparks, bursts, claw marks and the clamp's
//! crater; its shards (mother-of-pearl buttons, pleats, piano keys).

use bf_core::dmath::{self, PI};
use bf_core::event::{Event, Value};
use bf_core::py::{fmod_py, py_max, py_min};
use bf_core::roster::{AbilityKind, FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config;
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DefaultLook, FrameCtx, Fx, Look, Payload, Phase, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{self, PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, A, R_BALL as R};

use crate::fighters::shared::older::{Poly, annulus_polys, blend_into, mask_of_all, mask_of_lines};
use crate::fighters::shared::reeds::Num;

/// `PLEAT_DK`.
pub const PLEAT_DK: V3 = [46.0, 16.0, 44.0];
/// `PLEAT_EDGE`.
const PLEAT_EDGE: V3 = [255.0, 236.0, 250.0];
/// `BOARD`.
const BOARD: V3 = [34.0, 24.0, 40.0];
/// `BUTTON`.
const BUTTON: V3 = [248.0, 242.0, 228.0];
/// The claw marks' pale (`PALE`).
const PALE: V3 = [255.0, 240.0, 248.0];

/// #30's hit kinds with the big spark spray.
const BIG: [&str; 5] = ["flight", "bodyslam", "clamp", "wave", "shockwave"];

/// `e[7]`, the `how` of a `hit` event.
fn how(e: &Event) -> &str {
    match e.fields.get(5) {
        Some(Value::Str(s)) => s,
        _ => "",
    }
}

fn sc(a: V3, k: f64) -> V3 {
    a.map(|v| v * k)
}

fn clip255(a: V3) -> V3 {
    a.map(|v| np::clip(v, 0.0, 255.0))
}

fn add3(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// One BELLOWS BITE as the look rebuilds it from the `bite` and `snap` events.
#[derive(Clone, Copy, Debug)]
struct Bite {
    t0: f64,
    t_snap: f64,
    aim: f64,
    reach: f64,
    landed: bool,
}

/// One FULL BELLOWS from the `clamp` and `clamp_snap` events.
#[derive(Clone, Copy, Debug)]
struct Clamp {
    t0: f64,
    t_snap: f64,
    axis: f64,
    c: [f64; 2],
    q: [f64; 2],
}

/// What it queues in the painter pass.
enum Item {
    Jaws(Bite, (f64, f64)),
    Clamp(Clamp),
    Collar(bool),
}

/// `bite_open(t, t0, t_snap)`: (openness, plate extension) of a bite, `None` when idle.
pub fn bite_open(t: f64, t0: f64, t_snap: f64) -> Option<(f64, f64)> {
    if t < t0 {
        return None;
    }
    if t < t_snap - 0.06 {
        let u = (t - t0) / py_max(1e-6, t_snap - 0.06 - t0);
        let w = 1.0 - py_min(1.0, u);
        let e = 1.0 - w * w;
        return Some((e, py_min(1.0, u * 1.6)));
    }
    if t < t_snap {
        let u = (t - (t_snap - 0.06)) / 0.06;
        return Some((1.0 - u * u, 1.0));
    }
    if t < t_snap + 0.10 {
        return Some((0.0, 1.0));
    }
    if t < t_snap + 0.24 {
        return Some((0.0, 1.0 - (t - t_snap - 0.10) / 0.14));
    }
    None
}

/// `rope_dist(c, d, A)`: the distance from `c` along unit `d` to the ropes.
fn rope_dist(c: [f64; 2], d: [f64; 2]) -> f64 {
    let mut m = f64::INFINITY;
    for ax in 0..2 {
        if d[ax].abs() > 1e-9 {
            let edge = if d[ax] > 0.0 { A } else { -A };
            m = m.min((edge - c[ax]) / d[ax]);
        }
    }
    py_max(0.2, m)
}

fn lerp(a: [f64; 2], b: [f64; 2], e: f64) -> [f64; 2] {
    [a[0] + (b[0] - a[0]) * e, a[1] + (b[1] - a[1]) * e]
}

/// `clamp_state(t, ...)`: (far board, near board, alpha) of FULL BELLOWS, `None` when idle.
fn clamp_state(t: f64, c: &Clamp) -> Option<([f64; 2], [f64; 2], f64)> {
    let d = [dmath::cos(c.axis), dmath::sin(c.axis)];
    let (fd, nd) = (rope_dist(c.c, d) - 0.12, rope_dist(c.c, [-d[0], -d[1]]) - 0.12);
    let far = [c.c[0] + d[0] * fd, c.c[1] + d[1] * fd];
    let near = [c.c[0] - d[0] * nd, c.c[1] - d[1] * nd];
    let proj = (c.q[0] - c.c[0]) * d[0] + (c.q[1] - c.c[1]) * d[1];
    let qa = [c.c[0] + d[0] * proj, c.c[1] + d[1] * proj];
    let qf = [qa[0] + d[0] * 0.32, qa[1] + d[1] * 0.32];
    let qn = [qa[0] - d[0] * 0.32, qa[1] - d[1] * 0.32];
    if t < c.t0 {
        return None;
    }
    if t < c.t_snap - 0.12 {
        let u = py_min(1.0, (t - c.t0) / py_max(1e-6, c.t_snap - 0.12 - c.t0) * 1.4);
        let w = 1.0 - u;
        let e = 1.0 - w * w * w;
        return Some((lerp(c.c, far, e), lerp(c.c, near, e), py_min(1.0, (t - c.t0) / 0.1)));
    }
    if t < c.t_snap {
        let u = (t - (c.t_snap - 0.12)) / 0.12;
        let e = u * u;
        return Some((lerp(far, qf, e), lerp(near, qn, e), 1.0));
    }
    if t < c.t_snap + 0.35 {
        return Some((qf, qn, 1.0));
    }
    if t < c.t_snap + 0.75 {
        let u = (t - c.t_snap - 0.35) / 0.4;
        let back = [(c.c[0] - qa[0]) * u, (c.c[1] - qa[1]) * u];
        return Some((
            [qf[0] + back[0], qf[1] + back[1]],
            [qn[0] + back[0], qn[1] + back[1]],
            1.0 - u,
        ));
    }
    None
}

/// `quad_ring(cx, cz, u, v, a, b, y)`: a world quad in the ring frame.
fn quad_ring(cx: f64, cz: f64, u: [f64; 2], v: [f64; 2], a: f64, b: f64, y: f64) -> [V3; 4] {
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .map(|(i, j)| scene::to_world(cx + u[0] * a * i + v[0] * b * j, cz + u[1] * a * i + v[1] * b * j, y))
}

fn mean3(q: &[V3]) -> V3 {
    let n = q.len() as f64;
    let mut m = [0.0; 3];
    for p in q {
        for ch in 0..3 {
            m[ch] += p[ch] / n;
        }
    }
    m
}

/// `box_faces(...)`: the camera-facing faces of a box, back to front, the top last: `[(screen quad, shade)]`.
#[allow(clippy::too_many_arguments)] // the Python signature
fn box_faces(
    cam: &Cam,
    cx: f64,
    cz: f64,
    u: [f64; 2],
    v: [f64; 2],
    a: f64,
    b: f64,
    y0: f64,
    y1: f64,
) -> Vec<(Poly, f64)> {
    let (lo, hi) = (quad_ring(cx, cz, u, v, a, b, y0), quad_ring(cx, cz, u, v, a, b, y1));
    let (ml, mh) = (mean3(&lo), mean3(&hi));
    let c = [(ml[0] + mh[0]) / 2.0, (ml[1] + mh[1]) / 2.0, (ml[2] + mh[2]) / 2.0];
    let mut faces: Vec<(f64, Poly, f64)> = Vec::new();
    for e in 0..4 {
        let q = [lo[e], lo[(e + 1) % 4], hi[(e + 1) % 4], hi[e]];
        let mid = mean3(&q);
        let dot: f64 = (0..3).map(|ch| (mid[ch] - c[ch]) * (cam.pos[ch] - mid[ch])).sum();
        if dot > 0.0 {
            faces.push((cam.depth(&[mid])[0], cam.wp(&q), 0.72 + 0.12 * (e % 2) as f64));
        }
    }
    faces.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out: Vec<(Poly, f64)> = faces.into_iter().map(|(_, q, k)| (q, k)).collect();
    out.push((cam.wp(&hi), 1.0));
    out
}

/// `dots(arr, pts, col, k, size)`.
fn dots(arr: &mut Img<f32>, pts: &[(f64, f64)], col: V3, k: f64, size: usize) {
    let (w, h) = (config::W as i64, config::H as i64);
    for &(x, y) in pts {
        let (xi, yi) = (x.round() as i64, y.round() as i64);
        if xi < 0 || yi < 0 || xi + size as i64 >= w || yi + size as i64 >= h {
            continue;
        }
        for yy in yi as usize..=yi as usize + size {
            for xx in xi as usize..=xi as usize + size {
                for (v, c) in arr.px_mut(xx, yy).iter_mut().zip(col) {
                    *v = (f64::from(*v) * (1.0 - k) + c * k) as f32;
                }
            }
        }
    }
}

fn fill(arr: &mut Img<f32>, polys: &[Poly], col: V3, k: f64) {
    if polys.is_empty() {
        return;
    }
    let m = mask_of_all(polys);
    blend_into(arr, &m, col, Num::Py(k));
}

/// The screen quads of a flat ring arc, one per segment.
#[allow(clippy::too_many_arguments)]
fn arc_polys(cam: &Cam, cx: f64, cz: f64, r_in: f64, r_out: f64, y: f64, n: usize, a0: f64, a1: f64) -> Vec<Poly> {
    annulus_polys(cam, cx, cz, r_in, r_out, y, n, a0, a1)
        .into_iter()
        .map(|q| q.to_vec())
        .collect()
}

/// `draw_jaws(arr, cam, p, aim, half_deg, reach, opn, ext, col)`: the open bellows between the two boards.
#[allow(clippy::too_many_arguments)]
fn draw_jaws(
    arr: &mut Img<f32>,
    cam: &Cam,
    p: [f64; 2],
    aim: f64,
    half_deg: f64,
    reach: f64,
    opn: f64,
    ext: f64,
    col: V3,
) {
    let y = R * 0.95;
    let a = half_deg.to_radians() * 1.15 * opn;
    let r0 = R * 0.9;
    let r1 = R + reach * ext;
    if a > 0.02 && r1 > R * 1.05 {
        let n = ((a * 12.0) as usize).max(6);
        let polys = arc_polys(cam, p[0], p[1], r0, r1 * 0.97, y, n * 2, aim - a, aim + a);
        let fold: Vec<Poly> = polys.iter().step_by(2).cloned().collect();
        let crease: Vec<Poly> = polys.iter().skip(1).step_by(2).cloned().collect();
        fill(arr, &fold, clip255(col.map(|c| c * 1.05 + 22.0)), 0.92);
        fill(arr, &crease, add3(PLEAT_DK, sc(col, 0.22)), 0.92);
        let tips: Vec<V3> = (0..=n * 2)
            .step_by(2)
            .map(|j| {
                let ang = aim - a + 2.0 * a * j as f64 / (n * 2) as f64;
                scene::to_world(
                    p[0] + r1 * 0.97 * dmath::cos(ang),
                    p[1] + r1 * 0.97 * dmath::sin(ang),
                    y + 0.01,
                )
            })
            .collect();
        dots(arr, &cam.wp(&tips), PLEAT_EDGE, 0.85, 0);
    }
    for side in [-1.0, 1.0] {
        let ang = aim + side * a;
        let d = [dmath::cos(ang), dmath::sin(ang)];
        let nrm = [-d[1], d[0]];
        let rm = (R * 1.02 + r1) / 2.0;
        let hl = py_max(0.05, (r1 - R * 1.02) / 2.0);
        let c = [p[0] + d[0] * rm, p[1] + d[1] * rm];
        for (q, k) in box_faces(cam, c[0], c[1], d, nrm, hl, 0.07, y - 0.10, y + 0.06) {
            let shade = clip255(add3(sc(BOARD, 0.8 + 0.5 * k), sc(col, 0.10 * k)));
            fill(arr, &[q], shade, 1.0);
        }
        let trim = quad_ring(c[0], c[1], d, nrm, hl, 0.025, y + 0.065);
        fill(arr, &[cam.wp(&trim)], clip255(col.map(|v| v * 1.1 + 30.0)), 1.0);
        let bpts: Vec<V3> = (0..3)
            .map(|j| r1 - 0.06 - 0.1 * f64::from(j))
            .filter(|&r| r > R)
            .map(|r| scene::to_world(p[0] + d[0] * r, p[1] + d[1] * r, y + 0.07))
            .collect();
        if !bpts.is_empty() {
            dots(arr, &cam.wp(&bpts), BUTTON, 1.0, 0);
        }
    }
}

/// `draw_clamp(arr, cam, a_pos, b_pos, axis, col, alpha, width=1.1, h=0.32)`: the pleated bellows wall.
fn draw_clamp(arr: &mut Img<f32>, cam: &Cam, a_pos: [f64; 2], b_pos: [f64; 2], axis: f64, col: V3, alpha: f64) {
    let (width, h) = (1.1, 0.32);
    let d = [dmath::cos(axis), dmath::sin(axis)];
    let nrm = [-d[1], d[0]];
    let l = (a_pos[0] - b_pos[0]) * d[0] + (a_pos[1] - b_pos[1]) * d[1];
    let n = ((l.abs() / 0.22) as usize).max(2);
    let at = |s: f64| lerp(b_pos, a_pos, s);
    // (depth, is a board, j, centre, half-length, width)
    let mut items: Vec<(f64, bool, usize, [f64; 2], f64, f64)> = Vec::new();
    for j in 0..n {
        let ca = at((j as f64 + 0.5) / n as f64);
        let hl = l.abs() / n as f64 / 2.0;
        let wj = width * if j % 2 == 0 { 1.0 } else { 0.86 };
        let dz = cam.depth(&[scene::to_world(ca[0], ca[1], h * 0.5)])[0];
        items.push((dz, false, j, ca, hl, wj));
    }
    for (j, pos) in [(0, a_pos), (1, b_pos)] {
        let dz = cam.depth(&[scene::to_world(pos[0], pos[1], h * 0.5)])[0];
        items.push((dz, true, j, pos, 0.08, width * 1.12));
    }
    items.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, board, j, cc, hl, wj) in items {
        if !board {
            let base = if j % 2 == 0 {
                clip255(col.map(|v| v * 1.05 + 20.0))
            } else {
                add3(PLEAT_DK, sc(col, 0.22))
            };
            let top = h * if j % 2 == 1 { 0.96 } else { 1.0 };
            for (q, k) in box_faces(cam, cc[0], cc[1], d, nrm, hl, wj / 2.0, 0.02, top) {
                fill(arr, &[q], clip255(sc(base, 0.75 + 0.35 * k)), alpha);
            }
        } else {
            let top = py_max(0.9, h * 1.08);
            for (q, k) in box_faces(cam, cc[0], cc[1], d, nrm, hl, wj / 2.0, 0.0, top) {
                fill(arr, &[q], clip255(add3(sc(BOARD, 0.8 + 0.5 * k), sc(col, 0.12))), alpha);
            }
            let pts: Vec<V3> = (0..7)
                .map(|i| {
                    let u = f64::from(i) / 6.0;
                    let o = wj / 2.0 * (u * 1.6 - 0.8);
                    scene::to_world(cc[0] + nrm[0] * o, cc[1] + nrm[1] * o, top + 0.01)
                })
                .collect();
            dots(arr, &cam.wp(&pts), BUTTON, alpha, 1);
        }
    }
    let lines: Vec<Poly> = (0..n)
        .step_by(2)
        .map(|j| {
            let cc = at((j as f64 + 0.5) / n as f64);
            let e0 = scene::to_world(cc[0] - nrm[0] * width / 2.0, cc[1] - nrm[1] * width / 2.0, h + 0.01);
            let e1 = scene::to_world(cc[0] + nrm[0] * width / 2.0, cc[1] + nrm[1] * width / 2.0, h + 0.01);
            cam.wp(&[e0, e1])
        })
        .collect();
    let m = mask_of_lines(&lines, Some(1));
    blend_into(arr, &m, PLEAT_EDGE, Num::Py(0.7 * alpha));
}

/// `draw_squeezebox(...)`: the collar (three stacked pleat rings folded shut around the ball) and two rings of
/// glowing buttons; `front` picks the half nearer the camera.
#[allow(clippy::too_many_arguments)]
fn draw_squeezebox(
    arr: &mut Img<f32>,
    cam: &Cam,
    cx: f64,
    cz: f64,
    t: f64,
    k: f64,
    front: bool,
    col: V3,
    close: f64,
    glow: f64,
) {
    let n = 32;
    for layer in 0..3usize {
        let lf = layer as f64;
        let y = R * (0.62 + 0.38 * lf) * (1.0 - 0.15 * close) + 0.05;
        let r_out = R * (1.85 - 0.45 * close) - 0.04 * lf;
        let polys = arc_polys(cam, cx, cz, R * 0.96, r_out, y, n, 0.0, 2.0 * PI);
        let c0 = cam.depth(&[scene::to_world(cx, cz, y)])[0];
        let (mut fold, mut crease) = (Vec::new(), Vec::new());
        for (j, q) in polys.into_iter().enumerate() {
            let a0 = 2.0 * PI * j as f64 / n as f64;
            let a1 = 2.0 * PI * (j + 1) as f64 / n as f64;
            let o0 = scene::to_world(cx + r_out * dmath::cos(a0), cz + r_out * dmath::sin(a0), y);
            let o1 = scene::to_world(cx + r_out * dmath::cos(a1), cz + r_out * dmath::sin(a1), y);
            let dz = cam.depth(&[o0, o1]);
            if ((dz[0] + dz[1]) / 2.0 < c0) != front {
                continue;
            }
            if (j + layer) % 2 == 0 {
                fold.push(q);
            } else {
                crease.push(q);
            }
        }
        fill(arr, &fold, clip255(col.map(|v| v * (0.95 + 0.1 * lf) + 20.0)), 0.86 * k);
        fill(arr, &crease, add3(PLEAT_DK, sc(col, 0.2)), 0.86 * k);
    }
    let rb = R * (1.62 - 0.42 * close);
    let c0 = cam.depth(&[scene::to_world(cx, cz, R)])[0];
    for (ring, (yy, nb)) in [(R * 1.05, 10), (R * 0.72, 12)].into_iter().enumerate() {
        for j in 0..nb {
            let ang = f64::from(j) * 2.0 * PI / f64::from(nb) + 0.3 * ring as f64 + t * 1.5;
            let pw = scene::to_world(cx + rb * dmath::cos(ang), cz + rb * dmath::sin(ang), yy);
            if (cam.depth(&[pw])[0] < c0) != front {
                continue;
            }
            let g = clip255(add3(sc(BUTTON, 0.75 + 0.4 * glow), sc(col, 0.25 * glow)));
            dots(arr, &cam.wp(&[pw]), g, k, usize::from(glow > 0.5));
        }
    }
}

/// ACCORDION's look.
#[derive(Clone, Debug, Default)]
pub struct AccordionLook {
    /// Its side.
    k: usize,
    /// Its bites, from the run.
    bites: Vec<Bite>,
    /// Its FULL BELLOWS, from the run.
    clamps: Vec<Clamp>,
    /// The jaws' half-angle in degrees (its weapon's `half`).
    half: f64,
    /// SQUEEZEBOX's length in seconds, if it has a shield.
    shield_dur: Option<f64>,
}

impl AccordionLook {
    fn bites_at(&self, t: f64) -> impl Iterator<Item = &Bite> {
        self.bites.iter().filter(move |b| b.t0 <= t && t <= b.t_snap + 0.25)
    }

    fn clamps_at(&self, t: f64) -> impl Iterator<Item = &Clamp> {
        self.clamps.iter().filter(move |c| c.t0 <= t && t <= c.t_snap + 0.75)
    }
}

impl Look for AccordionLook {
    fn shards(&self) -> Vec<String> {
        [("button", 6), ("pleat", 6), ("key", 4)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_flash(&self) -> &[&str] {
        &["clamp"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["clamp"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["clamp"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.half = match &cfg.weapon {
            WeaponCfg::Bite(w) => w.half,
            _ => 38.0,
        };
        self.shield_dur = [&cfg.unlock1, &cfg.unlock2, &cfg.low]
            .into_iter()
            .find(|a| a.kind == AbilityKind::Shield)
            .and_then(|a| a.get("dur_beats"))
            .map(|b| b * p.show.grid.beat);
        self.bites.clear();
        self.clamps.clear();
        let mut open: Option<usize> = None;
        for e in &p.rec.events {
            if e.who() != Some(k) {
                continue;
            }
            match e.kind.as_str() {
                "bite" => {
                    self.bites.push(Bite {
                        t0: e.t,
                        t_snap: e.t + e.f(6),
                        aim: e.f(4),
                        reach: e.f(5),
                        landed: false,
                    });
                    open = Some(self.bites.len() - 1);
                }
                "snap" => {
                    if let Some(j) = open.take() {
                        self.bites[j].t_snap = e.t;
                        self.bites[j].landed = e.f(4) != 0.0;
                    }
                }
                "clamp" => self.clamps.push(Clamp {
                    t0: e.t,
                    t_snap: e.t + e.f(4),
                    axis: e.f(3),
                    c: [e.f(5), e.f(6)],
                    q: [e.f(5), e.f(6)],
                }),
                "clamp_snap" => {
                    if let Some(c) = self.clamps.last_mut() {
                        c.t_snap = e.t;
                        c.q = [e.f(4), e.f(5)];
                    }
                }
                _ => {}
            }
        }
    }

    // ------------------------------------------------------------ the ball lights up as the jaws snap
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k || phase != Phase::Fight {
            return;
        }
        let tc = py_min(t, p.t_ko);
        for b in self.bites_at(tc) {
            if tc >= b.t_snap - 0.06 {
                let g = (if b.landed { 0.6 } else { 0.3 }) * dmath::exp(-py_max(0.0, tc - b.t_snap) / 0.1);
                st.glow = py_max(st.glow, g);
            }
        }
    }

    // ------------------------------------------------------------ events -> camera, sparks, glows
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "hit" && e.who() == Some(self.k) {
            match how(e) {
                "bite" => return Some(0.7),
                "clamp" => return Some(2.5),
                _ => {}
            }
        }
        DefaultLook.shake(p, e)
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let h = how(e);
        let (n, sp, life) = if BIG.contains(&h) {
            (16, 3.2, 0.3)
        } else if h == "bite" {
            (8, 2.2, 0.22)
        } else {
            (3, 1.4, 0.16)
        };
        let col = p.col[ring::owner(e).unwrap_or(self.k)];
        (col.map(|c| c * 0.6 + 100.0), n, sp, life)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "bite" => Some((7.0, 0.22)),
            "clamp" => Some((12.0, 0.35)),
            _ => None,
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.who() != Some(self.k) || e.t > p.t_ko + 0.01 {
            return;
        }
        let (t0, k) = (e.t, self.k);
        let c = p.col[k];
        match e.kind.as_str() {
            "hit" => {
                let (x, z) = (e.f(5), e.f(6));
                let pos = scene::to_world(x, z, R);
                match how(e) {
                    "bite" => {
                        let main = e.f(8) != 0.0;
                        fx.bu.add(t0, pos, c, if main { 0.9 } else { 0.5 }, None, main);
                        let peak = if main { 0.7 } else { 0.4 };
                        fx.fl
                            .add(t0, scene::to_world(x, z, 1.0), sc(c, 0.9), 1.5, peak, 0.03, 0.22, 0.0);
                        let puffs = Emit {
                            n: Some(if main { 6 } else { 3 }),
                            col: Some(vec![[250.0, 236.0, 248.0]]),
                            ..Emit::default()
                        };
                        fx.pa.emit(t0, pos, "puffs", &puffs);
                        let aim = self
                            .bites
                            .iter()
                            .find(|b| (b.t_snap - t0).abs() < 1e-4)
                            .map_or(0.0, |b| b.aim);
                        fx.mk.add(
                            t0,
                            Mark {
                                size: 0.40,
                                angle: aim + PI / 2.0,
                                n: 2,
                                col: PALE,
                                life: 3.0,
                                seed: (t0 * 100.0).max(0.0) as u64,
                                ..Mark::new(MarkKind::Claw, x, z)
                            },
                        );
                    }
                    "clamp" => {
                        fx.bu.add(t0, pos, c, 2.0, None, true);
                        fx.fl.add(t0, scene::to_world(x, z, 1.2), c, 2.6, 1.2, 0.03, 0.5, 0.0);
                        let puffs = Emit {
                            n: Some(24),
                            spread: Some(180.0),
                            speed: Some((1.5, 3.2)),
                            scale: 1.3,
                            ..Emit::default()
                        };
                        fx.pa.emit(t0, pos, "puffs", &puffs);
                        let seed = (t0 * 10.0).max(0.0) as u64;
                        fx.mk.add(
                            t0,
                            Mark {
                                size: 1.3,
                                n: 7,
                                life: 6.0,
                                seed,
                                ..Mark::new(MarkKind::Crack, x, z)
                            },
                        );
                        fx.mk.add(
                            t0,
                            Mark {
                                size: 1.0,
                                col: [120.0, 40.0, 100.0],
                                life: 6.0,
                                seed: seed + 1,
                                strength: 0.5,
                                ..Mark::new(MarkKind::Scorch, x, z)
                            },
                        );
                    }
                    _ => {}
                }
            }
            "snap" => {
                // every bite: a puff from the bellows as they close
                let q = p.run.p(p.sim_i(t0), k);
                let r = R + e.f(6) * 0.6;
                let tip = [q[0] + r * dmath::cos(e.f(5)), q[1] + r * dmath::sin(e.f(5))];
                let puffs = Emit {
                    n: Some(3),
                    ..Emit::default()
                };
                fx.pa
                    .emit(t0, scene::to_world(tip[0], tip[1], R * 0.9), "puffs", &puffs);
                fx.fl.add(
                    t0,
                    scene::to_world(tip[0], tip[1], 0.9),
                    sc(c, 0.8),
                    1.2,
                    0.35,
                    0.03,
                    0.16,
                    0.0,
                );
            }
            "clamp" => {
                fx.fl.add(
                    t0,
                    scene::to_world(e.f(5), e.f(6), 1.2),
                    sc(c, 0.9),
                    2.2,
                    0.7,
                    0.03,
                    0.6,
                    0.0,
                );
            }
            _ => {}
        }
    }

    /// The bellows tear at its K.O.: a burst of air.
    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let puffs = Emit {
            n: Some(30),
            spread: Some(180.0),
            speed: Some((1.5, 4.0)),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "puffs", &puffs);
    }

    // ------------------------------------------------------------ the open mouth lights the canvas under it
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !(p.show.grid.t_fight <= c.t && c.t < p.t_hs + 0.25) {
            return;
        }
        let tc = py_min(c.t, p.t_ko);
        let st = &c.fs[self.k];
        if !st.vis || st.shattered {
            return;
        }
        let tint = p.col[self.k].map(|v| v / 255.0 + 0.2);
        for b in self.bites_at(tc) {
            let Some(ob) = bite_open(tc, b.t0, b.t_snap) else {
                continue;
            };
            let a_ = self.half.to_radians() * 1.15 * py_max(ob.0, 0.15);
            let snapk = if tc >= b.t_snap - 0.06 {
                dmath::exp(-py_max(0.0, tc - b.t_snap) / 0.08)
            } else {
                0.35
            };
            let rmax = R + b.reach * ob.1;
            for (j, light) in ic.iter_mut().enumerate() {
                let (dx, dz) = (c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                let rd = np::hypot(dx, dz);
                if rd <= R || rd >= rmax {
                    continue;
                }
                let off = fmod_py(dmath::atan2(dz, dx) - b.aim + PI, 2.0 * PI) - PI;
                if off.abs() < a_ {
                    for ch in 0..3 {
                        light[ch] += 0.22 * snapk * tint[ch];
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ the painter: jaws, the clamp, the collar
    fn items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        if st.shield > 0.0 && st.vis && !st.shattered && self.shield_dur.is_some() && f.t < p.t_hs + 0.3 {
            let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
            let back: Payload = Box::new(Item::Collar(false));
            let front: Payload = Box::new(Item::Collar(true));
            return vec![(dz + 0.03, back), (dz - 0.03, front)];
        }
        Vec::new()
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let t = f.t;
        let tc = py_min(t, p.t_ko);
        let mut items: Vec<(f64, Payload)> = Vec::new();
        if p.show.grid.t_fight <= t && t < p.t_hs + 0.3 {
            for c in self.clamps_at(tc) {
                items.push((1e9, Box::new(Item::Clamp(*c))));
            }
        }
        let st = &f.fs[self.k];
        if p.show.grid.t_fight <= t && t < p.t_hs + 0.25 && st.vis && !st.shattered {
            for b in self.bites_at(tc) {
                if let Some(ob) = bite_open(tc, b.t0, b.t_snap) {
                    let r = R + b.reach * 0.5 * ob.1;
                    let mid = [st.p[0] + r * dmath::cos(b.aim), st.p[1] + r * dmath::sin(b.aim)];
                    let dz = f.cam.depth(&[scene::to_world(mid[0], mid[1], R)])[0];
                    items.push((dz, Box::new(Item::Jaws(*b, ob))));
                }
            }
        }
        items
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(item) = payload.downcast_ref::<Item>() else {
            return;
        };
        let col = p.col[self.k];
        let st = &f.fs[self.k];
        let (mut arr, depth) = (fr.array(), fr.depth());
        match item {
            Item::Jaws(b, ob) => {
                draw_jaws(&mut arr, f.cam, st.p, b.aim, self.half, b.reach, ob.0, ob.1, col);
            }
            Item::Clamp(c) => {
                let tt = if f.t < p.t_hs { py_min(f.t, p.t_ko) } else { f.t };
                if let Some((a, b, alpha)) = clamp_state(tt, c) {
                    draw_clamp(&mut arr, f.cam, a, b, c.axis, col, alpha);
                }
            }
            Item::Collar(front) => {
                let dur = self.shield_dur.unwrap_or(1.0);
                let k_ = py_min(py_min(1.0, st.shield / 0.2), (dur - st.shield) / 0.12 + 0.2);
                let close = config::ease(py_min(1.0, (dur - st.shield) / 0.28));
                let glow = p.show.grid.beat_pulse(f.t, 0.15, 0.07);
                draw_squeezebox(&mut arr, f.cam, st.p[0], st.p[1], f.t, k_, *front, col, close, glow);
            }
        }
        fr.set_array(&arr, Some(&depth));
    }

    /// #30's shards: a piano-style key, mother-of-pearl buttons, pleat chips.
    fn draw_shard(
        &self,
        p: &PictureCtx,
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
        let on_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            screen(cam, &world)
        };
        if kind == "key" {
            let (l, wk) = (0.30, 0.07);
            let q4: Vec<[f64; 2]> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(a1, a2)| {
                    [
                        sx + d[0] * l / 2.0 * a1 + nn[0] * wk / 2.0 * a2,
                        sz + d[1] * l / 2.0 * a1 + nn[1] * wk / 2.0 * a2,
                    ]
                })
                .collect();
            layer_blend(arr, &[on_screen(&q4)], [244.0, 240.0, 228.0], 1.0, None);
            return;
        }
        let col = if kind == "button" {
            [246.0, 240.0, 226.0]
        } else if (size * 100.0) as i64 % 2 != 0 {
            sc(p.col[p.lose], 0.9)
        } else {
            add3(PLEAT_DK, sc(p.col[p.lose], 0.2))
        };
        let s = size * if kind == "button" { 0.55 } else { 1.0 };
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
        layer_blend(arr, &[on_screen(&tri)], clip255(sc(col, shade_k)), 1.0, None);
    }
}
