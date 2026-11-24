//! SYNTH with the bold effects (`picture.effects = "bold"`; the owner's picks 28a to 28d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **PATCH CABLES** (28a): a big jack plug flies out on a thick cable, plugs in with a spark and the cable
//!   settles into a sag (it hangs down the screen, less the farther the enemy is). Each voltage pulse is a square
//!   bump running down the cable that ends in a small zap at the jack. When the leash holds, the cable snaps
//!   straight and white and speed lines yank the enemy back; an unplugged cable reels home with its plug.
//! - **LFO** (28b): a wobbling wave ring circles the ball with a bead running round it, the cables carry a fast
//!   wobble and every bump trails two ghosts (the ring's speed-up look, S6, adds the afterimages and the
//!   draining ring by itself).
//! - **SAWTOOTH** (28c): the plugged cables turn saw-toothed and thick saw arms lash from the ball over
//!   everything, each trailing two afterimages; every cut leaves a short white slash.
//! - **WHITE NOISE** (28d): static crackles thicker and wider round the ball, bursts out as a jagged ring with a
//!   band of static pixels riding it, and an enemy it hits flickers under static for a moment.

use bf_core::dmath::{self, PI, TAU};
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, INK, WHITE};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::Wave;
use bf_picture::scene::{self, A, R_BALL as R};

use super::{CableState, Item, SynthLook, lash_angle};

/// The dark metal of a jack's barrel.
const METAL: V3 = [58.0, 64.0, 84.0];
/// A zap's yellow-white.
const ZAP: V3 = [255.0, 246.0, 150.0];
/// Seconds a cable snapped straight by the leash stays white.
const TAUT_S: f64 = 0.16;
/// Seconds WHITE NOISE crackles before it bursts.
const CRACKLE_S: f64 = 0.45;
/// Seconds an enemy hit by WHITE NOISE flickers.
const FLICKER_S: f64 = 0.6;

/// What a bold cable carries.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    /// A plain cable.
    Smooth,
    /// LFO: a fast wobble.
    Wobble,
    /// SAWTOOTH: saw teeth.
    Saw,
}

/// A bold cable from `a` to `b` (ring frame): bowed sideways by `bow`, hanging by `droop` along `down` (the
/// ring direction that points down the screen), carrying its `shape`.
#[derive(Clone, Copy)]
struct Line {
    a: [f64; 2],
    b: [f64; 2],
    bow: f64,
    down: [f64; 2],
    droop: f64,
    shape: Shape,
    phase: f64,
}

impl Line {
    /// The length from end to end.
    fn len(&self) -> f64 {
        dmath::hypot(self.b[0] - self.a[0], self.b[1] - self.a[1]) + 1e-9
    }

    /// The ring-frame point at `s` (0 at the synth, 1 at the far end) and its height.
    fn at(&self, s: f64) -> ([f64; 2], f64) {
        let l = self.len();
        let d = [self.b[0] - self.a[0], self.b[1] - self.a[1]];
        let n = [-d[1] / l, d[0] / l];
        let arch = dmath::sin(PI * s.clamp(0.0, 1.0));
        let env = dmath::pow(arch.max(0.0), 0.6);
        let wave = match self.shape {
            Shape::Smooth => 0.0,
            Shape::Wobble => 0.08 * dmath::sin(TAU * (s * l / 0.5 - self.phase)),
            Shape::Saw => 0.13 * (2.0 * (s * l / 0.3 - self.phase).rem_euclid(1.0) - 1.0),
        };
        let lat = self.bow * arch + wave * env;
        (
            [
                self.a[0] + d[0] * s + n[0] * lat + self.down[0] * self.droop * arch,
                self.a[1] + d[1] * s + n[1] * lat + self.down[1] * self.droop * arch,
            ],
            0.44 - 0.2 * (self.droop / 0.5).min(1.0) * arch,
        )
    }

    /// The cable as a world polyline.
    fn points(&self) -> Vec<V3> {
        let m = ((self.len() / 0.05) as usize).clamp(10, 96);
        (0..=m)
            .map(|i| {
                let (q, y) = self.at(i as f64 / m as f64);
                scene::to_world(q[0], q[1], y)
            })
            .collect()
    }

    /// A square voltage bump centred at `s`: up off the cable, along it, and back down.
    fn bump(&self, s: f64, high: f64) -> Vec<V3> {
        let w = (0.13 / self.len()).min(0.2);
        let (s0, s1) = ((s - w).max(0.0), (s + w).min(1.0));
        let ((q0, y0), (q1, y1)) = (self.at(s0), self.at(s1));
        let up = [-self.down[0] * high, -self.down[1] * high];
        vec![
            scene::to_world(q0[0], q0[1], y0),
            scene::to_world(q0[0] + up[0], q0[1] + up[1], y0 + high * 0.4),
            scene::to_world(q1[0] + up[0], q1[1] + up[1], y1 + high * 0.4),
            scene::to_world(q1[0], q1[1], y1),
        ]
    }
}

/// The ring-frame unit direction that points straight down the screen at `m` (toward the camera): a cable that
/// hangs along it reads as sagging.
fn screen_down(cam: &Cam, m: [f64; 2]) -> [f64; 2] {
    let o = bold::at(cam, m[0], m[1], 0.42);
    let ex = bold::at(cam, m[0] + 0.1, m[1], 0.42);
    let ez = bold::at(cam, m[0], m[1] + 0.1, 0.42);
    let (a, b, c, d) = (ex.0 - o.0, ez.0 - o.0, ex.1 - o.1, ez.1 - o.1);
    let det = a * d - b * c;
    if det.abs() < 1e-9 {
        return [0.0, 0.0];
    }
    let g = [-b / det, a / det];
    let n = dmath::hypot(g[0], g[1]).max(1e-9);
    [g[0] / n, g[1] / n]
}

/// A big jack plug at `c` pointing along `ang`: its barrel and its tip.
fn big_jack(c: [f64; 2], ang: f64) -> ([V3; 4], [V3; 4]) {
    let (y, l, w) = (0.46, 0.42, 0.12);
    let d = [dmath::cos(ang), dmath::sin(ang)];
    let n = [-d[1], d[0]];
    let at = |k: f64, m: f64| {
        scene::to_world(
            c[0] + d[0] * l * k + n[0] * w * m,
            c[1] + d[1] * l * k + n[1] * w * m,
            y,
        )
    };
    (
        [at(-0.6, -1.0), at(0.1, -1.0), at(0.1, 1.0), at(-0.6, 1.0)],
        [at(0.1, -0.5), at(0.55, -0.32), at(0.55, 0.32), at(0.1, 0.5)],
    )
}

/// A saw arm out of the ball at `p` along `ang`, `length` from the centre: bigger teeth than the classic one.
fn saw_arm(p: [f64; 2], ang: f64, length: f64) -> Vec<V3> {
    let (teeth, amp, y, r0) = (6, 0.2, 0.5, 0.62);
    let d = [dmath::cos(ang), dmath::sin(ang)];
    let n = [-d[1], d[0]];
    let at = |r: f64, side: f64| {
        scene::to_world(
            p[0] + d[0] * r + n[0] * side * amp,
            p[1] + d[1] * r + n[1] * side * amp,
            y,
        )
    };
    let mut out = Vec::with_capacity(3 * teeth);
    for j in 0..teeth {
        let a = r0 + (length - r0) * j as f64 / teeth as f64;
        let b = r0 + (length - r0) * (j + 1) as f64 / teeth as f64;
        out.extend([at(a, -1.0), at(b, 1.0), at(b, -1.0)]);
    }
    out
}

/// What the bold look draws of the cables in a frame.
#[derive(Default)]
struct Scene {
    /// The cables: the polyline, how white it is (a leash just held) and whether it is a thin, fading one.
    cables: Vec<(Vec<V3>, f64, bool)>,
    /// The voltage bumps: the polyline and whether it is a ghost.
    bumps: Vec<(Vec<V3>, bool)>,
    /// The jacks: where and which way.
    jacks: Vec<([f64; 2], f64)>,
    /// SAWTOOTH's arms: the polyline, its tip and how old an afterimage it is (0: the arm itself).
    arms: Vec<(Vec<V3>, [f64; 2], u32)>,
    /// Where the glows go (world points).
    glows: Vec<V3>,
}

/// One static pixel: a crisp square of side `w` with its corner at `q`.
fn block(img: &mut Pic, q: (f64, f64), w: f64, col: V3, a: f64) {
    let (x, y) = (q.0.floor(), q.1.floor());
    px::poly(img, &[(x, y), (x + w, y), (x + w, y + w), (x, y + w)], col, a);
}

/// A handful of static pixels round the screen point `c`: `n` small marks within `r` pixels (squashed to the
/// floor's tilt by `flat`), white, cyan and dark, new every `seed`.
#[allow(clippy::too_many_arguments)]
fn static_px(img: &mut Pic, c: (f64, f64), r: f64, flat: f64, n: u64, seed: u64, col: V3, a: f64) {
    for i in 0..n {
        let ang = px::rnd(seed, i) * TAU;
        let far = r * dmath::pow(px::rnd(seed, i + 500), 0.5);
        let q = (c.0 + dmath::cos(ang) * far, c.1 + dmath::sin(ang) * far * flat);
        let w = if px::rnd(seed, i + 900) > 0.6 { 4.0 } else { 3.0 };
        let ink = match i % 4 {
            0 => INK,
            1 => col,
            _ => WHITE,
        };
        block(img, q, w, ink, a);
    }
}

impl SynthLook {
    /// The leash's length (the roster's `weapon.length`).
    fn leash(p: &PictureCtx, k: usize) -> f64 {
        p.cfg[k].raw["weapon"]["length"].as_f64().unwrap_or(2.4)
    }

    /// The frame's cables, bumps, jacks and arms.
    fn bold_scene(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Scene {
        let (k, t, fs) = (self.k, f.t, f.fs);
        let mut out = Scene::default();
        if !(p.show.grid.t_fight <= t && t < p.t_hs + 0.3) || fs[k].shattered || !fs[k].vis {
            return out;
        }
        let (ps, pe) = (fs[k].p, fs[1 - k].p);
        let d = [pe[0] - ps[0], pe[1] - ps[1]];
        let dist = dmath::hypot(d[0], d[1]) + 1e-9;
        let u = [d[0] / dist, d[1] / dist];
        let tc = p.fx_clock(t);
        let a_s = [ps[0] + u[0] * R * 0.95, ps[1] + u[1] * R * 0.95];
        let a_e = [pe[0] - u[0] * R * 0.92, pe[1] - u[1] * R * 0.92];
        let mid = [(a_s[0] + a_e[0]) / 2.0, (a_s[1] + a_e[1]) / 2.0];
        let down = screen_down(f.cam, mid);
        let lashing = self.lashes.iter().any(|l| l.t0 <= tc && tc < l.t1);
        let hasted = fs[k].haste > 0.0;
        let shape = if lashing {
            Shape::Saw
        } else if hasted {
            Shape::Wobble
        } else {
            Shape::Smooth
        };
        let phase = tc * if hasted { 7.0 } else { 3.2 };
        // how slack the cables are, and a leash that has just held them straight
        let leash = Self::leash(p, k);
        let slack = (1.0 - (dist - 2.0 * R) / (leash - 2.0 * R).max(1e-6)).clamp(0.0, 1.0);
        let taut = bold::since(p, k, &["leash"], tc, TAUT_S).map_or(0.0, |(age, _)| 1.0 - age / TAUT_S);
        let cab = self.live_cables(p, t);
        let ids_in: Vec<i64> = cab.iter().filter(|c| c.1 == CableState::In).map(|c| c.0).collect();
        let n_in = ids_in.len() as f64;
        let line_of = |j_in: usize, cid: i64| {
            // it settles into its sag after it plugs in, with a bounce
            let age = self.cables.get(&cid).map_or(1.0, |c| tc - c.t0).max(0.0);
            let settle = 1.0 - dmath::exp(-age / 0.07) * dmath::cos(age * 28.0);
            Line {
                a: a_s,
                b: a_e,
                bow: (j_in as f64 - (n_in - 1.0) / 2.0) * 0.26,
                down,
                droop: (0.12 + 0.42 * slack.sqrt()) * settle * (1.0 - taut),
                shape,
                phase: phase + cid as f64 * 0.37,
            }
        };
        let plain = |a: [f64; 2], b: [f64; 2], droop: f64| Line {
            a,
            b,
            bow: 0.0,
            down,
            droop,
            shape: Shape::Smooth,
            phase: 0.0,
        };
        let aim = dmath::atan2(d[1], d[0]);
        let mut j_in = 0usize;
        for &(cid, state, kk) in &cab {
            match state {
                CableState::In => {
                    let line = line_of(j_in, cid);
                    j_in += 1;
                    out.cables.push((line.points(), taut, false));
                    out.jacks.push((a_e, aim));
                }
                CableState::Retract => {
                    // it reels home with its plug
                    let ek = bold::ease_in(kk);
                    let e_ = [a_e[0] + (a_s[0] - a_e[0]) * ek, a_e[1] + (a_s[1] - a_e[1]) * ek];
                    out.cables
                        .push((plain(a_s, e_, 0.15 * (1.0 - kk)).points(), 0.0, false));
                    out.jacks.push((e_, aim));
                }
                CableState::Snap => {
                    // snapped: both halves whip back
                    let ek = bold::ease_out(kk);
                    let m1 = [mid[0] + (a_s[0] - mid[0]) * ek, mid[1] + (a_s[1] - mid[1]) * ek];
                    let m2 = [mid[0] + (a_e[0] - mid[0]) * ek, mid[1] + (a_e[1] - mid[1]) * ek];
                    for (a, b) in [(a_s, m1), (m2, a_e)] {
                        let mut half = plain(a, b, 0.0);
                        half.shape = Shape::Wobble;
                        half.phase = tc * 9.0;
                        out.cables.push((half.points(), 1.0 - kk, kk > 0.5));
                    }
                }
            }
        }
        // plugs in flight, trailing their cable from the synth
        for (_, pl) in &self.plugs {
            let t1 = pl.t1.unwrap_or(p.t_ko);
            if pl.t0 <= tc && tc < t1 {
                let v = [dmath::cos(pl.ang), dmath::sin(pl.ang)];
                let q = [
                    pl.o[0] + v[0] * pl.speed * (tc - pl.t0),
                    pl.o[1] + v[1] * pl.speed * (tc - pl.t0),
                ];
                let back = [q[0] - v[0] * 0.2, q[1] - v[1] * 0.2];
                out.cables.push((plain(a_s, back, 0.06).points(), 0.0, false));
                out.jacks.push((q, pl.ang));
            }
        }
        // voltage pulses: square bumps running down the plugged cables; under LFO each trails two ghosts
        if !self.volt_t.is_empty() {
            let j0 = self.volt_t.partition_point(|&v| v < tc - 0.6);
            let j1 = self.volt_t.partition_point(|&v| v <= tc);
            for v in self.volts.get(j0..j1.max(j0)).unwrap_or(&[]) {
                let uu = (tc - v.t0) / v.tr.max(1e-3);
                let Some(jj) = ids_in.iter().position(|&c| c == v.cid) else {
                    continue;
                };
                if !(0.0..=1.0).contains(&uu) {
                    continue;
                }
                let line = line_of(jj, v.cid);
                if hasted {
                    for g in [2.0, 1.0] {
                        let s = uu - 0.09 * g;
                        if s > 0.0 {
                            out.bumps.push((line.bump(s, 0.16), true));
                        }
                    }
                }
                out.bumps.push((line.bump(uu, 0.22), false));
                let (q, y) = line.at(uu);
                out.glows.push(scene::to_world(q[0], q[1], y + 0.1));
            }
        }
        // SAWTOOTH: saw arms lash from the ball, each trailing two afterimages
        for l in &self.lashes {
            if l.t0 <= tc && tc < l.t1 {
                let reach = R + l.len;
                for j in 0..l.n {
                    for ghost in [2u32, 1, 0] {
                        let tg = tc - 0.04 * f64::from(ghost);
                        if tg < l.t0 {
                            continue;
                        }
                        let a = lash_angle(l, j, tg);
                        let tip = [ps[0] + reach * dmath::cos(a), ps[1] + reach * dmath::sin(a)];
                        out.arms.push((saw_arm(ps, a, reach), tip, ghost));
                        if ghost == 0 {
                            out.glows.push(scene::to_world(tip[0], tip[1], 0.5));
                        }
                    }
                }
            }
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let col = p.col[self.k];
        let scene_now = self.bold_scene(p, f);
        match payload.downcast_ref::<Item>() {
            Some(Item::Cables) => {
                for (pts, white, thin) in &scene_now.cables {
                    if *thin {
                        bold::thin_line(fr, f.cam, pts, col);
                        continue;
                    }
                    bold::thick_line(fr, f.cam, pts, col, 3);
                    if *white > 0.0 {
                        // snapped straight: it flashes white
                        fr.line(f.cam, pts, WHITE, if *white > 0.5 { 3 } else { 1 });
                    }
                }
                for (pts, ghost) in &scene_now.bumps {
                    if *ghost {
                        fr.line(f.cam, pts, bold::pale(col), 1);
                    } else {
                        fr.line(f.cam, pts, INK, 5);
                        fr.line(f.cam, pts, WHITE, 3);
                    }
                }
            }
            Some(Item::Jacks) => {
                for (pts, tip, ghost) in &scene_now.arms {
                    match ghost {
                        0 => {
                            bold::thick_line(fr, f.cam, pts, bold::pale(col), 3);
                            bold::blob(fr, f.cam, scene::to_world(tip[0], tip[1], 0.5), 0.1, WHITE);
                        }
                        1 => fr.line(f.cam, pts, col, 2),
                        _ => fr.line(f.cam, pts, bold::deep(col).map(|v| v + 30.0), 1),
                    }
                }
                for &(q, ang) in &scene_now.jacks {
                    let (barrel, tip) = big_jack(q, ang);
                    bold::outlined_poly(fr, f.cam, &barrel, METAL);
                    bold::outlined_poly(fr, f.cam, &tip, bold::pale(col));
                    // the collar between them
                    fr.line(f.cam, &[barrel[1], barrel[2]], WHITE, 1);
                }
            }
            None => {}
        }
    }

    pub(super) fn bold_glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame) {
        let col = p.col[self.k].map(|c| c * 0.6 + 100.0);
        for w in self.bold_scene(p, f).glows {
            let s = f.cam.wp(&[w])[0];
            fr.glow(s.0, s.1, 7.0, col, 0.55);
        }
    }

    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        // WHITE NOISE: a jagged ring with static sprinkled over its band
        let tint = bold::light(p.col[w.owner]);
        floor::wave(ic, c, Family::Noise, w, c.t, tint, 1.3);
        if w.kind != "wave" {
            return;
        }
        let k_ = 1.0 - w.r / w.rmax.max(1e-6);
        let tf = (c.t * 30.0).floor() * 1.7;
        for (j, light) in ic.iter_mut().enumerate() {
            let (sx, sz) = (c.sxp[j], c.szp[j]);
            let rd = dmath::hypot(sx - w.cx, sz - w.cz);
            if (rd - w.r).abs() > 0.6 {
                continue;
            }
            let h = ((sx * 17.0).floor() * 7.3 + (sz * 17.0).floor() * 3.1 + tf).rem_euclid(1.0);
            if h > 0.66 {
                for v in light.iter_mut() {
                    *v += 1.1 * k_;
                }
            } else if h < 0.16 {
                for v in light.iter_mut() {
                    *v *= 1.0 - 0.7 * k_;
                }
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight || f.t > p.t_ko {
            return;
        }
        let k = self.k;
        let o = 1 - k;
        let t = f.t;
        let col = p.col[k];
        let pale = bold::pale(col);
        let (ps, pe) = (f.fs[k].p, f.fs[o].p);
        let d = [pe[0] - ps[0], pe[1] - ps[1]];
        let dist = dmath::hypot(d[0], d[1]) + 1e-9;
        let u = [d[0] / dist, d[1] / dist];
        // where the cables meet the enemy
        let socket = bold::at(f.cam, pe[0] - u[0] * R * 0.92, pe[1] - u[1] * R * 0.92, 0.46);
        let mine = drawn.iter().find(|b| b.k == k);
        let foe = drawn.iter().find(|b| b.k == o);
        let frame = f.f as u64;
        // a plug goes in: a spark at the socket
        for e in bold::events_in(p, k, &["plug"], t - 0.22, t) {
            let age = t - e.t;
            let v = age / 0.22;
            px::ink_star(img, socket, 5.0 + 9.0 * bold::ease_out(v), 3.0, 4, e.t, WHITE, 1.0 - v);
            px::chips(img, socket, age, 0.22, 7, 20.0, (e.t * 1000.0) as u64, col);
        }
        // a pulse arrives: a small zap at the jack
        for e in bold::events_in(p, k, &["hit:volt"], t - 0.09, t) {
            let seed = (e.t * 1000.0) as u64;
            let mut q = socket;
            let mut bolt = vec![q];
            for i in 0..3u64 {
                q = (
                    q.0 + (px::rnd(seed, i) - 0.5) * 12.0 + u[0] * 4.0,
                    q.1 + (px::rnd(seed, i + 9) - 0.5) * 12.0 - 2.0,
                );
                bolt.push(q);
            }
            px::ink_polyline(img, &bolt, 1.4, ZAP, 1.0 - (t - e.t) / 0.09);
        }
        // the leash holds: speed lines yank the enemy back toward the synth
        if let Some(b) = foe
            && let Some((age, _)) = bold::since(p, k, &["leash"], t, 0.2)
            && let Some(m) = mine
        {
            let a = 1.0 - age / 0.2;
            px::speed_lines(img, b.s, b.rp, (m.s.0 - b.s.0, m.s.1 - b.s.1), 16.0, 6, WHITE, a);
            px::ink_ring(img, socket, 4.0 + 8.0 * (1.0 - a), 1.4, WHITE, a);
        }
        // a cable is unplugged: a little pop where it was in
        for e in bold::events_in(p, k, &["unplug"], t - 0.16, t) {
            if e.i(4) == 0 {
                let v = (t - e.t) / 0.16;
                px::ink_ring(img, socket, 3.0 + 9.0 * bold::ease_out(v), 1.3, pale, 1.0 - v);
            }
        }
        if let Some(b) = mine {
            // LFO: a wobbling wave ring circles the ball, a bead running round it
            if f.fs[k].haste > 0.0 && !f.fs[k].shattered {
                let wobble = |ang: f64| b.rp + 15.0 + 3.5 * dmath::sin(7.0 * ang - t * 16.0);
                let ring: Vec<(f64, f64)> = (0..=56)
                    .map(|i| {
                        let ang = TAU * f64::from(i) / 56.0;
                        let r = wobble(ang);
                        (b.s.0 + dmath::cos(ang) * r, b.s.1 + dmath::sin(ang) * r)
                    })
                    .collect();
                px::ink_polyline(img, &ring, 1.6, pale, 1.0);
                let ang = t * 9.0;
                let r = wobble(ang);
                px::ink_disc(
                    img,
                    (b.s.0 + dmath::cos(ang) * r, b.s.1 + dmath::sin(ang) * r),
                    2.6,
                    WHITE,
                    1.0,
                );
            }
            // WHITE NOISE: a crackle builds round the ball
            if let Some((v, _)) = bold::until(p, k, &["ability:wave"], t, CRACKLE_S) {
                let n = (12.0 + 80.0 * v) as u64;
                static_px(img, b.s, b.rp * (1.1 + 1.2 * v), 0.85, n, frame, col, 1.0);
            }
        }
        // SAWTOOTH: every cut leaves a short white slash
        for e in bold::events_in(p, k, &["hit:lash"], t - 0.18, t) {
            let v = (t - e.t) / 0.18;
            let c = bold::at(f.cam, e.f(5), e.f(6), R);
            let tilt = if (e.t * 1000.0) as i64 % 2 == 0 { 1.0 } else { -1.0 };
            let half = 5.0 + 9.0 * bold::ease_out(v / 0.4);
            px::ink_line(
                img,
                (c.0 - half, c.1 - half * tilt),
                (c.0 + half, c.1 + half * tilt),
                2.2,
                WHITE,
                1.0 - bold::ease_in(v),
            );
        }
        // WHITE NOISE: a band of static pixels rides the ring out
        let low = &p.cfg[k].low;
        let (speed, radius) = (low.get("speed").unwrap_or(4.5), low.get("radius").unwrap_or(9.0));
        if let Some((age, e)) = bold::since(p, k, &["ability:wave"], t, (radius - R) / speed.max(1e-6)) {
            let c0 = p.run.p(p.sim_i(e.t), k);
            let r = R + speed * age;
            let fade = (1.0 - r / radius).clamp(0.0, 1.0);
            let a = (fade * 3.0).min(1.0);
            for i in 0..190u64 {
                let ang = px::rnd(frame, i) * TAU;
                let rr = r + (px::rnd(frame, i + 300) - 0.6) * 0.8;
                let (x, z) = (c0[0] + rr * dmath::cos(ang), c0[1] + rr * dmath::sin(ang));
                if x.abs() > A || z.abs() > A {
                    continue;
                }
                let q = bold::at(f.cam, x, z, 0.05);
                let w = if i % 3 == 0 { 5.0 } else { 3.0 };
                let ink = match i % 5 {
                    0 => INK,
                    1 | 2 => pale,
                    _ => WHITE,
                };
                block(img, q, w, ink, a);
            }
        }
        // the enemy it hits flickers under static
        if let Some(b) = foe
            && let Some((age, _)) = bold::since(p, k, &["hit:wave"], t, FLICKER_S)
        {
            let a = 1.0 - bold::ease_in(age / FLICKER_S);
            if frame.is_multiple_of(2) {
                px::disc(img, b.s, b.rp, WHITE, 0.55 * a);
            } else {
                px::disc(img, b.s, b.rp, INK, 0.45 * a);
            }
            static_px(img, b.s, b.rp, 1.0, 70, frame, col, a);
            // torn scan lines across it
            for i in 0..3u64 {
                let y = b.s.1 + (px::rnd(frame, 700 + i) - 0.5) * 1.6 * b.rp;
                let shift = (px::rnd(frame, 710 + i) - 0.5) * 10.0;
                px::line(
                    img,
                    (b.s.0 - b.rp + shift, y),
                    (b.s.0 + b.rp + shift, y),
                    3.0,
                    if i == 0 { WHITE } else { pale },
                    0.9 * a,
                );
            }
        }
    }

    /// A plug going in is a solid hit and the ring a heavy one; a lash cuts solidly; the pulses do not pop
    /// (each ends in this look's own small zap).
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match bold::how(e) {
            "volt" => None,
            "plug" | "lash" => Some(by_damage.max(HitSize::Solid)),
            "wave" => Some(HitSize::Heavy),
            _ => Some(by_damage),
        }
    }
}
