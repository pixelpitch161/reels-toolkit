//! PICCOLO with the bold effects (`picture.effects = "bold"`; the owner's picks 5a to 5d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **PIERCING NOTE** (5a): the needle's whole folded path of the last quarter second draws in behind it, thick
//!   and outlined at the head with a white tip, thinning to a hairline at the tail, and is eaten up from the
//!   tail once the needle is gone. Every rope bounce gets a four-point glint on the rope and twangs it (the
//!   ring's own rope spark runs from there). A pierce: a glint where it goes in, sparks out of the back and a
//!   white hairline left through the enemy for a moment.
//! - **TRILL** (5b): a fluttering trill line pops out round the ball and wraps it while it lasts, and every
//!   needle flies with two afterimages beside it (the ring adds the ball's own speed-up look).
//! - **OVER THE BAND** (5c): the needles carry a bright glass halo; where one cuts through a shield the shield
//!   breaks like glass: its ring cracks into arcs and glass shards fly off it.
//! - **GLASS SHATTER** (5d): a rising line of notes climbs over the ball while the ropes frost over from the
//!   corners, grow icicles and crack; on the shatter the frost flashes white and each of the 24 shards breaks off
//!   its crack with a glint and flies in as an outlined, twinkling glass blade; where a shard ends, glass
//!   specks scatter and lie on the floor for two beats.

use bf_core::dmath;
use bf_core::event::Event;
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, INK};
use bf_picture::camera::Cam;
use bf_picture::look::{DrawnBall, FrameCtx, Pic};
use bf_picture::ring::PictureCtx;
use bf_picture::ring::ropes::RopeEv;
use bf_picture::scene::{self, A, R_BALL as R};

use super::{FLY_Y, GLASS, PiccoloLook, WHITE, how, trail};

/// Seconds of flight a needle's path shows.
const PATH_S: f64 = 0.26;
/// Units back from the head the thick, outlined part of the path runs, and the middle part after it.
const HEAD_LEN: f64 = 1.2;
const MID_LEN: f64 = 3.4;
/// Seconds a bounce's glint shows.
const GLINT_S: f64 = 0.17;
/// Seconds the hairline through the enemy stays, and the sparks out of its back fly.
const HAIR_S: f64 = 0.42;
const SPRAY_S: f64 = 0.28;
/// Seconds a cut shield's glass flies.
const BREAK_S: f64 = 0.5;
/// Seconds before GLASS SHATTER the ropes start to frost, and after its last shard the frost takes to go.
const FROST_IN_S: f64 = 0.45;
const FROST_OUT_S: f64 = 0.35;
/// Points a rope is drawn with under the frost.
const ROPE_N: usize = 41;

/// One needle or shard from the recording's events: where and when it was fired, and when it ended.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Flight {
    id: i64,
    t0: f64,
    t1: f64,
    x: f64,
    z: f64,
    vx: f64,
    vz: f64,
    shard: bool,
}

impl Flight {
    fn speed(&self) -> f64 {
        dmath::hypot(self.vx, self.vz)
    }

    /// Its whole path from where it was fired to where it is at `t` (or ended), folded at the ropes.
    fn path(&self, t: f64) -> Vec<[f64; 2]> {
        let len = self.speed() * (t.min(self.t1) - self.t0).max(0.0);
        trail(self.x, self.z, -self.vx, -self.vz, len, if self.shard { 0 } else { 99 })
    }
}

/// A needle through the enemy: when, and its heading.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Thru {
    t: f64,
    ux: f64,
    uz: f64,
}

/// What the bold look works out once from the recording.
#[derive(Clone, Debug, Default)]
pub(super) struct Bold {
    flights: Vec<Flight>,
    thrus: Vec<Thru>,
}

/// The stretch of `path` from `s0` to `s1` units back from its last point, the nearer end first.
fn back(path: &[[f64; 2]], s0: f64, s1: f64) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    let mut s = 0.0;
    for w in path.windows(2).rev() {
        let (a, b) = (w[1], w[0]);
        let l = dmath::hypot(b[0] - a[0], b[1] - a[1]);
        if l < 1e-9 {
            continue;
        }
        let (lo, hi) = (s0.max(s), s1.min(s + l));
        if lo < hi {
            let at = |d: f64| {
                let u = (d - s) / l;
                [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u]
            };
            if out.is_empty() {
                out.push(at(lo));
            }
            out.push(at(hi));
        }
        s += l;
        if s >= s1 {
            break;
        }
    }
    out
}

/// Ring-frame points at the needles' height, on the screen.
fn scr(cam: &Cam, pts: &[[f64; 2]]) -> Vec<(f64, f64)> {
    pts.iter().map(|q| bold::at(cam, q[0], q[1], FLY_Y)).collect()
}

/// A number from 0 to 1 that is always the same for the same `(a, b)`: the scatter of sparks and specks.
fn hash(a: i64, b: i64) -> f64 {
    let v = dmath::sin(a as f64 * 12.9898 + b as f64 * 78.233) * 43_758.545_3;
    v - v.floor()
}

/// The point on the ropes nearest `(x, z)` and that rope's side (an index of the ring's rope sides).
fn on_rope(x: f64, z: f64) -> (usize, f64, f64) {
    if z.abs() >= x.abs() {
        if z > 0.0 { (0, x, A) } else { (2, x, -A) }
    } else if x > 0.0 {
        (1, A, z)
    } else {
        (3, -A, z)
    }
}

/// Whether the screen point is on one of the drawn balls.
fn on_ball(drawn: &[DrawnBall], s: (f64, f64)) -> bool {
    drawn.iter().any(|d| dmath::hypot(s.0 - d.s.0, s.1 - d.s.1) < d.rp)
}

/// `pts` moved `off` pixels sideways (to the left of its run).
fn beside(pts: &[(f64, f64)], off: f64) -> Vec<(f64, f64)> {
    (0..pts.len())
        .map(|i| {
            let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(pts.len() - 1)]);
            let n = dmath::hypot(b.0 - a.0, b.1 - a.1).max(1e-9);
            (pts[i].0 - (b.1 - a.1) / n * off, pts[i].1 + (b.0 - a.0) / n * off)
        })
        .collect()
}

impl PiccoloLook {
    /// The needles, shards and pierces of side `k`, from its events.
    pub(super) fn bold_setup(ev: &[Event], k: usize) -> Bold {
        let mut flights: Vec<Flight> = ev
            .iter()
            .filter(|e| e.kind == "needle" && e.who() == Some(k))
            .map(|e| Flight {
                id: e.i(3),
                t0: e.t,
                t1: f64::INFINITY,
                x: e.f(4),
                z: e.f(5),
                vx: e.f(6),
                vz: e.f(7),
                shard: e.s(8) == "shard",
            })
            .collect();
        for e in ev.iter().filter(|e| e.kind == "needle_end" && e.who() == Some(k)) {
            if let Some(fl) = flights.iter_mut().find(|fl| fl.id == e.i(3)) {
                fl.t1 = e.t;
            }
        }
        let thrus = ev
            .iter()
            .filter(|e| e.kind == "needle_hit" && e.who() == Some(k))
            .filter_map(|e| {
                let fl = flights.iter().find(|fl| fl.id == e.i(3) && !fl.shard)?;
                let path = fl.path(e.t);
                let (a, b) = (path.get(path.len().checked_sub(2)?)?, path.last()?);
                let n = dmath::hypot(b[0] - a[0], b[1] - a[1]);
                (n > 1e-9).then(|| Thru {
                    t: e.t,
                    ux: (b[0] - a[0]) / n,
                    uz: (b[1] - a[1]) / n,
                })
            })
            .collect();
        Bold { flights, thrus }
    }

    /// Each bounce and each shard breaking off twangs its rope (and the ring's own spark runs along it).
    pub(super) fn bold_rope_events(&self) -> Vec<RopeEv> {
        let bounces = self.bounces.iter().map(|b| (b.t, b.x, b.z, 2.0));
        let shards = self
            .bold
            .flights
            .iter()
            .filter(|fl| fl.shard)
            .map(|fl| (fl.t0, fl.x, fl.z, 1.3));
        bounces
            .chain(shards)
            .map(|(t, x, z, spd)| {
                let (side, x, z) = on_rope(x, z);
                RopeEv { t, side, x, z, spd }
            })
            .collect()
    }

    /// A needle that cuts through a shield lands as a solid hit whatever its damage.
    pub(super) fn bold_hit(&self, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        if how(e) == "needle" && self.cuts.iter().any(|c| (c.0 - e.t).abs() < 1e-6) {
            return Some(by_damage.max(HitSize::Solid));
        }
        Some(by_damage)
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, t) = (self.k, f.t);
        let o = 1 - k;
        let col = p.col[k];
        self.bold_frost(p, img, f, drawn);
        self.bold_specks(p, img, f, drawn);
        // TRILL: a fluttering trill line wraps the ball, popping out as it starts
        if f.fs[k].haste > 0.0
            && !f.fs[k].shattered
            && let Some(d) = drawn.iter().find(|d| d.k == k)
        {
            let pop = self
                .trills
                .iter()
                .rev()
                .find(|&&t0| t0 <= t)
                .map_or(1.0, |&t0| bold::ease_out((t - t0) / 0.15));
            let r = d.rp + 3.0 + 9.0 * pop;
            let amp = 2.8 * (0.65 + 0.35 * dmath::sin(t * 31.0));
            let line: Vec<(f64, f64)> = (0..=60)
                .map(|i| {
                    let a = i as f64 / 60.0 * dmath::TAU;
                    let rr = r + amp * dmath::sin(10.0 * a + 26.0 * t);
                    (d.s.0 + rr * dmath::cos(a), d.s.1 + rr * dmath::sin(a))
                })
                .collect();
            px::ink_polyline(img, &line, 1.5, bold::pale(col), 0.95);
        }
        // the needles and the shards
        let over = Self::in_window(&self.over, t);
        let trill = f.fs[k].haste > 0.0;
        for fl in self.bold.flights.iter().filter(|fl| fl.t0 <= t) {
            if fl.shard {
                if t < fl.t1 {
                    Self::bold_shard(img, f.cam, fl, t);
                }
                continue;
            }
            // the path is eaten up from the tail once the needle is gone
            let shown = fl.speed() * (PATH_S - (t - fl.t1).max(0.0));
            if shown <= 0.0 {
                continue;
            }
            let path = fl.path(t);
            if t >= fl.t1 {
                // gone: what is left of its path thins out and fades
                let left = scr(f.cam, &back(&path, 0.0, shown));
                px::polyline(img, &left, 1.8, col, 0.8 * shown / (fl.speed() * PATH_S));
                continue;
            }
            let tail = scr(f.cam, &back(&path, MID_LEN.min(shown), shown));
            let mid = scr(f.cam, &back(&path, HEAD_LEN.min(shown), MID_LEN.min(shown)));
            let head = scr(f.cam, &back(&path, 0.0, HEAD_LEN.min(shown)));
            px::polyline(img, &tail, 1.6, col, 0.6);
            if over {
                px::polyline(img, &mid, 6.0, GLASS, 0.4);
                px::polyline(img, &head, 9.5, GLASS, 0.55);
            }
            if trill {
                for off in [-3.5, 3.5] {
                    px::polyline(img, &beside(&mid, off), 1.0, bold::pale(col), 0.4);
                    px::polyline(img, &beside(&head, off), 1.4, bold::pale(col), 0.55);
                }
            }
            px::polyline(img, &mid, 2.6, col, 0.95);
            if over {
                px::polyline(img, &head, 6.0, WHITE, 0.9);
                px::polyline(img, &head, 3.8, col, 1.0);
            } else {
                px::ink_polyline(img, &head, 3.8, col, 1.0);
            }
            px::polyline(img, &head, 1.4, WHITE, 1.0);
            // the tip: a white diamond along its flight, while it flies
            if head.len() >= 2 {
                let (s0, s1) = (head[0], head[1]);
                let n = dmath::hypot(s0.0 - s1.0, s0.1 - s1.1).max(1e-9);
                let (ux, uy) = ((s0.0 - s1.0) / n, (s0.1 - s1.1) / n);
                let tip = [
                    (s0.0 + ux * 5.0, s0.1 + uy * 5.0),
                    (s0.0 - uy * 2.6, s0.1 + ux * 2.6),
                    (s0.0 - ux * 4.0, s0.1 - uy * 4.0),
                    (s0.0 + uy * 2.6, s0.1 - ux * 2.6),
                ];
                if over {
                    px::ring(img, s0, 7.5, 1.6, GLASS, 0.95);
                }
                px::ink_poly(img, &tip, WHITE, 1.0);
            }
        }
        // each rope bounce: a four-point glint on the rope
        for b in self.bounces.iter().filter(|b| b.t <= t && t < b.t + GLINT_S) {
            let u = (t - b.t) / GLINT_S;
            let (_, x, z) = on_rope(b.x, b.z);
            px::glint(
                img,
                bold::at(f.cam, x, z, FLY_Y),
                12.0 - 5.0 * u,
                bold::pale(col),
                1.0 - u * u,
            );
        }
        // a pierce: a glint going in, sparks out of the back, a hairline left through the enemy
        let e = f.fs[o].p;
        for h in self.bold.thrus.iter().filter(|h| h.t <= t && t < h.t + HAIR_S) {
            if f.fs[o].shattered {
                break;
            }
            let age = t - h.t;
            let along = |d: f64, side: f64| {
                bold::at(
                    f.cam,
                    e[0] + h.ux * d - h.uz * side,
                    e[1] + h.uz * d + h.ux * side,
                    FLY_Y,
                )
            };
            px::line(
                img,
                along(-R * 1.45, 0.0),
                along(R * 1.45, 0.0),
                1.3,
                WHITE,
                0.95 * (1.0 - age / HAIR_S),
            );
            if age < 0.12 {
                px::glint(img, along(-R, 0.0), 10.0, WHITE, 1.0 - age / 0.12);
            }
            if age < SPRAY_S {
                let u = age / SPRAY_S;
                let seed = (h.t * 1000.0) as i64;
                for j in 0..7i64 {
                    let ang = (j as f64 - 3.0) * 0.2;
                    let far = R + (0.5 + 0.9 * hash(seed, j)) * bold::ease_out(u);
                    let (c, s) = (dmath::cos(ang), dmath::sin(ang));
                    let (d0, d1) = (far - 0.3 * (1.0 - 0.6 * u), far);
                    let c_j = if j % 2 == 0 { WHITE } else { bold::pale(col) };
                    px::ink_line(img, along(d0 * c, d0 * s), along(d1 * c, d1 * s), 1.5, c_j, 1.0 - u * u);
                }
            }
        }
        // OVER THE BAND through a shield: the shield breaks like glass
        if let Some(d) = drawn.iter().find(|d| d.k == o) {
            for &(tc, _, _) in self.cuts.iter().filter(|c| c.0 <= t && t < c.0 + BREAK_S) {
                let u = (t - tc) / BREAK_S;
                let r0 = d.rp + 7.0;
                // the ring cracks into arcs that drift apart
                if u < 0.6 {
                    for j in 0..5 {
                        let a0 = j as f64 * dmath::TAU / 5.0 + 0.4;
                        px::ink_arc(img, d.s, r0 + 9.0 * u, 2.2, a0 + 0.12, a0 + 1.0, GLASS, 1.0 - u / 0.6);
                        let (c, s) = (dmath::cos(a0), dmath::sin(a0));
                        px::line(
                            img,
                            (d.s.0 + c * (r0 - 6.0), d.s.1 + s * (r0 - 6.0)),
                            (d.s.0 + c * (r0 + 7.0), d.s.1 + s * (r0 + 7.0)),
                            1.4,
                            WHITE,
                            1.0 - u / 0.6,
                        );
                    }
                }
                // and glass shards fly off it, turning
                for j in 0..10i64 {
                    let ang = j as f64 * dmath::TAU / 10.0 + 0.2;
                    let far = r0 + (20.0 + 16.0 * hash(j, 7)) * bold::ease_out(u);
                    let c = (
                        d.s.0 + far * dmath::cos(ang),
                        d.s.1 + far * dmath::sin(ang) + 14.0 * u * u,
                    );
                    let rot = ang + u * 5.0 * if j % 2 == 0 { 1.0 } else { -1.0 };
                    let size = 6.5 - 2.0 * u;
                    let tri: Vec<(f64, f64)> = (0..3)
                        .map(|q| {
                            let a = rot + q as f64 * dmath::TAU / 3.0;
                            let r = if q == 0 { size } else { size * 0.55 };
                            (c.0 + r * dmath::cos(a), c.1 + r * dmath::sin(a))
                        })
                        .collect();
                    px::ink_poly(img, &tri, if j % 3 == 0 { WHITE } else { GLASS }, 1.0 - u * u);
                }
            }
        }
    }

    /// One GLASS SHATTER shard in the air: an outlined glass blade with a light edge, a faint streak behind it
    /// and a twinkle.
    fn bold_shard(img: &mut Pic, cam: &Cam, fl: &Flight, t: f64) {
        let n = fl.speed().max(1e-9);
        let (ux, uz) = (fl.vx / n, fl.vz / n);
        let d = n * (t - fl.t0);
        let (x, z) = (fl.x + ux * d, fl.z + uz * d);
        let at = |a: f64, b: f64| bold::at(cam, x + ux * a - uz * b, z + uz * a + ux * b, FLY_Y);
        px::line(img, at(-0.5, 0.0), at(-0.1, 0.0), 1.4, GLASS, 0.5);
        px::ink_poly(
            img,
            &[at(0.24, 0.0), at(-0.14, 0.11), at(-0.06, 0.0), at(-0.14, -0.11)],
            GLASS,
            1.0,
        );
        px::line(img, at(0.2, 0.0), at(-0.04, 0.0), 1.0, WHITE, 1.0);
        let tw = t * 7.0 + fl.id as f64 * 0.37;
        if tw - tw.floor() < 0.4 {
            px::glint(img, at(0.1, 0.0), 6.0, WHITE, 0.95);
        }
    }

    /// GLASS SHATTER's ropes: `(start of the frost, the shatter, when the last shard broke off)`.
    fn frost_times(&self) -> Option<(f64, f64, f64)> {
        let ts = self.shatter?;
        let last = self
            .bold
            .flights
            .iter()
            .filter(|fl| fl.shard)
            .fold(ts, |m, fl| m.max(fl.t0));
        Some((ts - FROST_IN_S, ts, last))
    }

    /// GLASS SHATTER's wind-up and ropes: the rising line of notes over the ball, the frost creeping in from the
    /// corners with icicles, a crack where each shard will break off and a glint as it does.
    fn bold_frost(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let Some((t_in, ts, t_last)) = self.frost_times() else {
            return;
        };
        let t = f.t;
        if t < t_in || t >= t_last + FROST_OUT_S {
            return;
        }
        let grow = bold::ease_out((t - t_in) / FROST_IN_S);
        let fade = (1.0 - (t - t_last) / FROST_OUT_S).clamp(0.0, 1.0);
        let flash = (1.0 - (t - ts) / 0.12).clamp(0.0, 1.0) * if t >= ts { 1.0 } else { 0.0 };
        let ice = if flash > 0.0 { WHITE } else { GLASS };
        // the frost: over each rope from both corners toward its middle (the back ropes not over a ball)
        for side in 0..4 {
            for (n_y, y) in scene::ROPE_Y.into_iter().enumerate() {
                let pts = f.cam.wp(&p.rope_points(side, y, t, ROPE_N));
                for (i, pair) in pts.windows(2).enumerate() {
                    let v = (i as f64 + 0.5) / (ROPE_N - 1) as f64;
                    if v.min(1.0 - v) > 0.5 * grow {
                        continue;
                    }
                    let mid = ((pair[0].0 + pair[1].0) / 2.0, (pair[0].1 + pair[1].1) / 2.0);
                    if side >= 2 && on_ball(drawn, mid) {
                        continue;
                    }
                    px::line(img, pair[0], pair[1], 2.6 + 1.5 * flash, ice, 0.9 * fade);
                    // icicles under the lowest rope, and a sparkle here and there
                    if n_y == 0 && i % 3 == 1 {
                        let len = (4.0 + 4.0 * hash(side as i64, i as i64)) * grow;
                        px::poly(
                            img,
                            &[
                                (mid.0 - 1.6, mid.1 + 1.0),
                                (mid.0 + 1.6, mid.1 + 1.0),
                                (mid.0, mid.1 + 1.0 + len),
                            ],
                            ice,
                            0.9 * fade,
                        );
                    }
                    let tw = t * 5.0 + hash(i as i64, (side * 3 + n_y) as i64) * 9.0;
                    if tw - tw.floor() < 0.12 {
                        px::glint(img, mid, 4.0, WHITE, fade);
                    }
                }
            }
        }
        // the cracks: one where each shard will break off, spreading in before the shatter
        let shards: Vec<&Flight> = self.bold.flights.iter().filter(|fl| fl.shard).collect();
        let n = shards.len().max(1) as f64;
        for (j, fl) in shards.iter().enumerate() {
            let t_crack = ts - 0.3 + 0.3 * j as f64 / n;
            if t < t_crack {
                continue;
            }
            let (side, x, z) = on_rope(fl.x, fl.z);
            // along the rope
            let (ax, az) = if side % 2 == 0 { (1.0, 0.0) } else { (0.0, 1.0) };
            let zig = [(0.30, -0.05), (0.62, 0.06), (0.86, -0.06), (1.06, 0.05), (1.30, -0.04)];
            let line: Vec<(f64, f64)> = zig
                .iter()
                .map(|&(y, off)| bold::at(f.cam, x + ax * off, z + az * off, y))
                .collect();
            if side >= 2 && on_ball(drawn, line[2]) {
                continue;
            }
            px::polyline(img, &line, 1.6, INK, 0.95 * fade);
            let age = t - fl.t0;
            if (0.0..0.14).contains(&age) {
                px::glint(img, line[1], 11.0 - 30.0 * age, WHITE, 1.0 - age / 0.14);
            }
        }
        // the rising line of notes over the ball, until just after the shatter
        if t < ts + 0.15
            && let Some(d) = drawn.iter().find(|d| d.k == self.k)
        {
            let u = ((t - t_in) / FROST_IN_S).clamp(0.0, 1.0);
            let a = (1.0 - (t - ts) / 0.15).clamp(0.0, 1.0);
            let step = |j: f64| (d.s.0 - 14.0 + 7.0 * j, d.s.1 - d.rp - 7.0 - 6.0 * j - 1.2 * j * j);
            let steps = (u * 5.0).ceil().clamp(1.0, 5.0) as usize;
            let line: Vec<(f64, f64)> = (0..steps).map(|j| step(j as f64)).collect();
            px::ink_polyline(img, &line, 1.4, bold::pale(p.col[self.k]), a);
            for (j, &c) in line.iter().enumerate() {
                let top = j + 1 == steps;
                px::ink_disc(img, c, if top { 3.4 } else { 2.4 }, if top { WHITE } else { GLASS }, a);
            }
        }
    }

    /// Where each shard ended: glass specks scatter and lie on the floor for two beats, twinkling.
    fn bold_specks(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let life = 2.0 * p.show.grid.beat;
        let t = f.t;
        for fl in self
            .bold
            .flights
            .iter()
            .filter(|fl| fl.shard && fl.t1 <= t && t < fl.t1 + life)
        {
            let age = t - fl.t1;
            let n = fl.speed().max(1e-9);
            let (ux, uz) = (fl.vx / n, fl.vz / n);
            let d = n * (fl.t1 - fl.t0);
            let (x, z) = (fl.x + ux * d, fl.z + uz * d);
            let out = bold::ease_out(age / 0.2);
            let y = FLY_Y * (1.0 - bold::ease_in(age / 0.2)) + 0.02;
            for j in 0..6i64 {
                let ang = (hash(fl.id, j) - 0.5) * 4.4;
                let far = (0.14 + 0.4 * hash(fl.id, j + 9)) * out;
                let (c, s) = (dmath::cos(ang), dmath::sin(ang));
                let at = bold::at(f.cam, x + (ux * c - uz * s) * far, z + (uz * c + ux * s) * far, y);
                if age > 0.2 && on_ball(drawn, at) {
                    continue;
                }
                let tw = 0.65 + 0.35 * dmath::sin(t * 17.0 + j as f64 * 2.1 + fl.id as f64);
                let a = (1.0 - age / life).min(1.0) * tw;
                if j % 2 == 0 {
                    px::ink_star(img, at, 3.6, 1.3, 4, 0.0, GLASS, a);
                } else {
                    px::ink_disc(img, at, 1.4, WHITE, a);
                }
            }
        }
    }
}
