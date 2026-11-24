//! BASSOON with the bold effects (`picture.effects = "bold"`; the owner's picks 34a to 34d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **TIPTOE** (34a): the enemy wears a little arrow where it is heading and a dashed arc behind its back; before
//!   and during every hop a dashed path runs over the floor to a landing mark, with big dots along the arc of the
//!   hop and tiny dust dots where it leaves and lands. A hop that will land behind the enemy carries a gold glint
//!   and turns its mark and the enemy's back arc gold; it stomps with a quake of lifted tiles, long crossed
//!   slashes and a burst of ticks (a stomp from the front is a small quake and a few ticks).
//! - **STACCATO** (34b): little dots hop in a ring round the ball while it lasts (the ring's own speed-up look
//!   shows the time left).
//! - **BOCAL HOOK** (34c): a thick brass crook swings out and curls round the enemy; the hooked enemy's arrow
//!   spins right round to face away inside whipping arcs and under dizzy stars; a gold glow under the bassoon and
//!   a glint on it say it is now behind.
//! - **MOUNTAIN KING** (34d): every landing leaves a pair of footprints and the ball an afterimage where it took
//!   off, on a circle of light round the enemy that brightens hop by hop while the camera shakes harder and
//!   harder; the last hop is a high leap and its stomp a double quake with a big burst.

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::py::py_min;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, GOLD, HitSize, Status, StatusKind};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::BallState;
use bf_picture::scene::{self, R_BALL as R};

use super::{BassoonLook, Crook, Hop, MAPLE_DUST, Stomp, THREAD, WHITE, crook_points, hop_height, how, unit};

/// The crook's brass.
const BRASS: V3 = [232.0, 188.0, 92.0];
/// Seconds before a hop its path starts to show, and after its landing the path takes to fade.
const PATH_PRE: f64 = 0.2;
const PATH_POST: f64 = 0.15;
/// Seconds a stomp's ticks and a backstab's slashes show.
const TICKS: f64 = 0.22;
const SLASH: f64 = 0.3;
/// Seconds the dust dots of a take-off or a landing show.
const DUST: f64 = 0.26;
/// Seconds a hooked enemy takes to spin round, and it stays dizzy.
const SPIN: f64 = 0.35;
const DIZZY: f64 = 0.7;
/// Seconds the gold glow says the bassoon is behind the hooked enemy.
const BEHIND: f64 = 0.8;
/// Seconds MOUNTAIN KING's afterimages and its last stomp's burst show.
const AFTER: f64 = 0.4;
const FINALE: f64 = 0.4;
/// How much higher the last hop of MOUNTAIN KING leaps.
const LEAP: f64 = 2.2;

/// An angle brought into -pi to pi.
fn wrap(a: f64) -> f64 {
    dmath::atan2(dmath::sin(a), dmath::cos(a))
}

/// A dashed arc of light on the canvas: the part of the ring of radius `r` round `e` within `half` radians of the
/// direction `dir`.
#[allow(clippy::too_many_arguments)]
fn dashed_arc(ic: &mut [V3], c: &CanvasCtx<'_>, e: [f64; 2], dir: f64, half: f64, r: f64, tint: V3, k: f64) {
    for (j, light) in ic.iter_mut().enumerate() {
        let (dx, dz) = (c.sxp[j] - e[0], c.szp[j] - e[1]);
        if (dmath::hypot(dx, dz) - r).abs() > 0.055 {
            continue;
        }
        let off = wrap(dmath::atan2(dz, dx) - dir);
        if off.abs() <= half && ((off + half) * r / 0.13).floor() as i64 % 2 == 0 {
            for ch in 0..3 {
                light[ch] += k * tint[ch];
            }
        }
    }
}

/// Short lines bursting out over the floor from `(x, z)`: `n` of them from radius `r0` to `r1`.
#[allow(clippy::too_many_arguments)]
fn ticks(img: &mut Pic, cam: &Cam, x: f64, z: f64, n: usize, r0: f64, r1: f64, w: f64, col: V3, a: f64) {
    for i in 0..n {
        let d = unit(dmath::TAU * (i as f64 + 0.5) / n as f64);
        let p0 = bold::at(cam, x + d[0] * r0, z + d[1] * r0, 0.04);
        let p1 = bold::at(cam, x + d[0] * r1, z + d[1] * r1, 0.04);
        px::ink_line(img, p0, p1, w, col, a);
    }
}

impl BassoonLook {
    /// The stomp a hop lands with, if the recording has it.
    fn stomp_of(&self, h: &Hop) -> Option<&Stomp> {
        self.stomps.iter().find(|s| (s.t - h.t_land).abs() < 0.02)
    }

    /// The hops whose path shows at `t`: about to leave, in the air or just landed; each with its strength (0 to
    /// 1) and how far it is from the first warning to the landing (0 to 1).
    fn paths(&self, p: &PictureCtx, t: f64) -> Vec<(&Hop, f64, f64)> {
        if !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return Vec::new();
        }
        self.hops
            .iter()
            .filter(|h| h.t - PATH_PRE <= t && t < h.t_land + PATH_POST)
            .map(|h| {
                let k = if t < h.t {
                    bold::ease_out((t - (h.t - PATH_PRE)) / PATH_PRE * 2.0)
                } else if t < h.t_land {
                    1.0
                } else {
                    1.0 - (t - h.t_land) / PATH_POST
                };
                let u = ((t - (h.t - PATH_PRE)) / (h.t_land - h.t + PATH_PRE).max(1e-6)).clamp(0.0, 1.0);
                (h, k, u)
            })
            .collect()
    }

    /// Where the enemy is heading at `t`, as an angle; a hooked enemy's heading spins right round to its new one.
    fn facing(&self, p: &PictureCtx, t: f64) -> Option<f64> {
        let o = 1 - self.k;
        let at = |i: usize| {
            let v = p.run.row("V", i, o)?;
            (dmath::hypot(v[0], v[1]) > 1e-3).then(|| dmath::atan2(v[1], v[0]))
        };
        let now = at(p.sim_i(py_min(t, p.t_ko)))?;
        for &(th, hit, _) in &self.hooks {
            if hit && th <= t && t < th + SPIN {
                let before = at(p.sim_i(th).saturating_sub(1)).unwrap_or(now);
                let turn = wrap(now - before);
                let whole = turn + if turn >= 0.0 { dmath::TAU } else { -dmath::TAU };
                return Some(now - (1.0 - bold::ease_out((t - th) / SPIN)) * whole);
            }
        }
        Some(now)
    }

    /// The last hop of each MOUNTAIN KING that ran its whole course.
    fn finales(&self, p: &PictureCtx) -> Vec<Hop> {
        let k = self.k;
        self.mks
            .iter()
            .filter_map(|&(t0, t1)| {
                let n = p
                    .rec
                    .events
                    .iter()
                    .find(|e| e.kind == "mountain" && e.who() == Some(k) && (e.t - t0).abs() < 1e-6)?
                    .i(3);
                let n = usize::try_from(n).ok()?.checked_sub(1)?;
                self.hops
                    .iter()
                    .filter(|h| h.kind == 2 && t0 - 1e-6 <= h.t && h.t <= t1 + 1e-6)
                    .nth(n)
                    .copied()
            })
            .collect()
    }

    /// The MOUNTAIN KING under way at `t`: its start, and how many of its hops have landed out of how many.
    fn dance(&self, p: &PictureCtx, t: f64) -> Option<(f64, usize, usize)> {
        let k = self.k;
        let &(t0, _) = self
            .mks
            .iter()
            .find(|&&(t0, t1)| t0 <= t && t < py_min(t1 + AFTER, p.t_ko))?;
        let n = p
            .rec
            .events
            .iter()
            .find(|e| e.kind == "mountain" && e.who() == Some(k) && (e.t - t0).abs() < 1e-6)
            .map_or(20, |e| e.i(3).max(1) as usize);
        let done = self
            .stomps
            .iter()
            .filter(|s| s.kind == 2 && t0 <= s.t && s.t <= t)
            .count();
        Some((t0, done.min(n), n))
    }

    /// The last hop of MOUNTAIN KING is a high leap.
    pub(super) fn bold_state(&self, p: &PictureCtx, st: &mut BallState, t: f64) {
        if let Some((hop, _)) = self.hop_at(p, t)
            && hop.kind == 2
            && self.finales(p).iter().any(|h| h.t == hop.t)
        {
            st.lift *= LEAP;
        }
    }

    /// MOUNTAIN KING: the camera shakes a little harder with every landing, and hard on the last.
    pub(super) fn bold_shake_more(&self, p: &PictureCtx, e: &Event) -> Vec<(f64, f64)> {
        if e.kind != "stomp" || e.i(8) != 2 || e.t > p.t_ko {
            return Vec::new();
        }
        if self.finales(p).iter().any(|h| (h.t_land - e.t).abs() < 0.02) {
            return vec![(e.t, 3.0)];
        }
        match self.dance(p, e.t) {
            Some((_, done, n)) => {
                let u = done as f64 / n as f64;
                vec![(e.t, 0.25 + 1.5 * u * u)]
            }
            None => Vec::new(),
        }
    }

    /// The last stomp of MOUNTAIN KING pops as a heavy hit whatever its damage.
    pub(super) fn bold_hit(&self, p: &PictureCtx, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        if how(e) == "stomp" && self.finales(p).iter().any(|h| (h.t_land - e.t).abs() < 0.02) {
            return Some(HitSize::Heavy);
        }
        Some(by_damage)
    }

    /// A hooked enemy is dizzy for a moment.
    pub(super) fn bold_statuses(&self, p: &PictureCtx, t: f64) -> Vec<Status> {
        self.hooks
            .iter()
            .filter(|&&(th, hit, _)| hit && th <= t && t < th + DIZZY && th < p.t_ko)
            .map(|&(th, _, _)| Status {
                who: 1 - self.k,
                kind: StatusKind::Stun,
                left: Some(1.0 - (t - th) / DIZZY),
            })
            .collect()
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let o = 1 - k;
        let green = bold::light(p.col[k]);
        let gold = bold::light(GOLD);
        let finales = self.finales(p);
        // the stomps: a quake of lifted tiles (a backstab's bigger and double, the last of MOUNTAIN KING's huge)
        for (s, age) in self.live_stomps(p, c.t) {
            let last = finales.iter().any(|h| (h.t_land - s.t).abs() < 0.02);
            let back = if s.back { 1.0 } else { 0.0 };
            let life = if last { 0.34 } else { 0.24 + 0.08 * back };
            if !(0.0..life).contains(&age) {
                continue;
            }
            let u = age / life;
            let reach = s.rad + 0.5 + 0.5 * back + if last { 1.3 } else { 0.0 };
            let r = 0.35 + reach * bold::ease_out(u);
            let tint = if s.back || last { gold } else { green };
            floor::band(ic, c, Family::Quake, s.x, s.z, r, 1.0 - u, c.t, tint, 0.8 + 0.5 * back);
            if s.back || last {
                floor::band(ic, c, Family::Quake, s.x, s.z, r * 0.58, 1.0 - u, c.t, green, 0.9);
            }
        }
        if !c.in_fight {
            return;
        }
        // the dashed path of the hop to come and its landing mark, gold when it will land behind the enemy
        for (h, strength, u) in self.paths(p, c.t) {
            let back = self.stomp_of(h).is_some_and(|s| s.back);
            let tint = if back { gold } else { green };
            floor::dashes(ic, c, h.p0, h.p1, 0.06, 0.2, c.t * 9.0, tint, 1.1 * strength);
            if c.t < h.t_land {
                floor::target(ic, c, h.p1[0], h.p1[1], R * 0.85, u, tint);
            }
        }
        let e = &c.fs[o];
        if e.vis
            && !e.shattered
            && let Some(face) = self.facing(p, c.t)
        {
            // the enemy's back: a dashed arc, gold while a hop is on its way to land behind it
            let half = p.cfg[k].raw["weapon"]["back_half"].as_f64().unwrap_or(55.0) * dmath::PI / 180.0;
            let coming = self
                .hop_at(p, c.t)
                .is_some_and(|(h, _)| self.stomp_of(h).is_some_and(|s| s.back));
            let (tint, kk) = if coming { (gold, 1.3) } else { ([0.9, 0.9, 1.0], 0.4) };
            dashed_arc(ic, c, e.p, face + dmath::PI, half, R * 1.55, tint, kk);
        }
        // BOCAL HOOK landed: a gold glow under the bassoon, now behind the enemy
        for &(th, hit, _) in &self.hooks {
            if hit && th <= c.t && c.t < th + BEHIND {
                let u = (c.t - th) / BEHIND;
                let me = c.fs[k].p;
                floor::disc(ic, c, me[0], me[1], R * 1.9, gold, 0.55 * (1.0 - u * u));
                floor::ring(ic, c, me[0], me[1], R * (1.5 + 0.9 * u), 0.05, gold, 1.2 * (1.0 - u));
            }
        }
        // MOUNTAIN KING: the circle of the dance brightens hop by hop; every landing leaves two footprints
        if let Some((t0, done, n)) = self.dance(p, c.t) {
            let gap = self.mk_gap.unwrap_or(0.1);
            let u = done as f64 / n as f64;
            floor::ring(ic, c, e.p[0], e.p[1], 2.0 * R + gap, 0.035, green, 0.15 + 0.75 * u);
            let life = 2.0 * p.show.grid.beat;
            let prints: Vec<&Stomp> = self
                .stomps
                .iter()
                .filter(|s| s.kind == 2 && t0 <= s.t && s.t <= c.t && c.t < s.t + life)
                .collect();
            for s in prints.iter().rev().take(6) {
                let fade = 1.0 - (c.t - s.t) / life;
                // side by side across the line to the enemy
                let toward = dmath::atan2(e.p[1] - s.z, e.p[0] - s.x);
                let n = unit(toward + dmath::PI / 2.0);
                for side in [1.0, -1.0] {
                    floor::disc(
                        ic,
                        c,
                        s.x + n[0] * 0.13 * side,
                        s.z + n[1] * 0.13 * side,
                        0.11,
                        green,
                        1.6 * fade,
                    );
                }
            }
        }
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if payload.downcast_ref::<Crook>().is_none() {
            return;
        }
        // BOCAL HOOK: the crook, thick brass with the dark outline, a knob at its tip and the red thread at its root
        let st = &f.fs[self.k];
        let Some((aim, ext, curl, glow)) = self.crook_state(p, f.t) else {
            return;
        };
        let pts: Vec<V3> = crook_points(st.p, aim, ext, R, curl, R + st.lift)
            .iter()
            .map(|q| scene::to_world(q[0], q[1], q[2]))
            .collect();
        let brass = if glow > 0.6 { bold::pale(BRASS) } else { BRASS };
        bold::thick_line(fr, f.cam, &pts, brass, 5);
        bold::blob(fr, f.cam, pts[0], 0.1, THREAD);
        bold::blob(fr, f.cam, pts[pts.len() - 1], 0.085, brass);
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let (k, t, cam) = (self.k, f.t, f.cam);
        let o = 1 - k;
        let col = p.col[k];
        let pale = bold::pale(col);
        let me = drawn.iter().find(|d| d.k == k);
        let foe = drawn.iter().find(|d| d.k == o);
        let finales = self.finales(p);
        if f.in_fight && t < p.t_ko {
            // the enemy shows where it is heading: a little arrow on the floor ahead of it
            let e = &f.fs[o];
            if e.vis
                && !e.shattered
                && let Some(face) = self.facing(p, t)
            {
                let d = unit(face);
                let n = [-d[1], d[0]];
                let at = |along: f64, across: f64| {
                    bold::at(
                        cam,
                        e.p[0] + d[0] * (R + along) + n[0] * across,
                        e.p[1] + d[1] * (R + along) + n[1] * across,
                        0.03,
                    )
                };
                let arrow = [at(0.7, 0.0), at(0.26, 0.32), at(0.42, 0.0), at(0.26, -0.32)];
                px::ink_poly(img, &arrow, bold::pale(p.col[o]), 0.95);
            }
            // MOUNTAIN KING: an afterimage where the ball took off
            if let (Some((t0, _, _)), Some(d)) = (self.dance(p, t), me) {
                let enemy_depth = cam.depth(&[scene::to_world(e.p[0], e.p[1], R)])[0];
                for h in self
                    .hops
                    .iter()
                    .filter(|h| h.kind == 2 && t0 <= h.t && h.t <= t && t < h.t + AFTER)
                {
                    let s = bold::at(cam, h.p0[0], h.p0[1], R);
                    // (not one that stands behind the enemy's ball)
                    let behind = cam.depth(&[scene::to_world(h.p0[0], h.p0[1], R)])[0] > enemy_depth
                        && foe.is_some_and(|q| dmath::hypot(s.0 - q.s.0, s.1 - q.s.1) < q.rp + d.rp);
                    if behind {
                        continue;
                    }
                    let a = 1.0 - (t - h.t) / AFTER;
                    px::disc(img, s, d.rp * 0.94, pale, 0.34 * a);
                    px::ring(img, s, d.rp, 1.4, col, 0.7 * a);
                }
            }
            // the hop's arc in big dots, with tiny dust dots where it leaves and lands
            for (h, strength, _) in self.paths(p, t) {
                let len = dmath::hypot(h.p1[0] - h.p0[0], h.p1[1] - h.p0[1]);
                let leap = if finales.iter().any(|q| q.t == h.t) { LEAP } else { 1.0 };
                let peak = hop_height(len) * leap;
                let back = self.stomp_of(h).is_some_and(|s| s.back);
                let shade = if back { GOLD } else { pale };
                let u_now = (t - h.t) / (h.t_land - h.t).max(1e-6);
                for i in 1..9 {
                    let u = i as f64 / 9.0;
                    // (the dots the ball has passed are gone)
                    if u < u_now {
                        continue;
                    }
                    let s = bold::at(
                        cam,
                        h.p0[0] + (h.p1[0] - h.p0[0]) * u,
                        h.p0[1] + (h.p1[1] - h.p0[1]) * u,
                        R + peak * 4.0 * u * (1.0 - u),
                    );
                    px::ink_disc(img, s, 1.9, shade, strength);
                }
                for (t_dust, q) in [(h.t, h.p0), (h.t_land, h.p1)] {
                    let age = t - t_dust;
                    if !(0.0..DUST).contains(&age) {
                        continue;
                    }
                    let u = age / DUST;
                    let seed = (t_dust * 1000.0) as u64;
                    for n in 0..6u64 {
                        let d = unit(dmath::TAU * px::rnd(seed, n));
                        let r = R * 0.55 + (0.3 + 0.4 * px::rnd(seed, n + 10)) * bold::ease_out(u);
                        let s = bold::at(cam, q[0] + d[0] * r, q[1] + d[1] * r, 0.04 + 0.16 * u);
                        px::ink_disc(img, s, 1.4, MAPLE_DUST[(n % 3) as usize], 1.0 - u);
                    }
                }
                // a hop that will land behind the enemy carries a gold glint
                if let (true, Some(d)) = (back && (0.0..1.0).contains(&u_now), me) {
                    let c = (d.s.0 + d.rp * 0.6, d.s.1 - d.rp * 0.75);
                    px::glint(img, c, 5.0 + 8.0 * u_now, GOLD, 1.0);
                }
            }
        }
        // the stomps: ticks burst over the floor; a backstab adds long crossed slashes over the enemy's back
        for (s, age) in self.live_stomps(p, t) {
            let last = finales.iter().any(|h| (h.t_land - s.t).abs() < 0.02);
            if last && age < FINALE {
                let u = age / FINALE;
                let (r0, r1) = (R * 1.1 + 1.6 * bold::ease_out(u), R * 1.6 + 2.3 * bold::ease_out(u));
                ticks(img, cam, s.x, s.z, 14, r0, r1, 3.0, WHITE, 1.0 - u);
                ticks(img, cam, s.x, s.z, 7, r0 * 0.6, r1 * 0.6, 2.2, GOLD, 1.0 - u);
            } else if age < TICKS {
                let u = age / TICKS;
                let (n, reach, w) = if s.back { (9, 1.0, 2.4) } else { (5, 0.5, 1.6) };
                let (r0, r1) = (R * 1.05 + reach * 0.6 * u, R * 1.25 + reach * bold::ease_out(u));
                ticks(
                    img,
                    cam,
                    s.x,
                    s.z,
                    n,
                    r0,
                    r1,
                    w,
                    if s.back { GOLD } else { pale },
                    1.0 - u,
                );
            }
        }
        for (x, z, aim, age) in self.live_slashes(p, t) {
            if age >= SLASH {
                continue;
            }
            let d = unit(aim);
            let n = [-d[1], d[0]];
            let grow = bold::ease_out(age / 0.06);
            let a = 1.0 - bold::ease_in(age / SLASH);
            for sgn in [1.0, -1.0] {
                let from = [x - d[0] * 0.3 + n[0] * 0.85 * sgn, z - d[1] * 0.3 + n[1] * 0.85 * sgn];
                let to = [x + d[0] * 0.3 - n[0] * 0.85 * sgn, z + d[1] * 0.3 - n[1] * 0.85 * sgn];
                let to = [from[0] + (to[0] - from[0]) * grow, from[1] + (to[1] - from[1]) * grow];
                let (p0, p1) = (
                    bold::at(cam, from[0], from[1], R + 0.35),
                    bold::at(cam, to[0], to[1], R - 0.1),
                );
                px::ink_line(img, p0, p1, 4.2, pale, a);
                px::line(img, p0, p1, 1.4, WHITE, a);
            }
        }
        if !f.in_fight {
            return;
        }
        // STACCATO: little dots hop in a ring round the ball
        let st = &f.fs[k];
        if st.haste > 0.0
            && st.vis
            && !st.shattered
            && let Some(d) = me
        {
            let own_depth = cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
            let pace = dmath::PI / (p.show.grid.beat / 2.0).max(1e-6);
            for n in 0..8 {
                let a = dmath::TAU * n as f64 / 8.0 + t * 1.6;
                let q = [st.p[0] + R * 1.7 * dmath::cos(a), st.p[1] + R * 1.7 * dmath::sin(a)];
                let y = 0.06 + 0.42 * dmath::sin(t * pace + n as f64 * 0.9).abs();
                let s = bold::at(cam, q[0], q[1], y);
                let hidden = dmath::hypot(s.0 - d.s.0, s.1 - d.s.1) < d.rp
                    && cam.depth(&[scene::to_world(q[0], q[1], y)])[0] > own_depth;
                if !hidden {
                    px::ink_disc(img, s, 3.6, if n % 2 == 0 { pale } else { WHITE }, 1.0);
                }
            }
        }
        // BOCAL HOOK: the hooked enemy whips round inside three arcs; a glint on the bassoon, now behind it
        for &(th, hit, _) in &self.hooks {
            if !hit || th >= p.t_ko {
                continue;
            }
            let age = t - th;
            if let (true, Some(q)) = ((0.0..SPIN + 0.1).contains(&age), foe) {
                let kk = 1.0 - age / (SPIN + 0.1);
                for j in 0..3 {
                    let a0 = t * 20.0 + j as f64 * dmath::TAU / 3.0;
                    let shade = if j == 0 { WHITE } else { pale };
                    px::ink_arc(img, q.s, q.rp * (1.3 + 0.12 * j as f64), 2.6, a0, a0 + 1.5, shade, kk);
                }
            }
            if let (true, Some(d)) = ((0.0..BEHIND).contains(&age), me) {
                let u = age / BEHIND;
                let c = (d.s.0 + d.rp * 0.6, d.s.1 - d.rp * 0.75);
                px::glint(img, c, 13.0 - 6.0 * u, GOLD, 1.0 - u * u);
            }
        }
    }
}
