//! ACCORDION with the bold effects (`picture.effects = "bold"`; the owner's picks 21a to 21d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **Bellows bite** (21a): the two halves (the keyboard and the button board) fly from the ball to the two
//!   corners of the mouth with a zig-zag bellows stretched between them; they quiver, arrows point inward and
//!   the floor between them lights; they snap shut on the enemy (which squashes: a main bite pops as a heavy
//!   hit), a star bursts at each half and air jets out both ways; the shut pose is held, then they spring back.
//! - **POLKA** (21b): notes hop round the ball in time while it lasts (the bite on every beat and the draining
//!   ring are the fight's and the ring's own).
//! - **SQUEEZEBOX** (21c): a big pleated collar of two rings folds round the ball; a blocked hit squeezes the
//!   pleats and puffs of air shoot out all round; the pleats flatten and the collar sinks as the shield runs
//!   down.
//! - **FULL BELLOWS** (21d): the halves, twice the size, fly to the ropes at both ends of the line through the
//!   enemy with the bellows spanning the ring; they quiver under inward arrows, slam together on the enemy
//!   with two big stars, and air blasts out both sides.

use bf_core::dmath::{self, TAU};
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::BallState;
use bf_picture::scene::{self, R_BALL as R};

use super::{AccordionLook, BOARD, BUTTON, Bite, Clamp, PLEAT_DK, how, lerp, rope_dist};

/// Seconds the jaws take to snap shut, stay shut (a landed bite is held three frames more) and spring back.
const SNAP_S: f64 = 0.05;
const SHUT_S: f64 = 0.04;
const BACK_S: f64 = 0.1;
/// Seconds the stars and the air of a snap show.
const CHOMP_S: f64 = 0.24;
/// FULL BELLOWS: seconds it takes to slam, stays shut and takes to come home; seconds its blast shows.
const SLAM_S: f64 = 0.1;
const CLAMPED_S: f64 = 0.35;
const HOME_S: f64 = 0.3;
const BLAST_S: f64 = 0.45;
/// Seconds SQUEEZEBOX takes to fold round the ball, and a blocked hit squeezes it.
const FOLD_S: f64 = 0.25;
const SQUEEZE_S: f64 = 0.25;

/// A puff of air.
const AIR: V3 = [250.0, 238.0, 250.0];

/// What the two halves are doing.
#[derive(Clone, Copy, PartialEq)]
enum Doing {
    /// Flying out.
    Out,
    /// In place and quivering; how far the wind-up is, 0 to 1.
    Quiver(f64),
    /// Snapping shut.
    Snap,
    /// Shut.
    Shut,
    /// Springing back to the ball.
    Back,
}

/// The two halves at one moment.
#[derive(Clone, Copy)]
struct Jaws {
    /// The keyboard half and the button half (ring frame).
    a: [f64; 2],
    b: [f64; 2],
    /// Their size, 1 full.
    size: f64,
    doing: Doing,
}

/// One painter item of the bold look.
enum BItem {
    /// FULL BELLOWS's bellows between the halves: its ends, its half width and the number of its pleats.
    Bellows([f64; 2], [f64; 2], f64, usize),
    /// A bite's bellows, fanned from the ball between the halves: the ball and the two halves.
    Fan([f64; 2], [f64; 2], [f64; 2]),
    /// A half: where, the unit vector toward the other half, its size and whether it is the keyboard.
    Half([f64; 2], [f64; 2], f64, bool),
    /// SQUEEZEBOX's collar: the half behind the ball or the half in front.
    Collar(bool),
}

fn unit(d: [f64; 2]) -> [f64; 2] {
    let n = dmath::hypot(d[0], d[1]).max(1e-9);
    [d[0] / n, d[1] / n]
}

fn off(a: [f64; 2], d: [f64; 2], s: f64) -> [f64; 2] {
    [a[0] + d[0] * s, a[1] + d[1] * s]
}

/// A box on the floor: its four sides back to front, then its top, each outlined.
#[allow(clippy::too_many_arguments)]
fn draw_box(
    fr: &mut Frame,
    cam: &Cam,
    c: [f64; 2],
    u: [f64; 2],
    v: [f64; 2],
    a: f64,
    b: f64,
    y0: f64,
    y1: f64,
    side: V3,
    top: V3,
) {
    let corner = |i: f64, j: f64, y: f64| {
        scene::to_world(
            c[0] + u[0] * a * i + v[0] * b * j,
            c[1] + u[1] * a * i + v[1] * b * j,
            y,
        )
    };
    let ring = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let mut sides: Vec<(f64, [V3; 4])> = (0..4)
        .map(|e| {
            let (p, q) = (ring[e], ring[(e + 1) % 4]);
            let quad = [
                corner(p.0, p.1, y0),
                corner(q.0, q.1, y0),
                corner(q.0, q.1, y1),
                corner(p.0, p.1, y1),
            ];
            let d = cam.depth(&quad);
            ((d[0] + d[1]) / 2.0, quad)
        })
        .collect();
    sides.sort_by(|p, q| q.0.total_cmp(&p.0));
    for (_, quad) in &sides {
        bold::outlined_poly(fr, cam, quad, side);
    }
    bold::outlined_poly(fr, cam, &ring.map(|(i, j)| corner(i, j, y1)), top);
}

/// One half of the accordion standing on the floor at `c`, facing the other half along `face`.
fn draw_half(fr: &mut Frame, cam: &Cam, c: [f64; 2], face: [f64; 2], size: f64, keys: bool, col: V3) {
    let long = [-face[1], face[0]];
    let (hl, hw) = (0.38 * size, 0.12 * size);
    let (y0, y1) = (0.1, 0.1 + 0.66 * size);
    let top = if keys { BUTTON } else { bold::pale(col) };
    draw_box(fr, cam, c, long, face, hl, hw, y0, y1, BOARD.map(|v| v + 26.0), top);
    // the keys, or the rows of buttons, on the top
    for i in 0..5 {
        let s = (f64::from(i) - 2.0) * hl * 0.4;
        let q = off(c, long, s);
        if keys {
            let (p0, p1) = (off(q, face, -hw * 0.8), off(q, face, hw * 0.2));
            fr.line(
                cam,
                &[
                    scene::to_world(p0[0], p0[1], y1 + 0.01),
                    scene::to_world(p1[0], p1[1], y1 + 0.01),
                ],
                bold::INK,
                1,
            );
        } else if i % 2 == 0 {
            bold::matte_blob(fr, cam, scene::to_world(q[0], q[1], y1 + 0.01), 0.04 * size, BUTTON);
        }
    }
    // a trim in the fighter's colour along the inner edge
    let (e0, e1) = (off(off(c, face, hw), long, -hl), off(off(c, face, hw), long, hl));
    fr.line(
        cam,
        &[
            scene::to_world(e0[0], e0[1], y1 + 0.01),
            scene::to_world(e1[0], e1[1], y1 + 0.01),
        ],
        col,
        2,
    );
}

/// The bellows from `a` to `b` seen from above: `n` pleats, wide and narrow in turn, light and dark in turn.
#[allow(clippy::too_many_arguments)]
fn draw_bellows(fr: &mut Frame, cam: &Cam, a: [f64; 2], b: [f64; 2], half_w: f64, n: usize, y: f64, col: V3) {
    let len = dmath::hypot(b[0] - a[0], b[1] - a[1]);
    if len < 0.06 || n == 0 {
        return;
    }
    let ax = unit([b[0] - a[0], b[1] - a[1]]);
    let nr = [-ax[1], ax[0]];
    let light = col.map(|c| (c * 1.05 + 22.0).min(255.0));
    let dark: V3 = std::array::from_fn(|ch| PLEAT_DK[ch] + col[ch] * 0.22);
    let edge = |i: usize, side: f64| {
        let c = off(a, ax, len * i as f64 / n as f64);
        let w = half_w * if i % 2 == 1 { 1.0 } else { 0.7 };
        let q = off(c, nr, side * w);
        scene::to_world(q[0], q[1], y)
    };
    for i in 0..n {
        let quad = [edge(i, -1.0), edge(i + 1, -1.0), edge(i + 1, 1.0), edge(i, 1.0)];
        bold::outlined_poly(fr, cam, &quad, if i % 2 == 0 { light } else { dark });
    }
}

/// A bite's bellows: a pleated fan from the ball at `me`, stretched between the halves at `a` and `b`, its rim
/// a zig-zag.
fn draw_fan(fr: &mut Frame, cam: &Cam, me: [f64; 2], a: [f64; 2], b: [f64; 2], y: f64, col: V3) {
    let (ra, rb) = (
        dmath::hypot(a[0] - me[0], a[1] - me[1]),
        dmath::hypot(b[0] - me[0], b[1] - me[1]),
    );
    if ra.max(rb) < R * 1.1 {
        return;
    }
    let a0 = dmath::atan2(a[1] - me[1], a[0] - me[0]);
    let mut da = dmath::atan2(b[1] - me[1], b[0] - me[0]) - a0;
    da -= TAU * (da / TAU).round();
    let n = 10;
    let light = col.map(|c| (c * 1.05 + 22.0).min(255.0));
    let dark: V3 = std::array::from_fn(|ch| PLEAT_DK[ch] + col[ch] * 0.22);
    let at = |i: usize, inner: bool| {
        let s = i as f64 / n as f64;
        let ang = a0 + da * s;
        let r = if inner {
            R * 0.9
        } else {
            (ra + (rb - ra) * s) * if i % 2 == 1 { 0.86 } else { 1.0 }
        };
        scene::to_world(me[0] + r * dmath::cos(ang), me[1] + r * dmath::sin(ang), y)
    };
    for i in 0..n {
        let quad = [at(i, true), at(i, false), at(i + 1, false), at(i + 1, true)];
        bold::outlined_poly(fr, cam, &quad, if i % 2 == 0 { light } else { dark });
    }
}

/// A hopping note, half as big again as [`px::note`]: a head, a stem and a flag.
fn big_note(img: &mut Pic, c: (f64, f64), col: V3) {
    let top = (c.0 + 3.0, c.1 - 11.0);
    px::ink_line(img, (c.0 + 3.0, c.1), top, 1.6, col, 1.0);
    px::ink_line(img, top, (top.0 + 5.0, top.1 + 3.5), 2.0, col, 1.0);
    px::ink_disc(img, c, 3.6, col, 1.0);
}

/// Two arrows on the screen pointing inward at the screen points `sa` and `sb` from outside them.
fn arrows_in(img: &mut Pic, sa: (f64, f64), sb: (f64, f64), gap: f64, len: f64, t: f64, a: f64) {
    let d = (sb.0 - sa.0, sb.1 - sa.1);
    let n = (d.0 * d.0 + d.1 * d.1).sqrt().max(1e-6);
    let (ux, uy) = (d.0 / n, d.1 / n);
    let bob = 3.0 * dmath::sin(t * 26.0);
    for (s, sign) in [(sa, -1.0), (sb, 1.0)] {
        let tip = (s.0 + sign * ux * (gap + bob), s.1 + sign * uy * (gap + bob));
        let tail = (tip.0 + sign * ux * len, tip.1 + sign * uy * len);
        px::ink_arrow(img, tail, tip, 2.4, 7.0, bold::WHITE, a);
    }
}

/// Air shooting from the screen point `from` toward `to` (and past it): three puffs and a streak, `u` from 0
/// to 1 through its life.
fn jet(img: &mut Pic, from: (f64, f64), to: (f64, f64), big: f64, u: f64) {
    let out = bold::ease_out(u);
    let at = |s: f64| (from.0 + (to.0 - from.0) * s, from.1 + (to.1 - from.1) * s);
    let a = ((1.0 - u) / 0.5).min(1.0);
    px::ink_line(img, at(0.4 + 0.2 * out), at(0.6 + 0.5 * out), 2.0 * big, bold::WHITE, a);
    for i in 0..3 {
        let s = 0.42 + (0.2 + 0.24 * f64::from(i)) * out;
        let r = big * (6.5 - 1.3 * f64::from(i)) * (1.0 - 0.5 * u);
        px::ink_disc(img, at(s), r, AIR, a);
    }
}

impl AccordionLook {
    /// The jaws' true half-angle (radians).
    fn half_rad(&self) -> f64 {
        self.half.to_radians()
    }

    /// Where the halves of bite `b` wait open, for a ball at `me`.
    fn open_spots(&self, b: &Bite, me: [f64; 2]) -> ([f64; 2], [f64; 2]) {
        let (a0, a1) = (b.aim - self.half_rad(), b.aim + self.half_rad());
        let r = R + b.reach;
        (
            off(me, [dmath::cos(a0), dmath::sin(a0)], r),
            off(me, [dmath::cos(a1), dmath::sin(a1)], r),
        )
    }

    /// Where the halves of bite `b` meet: either side of the enemy when it landed, else in the middle of the
    /// mouth. Also the point between them and the unit vector from the ball through it.
    fn shut_spots(&self, p: &PictureCtx, b: &Bite) -> ([f64; 2], [f64; 2], [f64; 2], [f64; 2]) {
        let i = p.sim_i(b.t_snap);
        let me = p.run.p(i, self.k);
        if b.landed {
            let foe = p.run.p(i, 1 - self.k);
            let d = unit([foe[0] - me[0], foe[1] - me[1]]);
            let n = [-d[1], d[0]];
            (off(foe, n, -R * 0.8), off(foe, n, R * 0.8), foe, d)
        } else {
            let d = [dmath::cos(b.aim), dmath::sin(b.aim)];
            let n = [-d[1], d[0]];
            let mid = off(me, d, R + b.reach * 0.75);
            (off(mid, n, -0.13), off(mid, n, 0.13), mid, d)
        }
    }

    /// The halves of bite `b` at `t` (a time already held on contact), for a ball at `me`.
    fn jaws(&self, p: &PictureCtx, b: &Bite, t: f64, me: [f64; 2]) -> Option<Jaws> {
        if t < b.t0 || t > b.t_snap + SHUT_S + BACK_S {
            return None;
        }
        let (oa, ob) = self.open_spots(b, me);
        let (ca, cb, _, _) = self.shut_spots(p, b);
        let t_close = b.t_snap - SNAP_S;
        Some(if t < t_close {
            let u = (t - b.t0) / (t_close - b.t0).max(1e-6);
            let k = bold::ease_out((u * 2.2).min(1.0));
            if u * 2.2 < 1.0 {
                Jaws {
                    a: lerp(me, oa, k),
                    b: lerp(me, ob, k),
                    size: 0.45 + 0.55 * k,
                    doing: Doing::Out,
                }
            } else {
                // in place: they quiver along the line between them
                let ax = unit([ob[0] - oa[0], ob[1] - oa[1]]);
                let q = 0.04 * dmath::sin(t * 85.0);
                Jaws {
                    a: off(oa, ax, q),
                    b: off(ob, ax, -q),
                    size: 1.0,
                    doing: Doing::Quiver((u * 2.2 - 1.0) / 1.2),
                }
            }
        } else if t < b.t_snap {
            let v = (t - t_close) / SNAP_S;
            Jaws {
                a: lerp(oa, ca, v * v),
                b: lerp(ob, cb, v * v),
                size: 1.0,
                doing: Doing::Snap,
            }
        } else if t < b.t_snap + SHUT_S {
            Jaws {
                a: ca,
                b: cb,
                size: 1.0,
                doing: Doing::Shut,
            }
        } else {
            let w = bold::ease_in((t - b.t_snap - SHUT_S) / BACK_S);
            Jaws {
                a: lerp(ca, me, w),
                b: lerp(cb, me, w),
                size: 1.0 - 0.55 * w,
                doing: Doing::Back,
            }
        })
    }

    /// The bites showing at `tc`, each with its halves. The pose is held where it bit for three frames (S1).
    fn jaws_at(&self, p: &PictureCtx, tc: f64, me: [f64; 2]) -> Vec<(&Bite, Jaws)> {
        let tp = bold::hold_time(p, self.k, &["bite"], tc);
        self.bites
            .iter()
            .filter(|b| b.t0 <= tc && tc <= b.t_snap + 0.4)
            .filter_map(|b| Some((b, self.jaws(p, b, tp, me)?)))
            .collect()
    }

    /// FULL BELLOWS at `t`: its far and near halves, their size and what they are doing.
    fn clamp_jaws(c: &Clamp, t: f64, me: [f64; 2]) -> Option<Jaws> {
        if t < c.t0 || t > c.t_snap + CLAMPED_S + HOME_S {
            return None;
        }
        let d = [dmath::cos(c.axis), dmath::sin(c.axis)];
        let back = [-d[0], -d[1]];
        let far = off(c.c, d, rope_dist(c.c, d) - 0.25);
        let near = off(c.c, back, rope_dist(c.c, back) - 0.25);
        let proj = (c.q[0] - c.c[0]) * d[0] + (c.q[1] - c.c[1]) * d[1];
        let qa = off(c.c, d, proj);
        let (qf, qn) = (off(qa, d, R * 0.9), off(qa, d, -R * 0.9));
        let t_close = c.t_snap - SLAM_S;
        Some(if t < t_close {
            let u = (t - c.t0) / (t_close - c.t0).max(1e-6);
            let k = bold::ease_out((u * 1.8).min(1.0));
            if u * 1.8 < 1.0 {
                Jaws {
                    a: lerp(c.c, far, k),
                    b: lerp(c.c, near, k),
                    size: 0.5 + 0.5 * k,
                    doing: Doing::Out,
                }
            } else {
                let q = 0.05 * dmath::sin(t * 85.0);
                Jaws {
                    a: off(far, d, q),
                    b: off(near, d, -q),
                    size: 1.0,
                    doing: Doing::Quiver((u * 1.8 - 1.0) / 0.8),
                }
            }
        } else if t < c.t_snap {
            let v = (t - t_close) / SLAM_S;
            Jaws {
                a: lerp(far, qf, v * v),
                b: lerp(near, qn, v * v),
                size: 1.0,
                doing: Doing::Snap,
            }
        } else if t < c.t_snap + CLAMPED_S {
            Jaws {
                a: qf,
                b: qn,
                size: 1.0,
                doing: Doing::Shut,
            }
        } else {
            let w = bold::ease_in((t - c.t_snap - CLAMPED_S) / HOME_S);
            Jaws {
                a: lerp(qf, me, w),
                b: lerp(qn, me, w),
                size: 1.0 - 0.6 * w,
                doing: Doing::Back,
            }
        })
    }

    /// The fight's clock for the weapon: stopped at the K.O.
    fn clock(p: &PictureCtx, t: f64) -> Option<f64> {
        (p.show.grid.t_fight <= t && t < p.t_hs + 0.25).then(|| t.min(p.t_ko))
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let Some(tc) = Self::clock(p, c.t) else {
            return;
        };
        let st = &c.fs[self.k];
        if !st.vis || st.shattered {
            return;
        }
        let tint = bold::light(p.col[self.k]);
        // where a bite will shut: the floor between the waiting halves lights up as the snap nears
        for (b, j) in self.jaws_at(p, tc, st.p) {
            match j.doing {
                Doing::Out => floor::lane(ic, c, j.a, j.b, 0.3, tint, 0.2),
                Doing::Quiver(u) => floor::lane(ic, c, j.a, j.b, 0.34, tint, 0.3 + 0.4 * u),
                Doing::Snap | Doing::Shut => {
                    let (_, _, mid, _) = self.shut_spots(p, b);
                    floor::disc(ic, c, mid[0], mid[1], 0.9, tint, 0.6);
                }
                Doing::Back => {}
            }
        }
        // FULL BELLOWS: the line it will slam along, then the blast going out
        for cl in &self.clamps {
            if let Some(j) = Self::clamp_jaws(cl, tc, st.p) {
                match j.doing {
                    Doing::Out => floor::lane(ic, c, j.a, j.b, 0.6, tint, 0.2),
                    Doing::Quiver(u) => floor::lane(ic, c, j.a, j.b, 0.65, tint, 0.25 + 0.4 * u),
                    _ => {}
                }
            }
            let u = (tc - cl.t_snap) / BLAST_S;
            if (0.0..1.0).contains(&u) {
                floor::band(
                    ic,
                    c,
                    Family::Air,
                    cl.q[0],
                    cl.q[1],
                    0.5 + 2.6 * bold::ease_out(u),
                    1.0 - u,
                    c.t,
                    tint,
                    1.1,
                );
            }
        }
    }

    pub(super) fn bold_items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        if st.shield > 0.0 && st.vis && !st.shattered && self.shield_dur.is_some() && f.t < p.t_hs + 0.3 {
            let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
            let back: Payload = Box::new(BItem::Collar(false));
            let front: Payload = Box::new(BItem::Collar(true));
            return vec![(dz + 0.03, back), (dz - 0.03, front)];
        }
        Vec::new()
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let mut out: Vec<(f64, Payload)> = Vec::new();
        let Some(tc) = Self::clock(p, f.t) else {
            return out;
        };
        let st = &f.fs[self.k];
        if !st.vis || st.shattered {
            return out;
        }
        let depth_at = |q: [f64; 2]| f.cam.depth(&[scene::to_world(q[0], q[1], 0.4)])[0];
        let halves = |out: &mut Vec<(f64, Payload)>, j: &Jaws, big: f64| {
            let face = unit([j.b[0] - j.a[0], j.b[1] - j.a[1]]);
            let back = [-face[0], -face[1]];
            out.push((depth_at(j.a), Box::new(BItem::Half(j.a, face, big * j.size, true))));
            out.push((depth_at(j.b), Box::new(BItem::Half(j.b, back, big * j.size, false))));
        };
        for (_, j) in self.jaws_at(p, tc, st.p) {
            // the bellows lie low: behind any ball they pass
            let dz = depth_at(lerp(j.a, j.b, 0.5)) + 1.5;
            out.push((dz, Box::new(BItem::Fan(st.p, j.a, j.b))));
            halves(&mut out, &j, 1.15);
        }
        for cl in &self.clamps {
            if let Some(j) = Self::clamp_jaws(cl, tc, st.p) {
                out.push((1e9, Box::new(BItem::Bellows(j.a, j.b, 0.5 * j.size, 21))));
                halves(&mut out, &j, 1.9);
            }
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let col = p.col[self.k];
        match payload.downcast_ref::<BItem>() {
            Some(&BItem::Bellows(a, b, half_w, n)) => draw_bellows(fr, f.cam, a, b, half_w, n, 0.34, col),
            Some(&BItem::Fan(me, a, b)) => draw_fan(fr, f.cam, me, a, b, 0.3, col),
            Some(&BItem::Half(c, face, size, keys)) => draw_half(fr, f.cam, c, face, size, keys, col),
            Some(&BItem::Collar(front)) => self.draw_collar(p, f, fr, front),
            None => {}
        }
    }

    /// SQUEEZEBOX: two pleated rings round the ball, one over the other and half a pleat apart. They fold out
    /// as the shield goes up, a blocked hit squeezes them, and they flatten and sink as it runs down.
    fn draw_collar(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, front: bool) {
        let st = &f.fs[self.k];
        let dur = self.shield_dur.unwrap_or(1.0);
        let left = p
            .bold_shield_left(self.k, f.t)
            .unwrap_or(st.shield / dur.max(1e-6))
            .clamp(0.0, 1.0);
        let fold = bold::ease_out((dur - st.shield) / FOLD_S) * (st.shield / 0.12).min(1.0);
        let squeeze = p
            .bold_last_block(self.k, f.t)
            .map_or(0.0, |(tb, _, _)| (1.0 - (f.t - tb) / SQUEEZE_S).max(0.0));
        let col = p.col[self.k];
        let light = col.map(|c| (c * 1.05 + 22.0).min(255.0));
        let dark: V3 = std::array::from_fn(|ch| PLEAT_DK[ch] + col[ch] * 0.22);
        let (cx, cz) = (st.p[0], st.p[1]);
        let c0 = f.cam.depth(&[scene::to_world(cx, cz, R)])[0];
        let n = 24;
        let tip = R * (1.2 + 0.85 * fold * (0.4 + 0.6 * left)) * (1.0 - 0.2 * squeeze);
        let valley = R * (1.1 + 0.3 * fold) * (1.0 - 0.1 * squeeze);
        for layer in 0..2usize {
            let lf = layer as f64;
            let y = R * (0.45 + 0.55 * lf * (0.3 + 0.7 * left)) + 0.04 * squeeze * lf;
            let turn = lf * TAU / n as f64 + f.t * 0.5;
            let at = |i: usize, r: f64| {
                let ang = TAU * i as f64 / n as f64 + turn;
                scene::to_world(cx + r * dmath::cos(ang), cz + r * dmath::sin(ang), y)
            };
            let rim = |i: usize| if i.is_multiple_of(2) { tip - 0.05 * lf } else { valley };
            for i in 0..n {
                let quad = [
                    at(i, R * 0.97),
                    at(i, rim(i)),
                    at(i + 1, rim(i + 1)),
                    at(i + 1, R * 0.97),
                ];
                let dz = f.cam.depth(&[quad[1], quad[2]]);
                if ((dz[0] + dz[1]) / 2.0 < c0) != front {
                    continue;
                }
                bold::outlined_poly(fr, f.cam, &quad, if (i + layer) % 2 == 0 { light } else { dark });
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let Some(tc) = Self::clock(p, f.t) else {
            return;
        };
        let st = &f.fs[self.k];
        if !st.vis || st.shattered {
            return;
        }
        let cam = f.cam;
        let col = p.col[self.k];
        let on_screen = |q: [f64; 2], y: f64| bold::at(cam, q[0], q[1], y);
        // a bite: arrows point inward while the halves quiver; the chomp is a star at each half and air out
        // both ways along the mouth
        for (_, j) in self.jaws_at(p, tc, st.p) {
            if let Doing::Quiver(u) = j.doing {
                arrows_in(
                    img,
                    on_screen(j.a, 0.45),
                    on_screen(j.b, 0.45),
                    15.0,
                    13.0,
                    f.t,
                    0.5 + 0.5 * u,
                );
            }
        }
        for b in self.bites.iter().filter(|b| (0.0..CHOMP_S).contains(&(tc - b.t_snap))) {
            let u = (tc - b.t_snap) / CHOMP_S;
            let (ca, cb, mid, d) = self.shut_spots(p, b);
            let big = if b.landed { 1.0 } else { 0.6 };
            let m = on_screen(mid, 0.5);
            for side in [-1.0, 1.0] {
                jet(img, m, on_screen(off(mid, d, side * 1.9 * big), 0.5), big, u);
            }
            if b.landed {
                for (q, rot) in [(ca, 0.3), (cb, 1.1)] {
                    let s = on_screen(q, 0.6);
                    let r = 5.0 + 5.0 * bold::ease_out(u / 0.5);
                    px::ink_star(img, s, r, r * 0.4, 4, rot, bold::WHITE, ((1.0 - u) / 0.4).min(1.0));
                }
            }
        }
        // FULL BELLOWS: the same, big
        for cl in &self.clamps {
            if let Some(j) = Self::clamp_jaws(cl, tc, st.p)
                && let Doing::Quiver(u) = j.doing
            {
                arrows_in(
                    img,
                    on_screen(j.a, 0.6),
                    on_screen(j.b, 0.6),
                    24.0,
                    20.0,
                    f.t,
                    0.5 + 0.5 * u,
                );
            }
            let u = (tc - cl.t_snap) / BLAST_S;
            if (0.0..1.0).contains(&u) {
                let d = [dmath::cos(cl.axis), dmath::sin(cl.axis)];
                let n = [-d[1], d[0]];
                let proj = (cl.q[0] - cl.c[0]) * d[0] + (cl.q[1] - cl.c[1]) * d[1];
                let qa = off(cl.c, d, proj);
                let m = on_screen(qa, 0.6);
                for side in [-1.0, 1.0] {
                    for spread in [-0.35, 0.0, 0.35] {
                        let to = off(off(qa, n, side * 2.6), d, spread * 2.0);
                        jet(img, m, on_screen(to, 0.6), 1.7, u);
                    }
                }
                for (s, rot) in [(-1.0, 0.2), (1.0, 0.9)] {
                    let at = on_screen(off(qa, d, s * R * 0.9), 0.7);
                    let r = 12.0 + 12.0 * bold::ease_out(u / 0.5);
                    px::ink_star(img, at, r, r * 0.45, 8, rot, bold::WHITE, ((1.0 - u) / 0.4).min(1.0));
                }
            }
        }
        let Some(d) = drawn.iter().find(|d| d.k == self.k) else {
            return;
        };
        // POLKA: notes hop round the ball in time
        if st.haste > 0.0 && f.in_fight {
            let beat = p.show.grid.beat.max(1e-6);
            let ph = (f.t - p.show.grid.t_fight) / beat;
            for i in 0..4 {
                let fi = f64::from(i);
                let ang = fi * TAU / 4.0 + f.t * 1.3;
                let hop = dmath::sin(std::f64::consts::PI * (ph * 2.0 + fi * 0.5)).abs();
                let c = (
                    d.s.0 + d.rp * 1.6 * dmath::cos(ang),
                    d.s.1 + d.rp * 0.75 * dmath::sin(ang) + d.rp * 0.5 - 13.0 * hop,
                );
                big_note(img, c, if i % 2 == 0 { bold::WHITE } else { bold::pale(col) });
            }
        }
        // SQUEEZEBOX: a blocked hit puffs air out all round
        if st.shield > 0.0
            && let Some((tb, _, _)) = p.bold_last_block(self.k, f.t)
        {
            let u = (f.t - tb) / (SQUEEZE_S + 0.1);
            if (0.0..1.0).contains(&u) {
                for i in 0..8 {
                    let ang = f64::from(i) * TAU / 8.0 + 0.3;
                    let far = d.rp * (1.7 + 1.3 * bold::ease_out(u));
                    let c = (d.s.0 + far * dmath::cos(ang), d.s.1 + far * 0.75 * dmath::sin(ang));
                    px::ink_disc(img, c, 4.5 * (1.0 - 0.6 * u), AIR, ((1.0 - u) / 0.5).min(1.0));
                }
            }
        }
    }

    /// The chomp: a main bite that lands is a heavy hit (the enemy squashes) and a bite between the beats a
    /// solid one, whatever their damage.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match how(e) {
            "bite" if e.f(8) != 0.0 => Some(HitSize::Heavy),
            "bite" => Some(by_damage.max(HitSize::Solid)),
            _ => Some(by_damage),
        }
    }
}
