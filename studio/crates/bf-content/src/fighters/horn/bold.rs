//! HORN with the bold effects (`picture.effects = "bold"`; the owner's picks 33a to 33d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **ECHO CALL** (33a): the blast fires as stacked arcs toward the enemy over a brass band on the floor; a
//!   see-through ghost stands where the ball blasted, a gold ring round it draining until it repeats the blast, and
//!   it fires its own, smaller arcs. The echoes' full rings are only a faint line, so they no longer pile up.
//! - **STOPPED HORN** (33b): a big copper hand comes up in front of the bell, jolts out white with a burst of
//!   ticks when it blocks, and sinks as the shield runs down.
//! - **HORN RIP** (33c): gold streaks flicker round the ball while it lasts, with a burst of them as it starts
//!   (the countdown rings of the ghosts drain twice as fast; the ring's own speed-up look shows the time left).
//! - **FINAL FLICK** (33d): a giant copper glove with a brass cuff, bright over the dimmed ring, trembles harder and
//!   harder with a glint on its curled finger over a lane of running dashes; it flicks with a white swoosh, and
//!   the enemy rockets off on a copper streak with speed lines, rope to rope.

use bf_core::dmath;
use bf_core::py::py_min;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, GOLD};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::ss;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::scene::{self, R_BALL as R};

use super::{
    BRASS, COPPER, COPPER_DK, GhostAt, Glove, HornLook, Item, Part, RingAt, WHITE, draw_hand, glove_world, norm2, unit,
};

/// How strong a standing ghost is by the echo it will fire next (the first the strongest).
const STAND_A: [f64; 4] = [0.92, 0.78, 0.66, 0.56];
/// Seconds a ghost takes to appear where the ball blasted, and to fade after its last echo.
const PEEL: f64 = 0.14;
/// Seconds the copper hand takes to come up.
const HAND_UP: f64 = 0.25;
/// Seconds the ticks of a block show.
const TICKS: f64 = 0.16;
/// Seconds HORN RIP's opening burst of streaks lasts.
const RIP_POP: f64 = 0.3;
/// Seconds the swoosh of the flick shows.
const SWOOSH: f64 = 0.12;
/// Seconds a flicked enemy trails its streak, and how many seconds of its way the streak covers.
const FLUNG: f64 = 0.9;
const STREAK: f64 = 0.24;

/// What the copper hand's colour is mixed with: polished, lighter than the horn's own ball.
const HAND: V3 = [255.0, 226.0, 196.0];
/// How much bigger than the fight's glove the bold one is drawn.
const GIANT: f64 = 1.3;

/// A ghost standing where the ball blasted.
#[derive(Clone, Copy)]
struct Stand {
    x: f64,
    z: f64,
    /// Its strength.
    a: f64,
    /// Its flash as it repeats the blast.
    fl: f64,
    /// The share of its wait that is left until it blasts again; `None` once it has fired its last.
    left: Option<f64>,
}

/// A piece of a ring round `c` on the screen: `half` radians either side of `dir`, at height `y`.
fn arc_pts(cam: &Cam, c: [f64; 2], r: f64, dir: f64, half: f64, y: f64) -> Vec<(f64, f64)> {
    let steps = ((half * r * 6.0).ceil() as usize).clamp(5, 36);
    (0..=steps)
        .map(|i| {
            let a = dir - half + 2.0 * half * i as f64 / steps as f64;
            bold::at(cam, c[0] + r * dmath::cos(a), c[1] + r * dmath::sin(a), y)
        })
        .collect()
}

impl HornLook {
    /// The ghosts standing at `t`: one where each of the last calls was sounded, until it has repeated it as often
    /// as the call had ghosts. (The fight's ghosts slide along the path between the beats and are at these places
    /// as they blast: a ghost that stands still says where the next echo comes from.)
    fn stands(&self, p: &PictureCtx, t: f64) -> Vec<Stand> {
        let g = &p.show.grid;
        if !self.echo || t < g.t_fight {
            return Vec::new();
        }
        let k = self.k;
        // after the K.O. a winner's ghosts fade out, a loser's are gone (they burst into embers)
        let out_k = if t < p.t_hs {
            1.0
        } else if k == p.lose {
            0.0
        } else {
            1.0 - ss(p.t_hs, p.t_hs + 0.8, t)
        };
        if out_k <= 0.0 {
            return Vec::new();
        }
        let tc = py_min(p.fx_clock(t), p.t_ko);
        let mut out = Vec::new();
        for e in bold::events_in(p, k, &["echo_call"], tc - 4.0 * g.beat - PEEL - 0.05, tc) {
            let (call, n, sp) = (e.i(3), e.i(4), e.i(5) as f64 * g.beat / 4.0);
            if n < 1 || sp <= 0.0 {
                continue;
            }
            let age = tc - e.t;
            let j = (age / sp + 1e-6).floor() as i64 + 1;
            let at = p.run.p(p.sim_i(e.t), k);
            if j > n {
                // it has fired its last: a flash, and gone
                let over = age - n as f64 * sp;
                if over < PEEL {
                    out.push(Stand {
                        x: at[0],
                        z: at[1],
                        a: STAND_A[(n as usize - 1).min(3)] * (1.0 - over / PEEL) * out_k,
                        fl: 1.0 - over / PEEL,
                        left: None,
                    });
                }
                continue;
            }
            // its next blast as the fight recorded it (the place can differ a little while the pace changes)
            let next = bold::events_in(p, k, &["echo"], tc, tc + sp + 0.1).find(|x| x.i(4) == call && x.i(3) == j);
            let (t_next, x, z) = next.map_or((e.t + j as f64 * sp, at[0], at[1]), |x| (x.t, x.f(5), x.f(6)));
            let t_prev = e.t + (j - 1) as f64 * sp;
            let fl = if j >= 2 { dmath::exp(-(tc - t_prev) / 0.14) } else { 0.0 };
            out.push(Stand {
                x,
                z,
                a: STAND_A[(j as usize - 1).min(3)] * bold::ease_out(age / PEEL) * out_k,
                fl,
                left: Some(((t_next - tc) / (t_next - t_prev).max(1e-6)).clamp(0.0, 1.0)),
            });
        }
        out
    }

    /// Where the copper hand is on the screen for a ball drawn at `s` with radius `rp`, its enemy at `so`: its
    /// centre and which way it is mirrored. `sunk` (0 to 1) lowers it.
    fn hand_at(s: (f64, f64), rp: f64, so: (f64, f64), push: f64, sunk: f64) -> ((f64, f64), f64) {
        let dv = [so.0 - s.0, so.1 - s.1];
        let n = norm2(dv) + 1e-9;
        let dv = [dv[0] / n, dv[1] / n];
        let out = rp * (1.0 + 0.22 * push);
        let c = (s.0 + dv[0] * out, s.1 + dv[1] * out + rp * 0.1 + rp * 1.05 * sunk);
        (c, if dv[0] >= 0.0 { 1.0 } else { -1.0 })
    }

    /// How far the copper hand is sunk at `t` (1 out of sight below the ball, 0 in front of the bell): it comes up
    /// as the shield starts and sinks as it runs down.
    fn hand_sunk(&self, p: &PictureCtx, t: f64, shield: f64) -> f64 {
        let dur = self.dur_beats1.unwrap_or(6.0) * p.show.grid.beat;
        let up = bold::ease_out((dur - shield) / HAND_UP);
        let left = p.bold_shield_left(self.k, t).unwrap_or(1.0);
        (1.0 - up) + 0.62 * bold::ease_in(1.0 - left)
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let tint = bold::light(p.col[k]);
        // the call: a brass band; an echo: only a faint line (its arcs toward the enemy say the rest)
        for r in self.rings_at(p, c.t) {
            if r.j == 0 {
                floor::band(ic, c, Family::Brass, r.x, r.z, r.r, r.a, c.t, tint, 0.85);
            } else {
                let fade = dmath::pow(0.85, (r.j - 1) as f64);
                floor::ring(ic, c, r.x, r.z, r.r, 0.04, tint, 0.3 * r.a * fade);
            }
        }
        // a copper glow under every standing ghost
        for gh in self.stands(p, c.t) {
            floor::disc(ic, c, gh.x, gh.z, 0.7, tint, (0.2 + 0.35 * gh.fl) * gh.a);
        }
        // FINAL FLICK: the lane the glove will flick along, dashes running down it, a mark on the enemy in reach
        if let Some(g) = self.glove_at(p, c.t)
            && g.pose < 0.5
        {
            let u = bold::until(p, k, &["flick"], c.t, p.show.grid.beat).map_or(0.0, |(u, _)| u);
            let reach = self.reach.unwrap_or(2.4);
            let d = unit(g.aim);
            let a = [g.p[0] + d[0] * R, g.p[1] + d[1] * R];
            let b = [g.p[0] + d[0] * (R + reach), g.p[1] + d[1] * (R + reach)];
            floor::lane(ic, c, a, b, 0.3, tint, (0.2 + 0.3 * u) * g.scale.min(1.0));
            floor::dashes(ic, c, a, b, 0.12, 0.45, c.t * 7.0, tint, 0.9 * g.scale.min(1.0));
            let e = c.fs[1 - k].p;
            if dmath::hypot(e[0] - g.p[0], e[1] - g.p[1]) <= 2.0 * R + reach {
                floor::target(ic, c, e[0], e[1], R * 1.2, u, tint);
            }
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let (cam, t) = (f.cam, f.t);
        let mut items: Vec<(f64, Payload)> = Vec::new();
        // the ghosts stand where the ball blasted (the rings and the glove are drawn after the grade)
        for gh in self.stands(p, t) {
            let dz = cam.depth(&[scene::to_world(gh.x, gh.z, R)])[0];
            let ghost = GhostAt {
                x: gh.x,
                z: gh.z,
                a: gh.a,
                fl: gh.fl,
            };
            items.push((dz + 0.02, Box::new(Item::Ghost(ghost))));
        }
        // the flung enemy's afterimages
        let tcf = p.fx_clock(t);
        for &(tf, vic) in &self.flung {
            if 0.0 <= tcf - tf && tcf - tf < 0.5 && f.fs[vic].vis {
                for (m, a) in [(1.0, 0.5), (2.0, 0.36), (3.0, 0.22)] {
                    let q = p.fighter(vic, tf.max(py_min(t, p.t_ko) - 0.035 * m)).p;
                    let dz = cam.depth(&[scene::to_world(q[0], q[1], R)])[0];
                    let after = Item::After(vic, q, a * (1.0 - (tcf - tf) / 0.5));
                    items.push((dz + 0.05 * m, Box::new(after)));
                }
            }
        }
        items
    }

    /// Draws the painter items that differ with the bold effects and says so; `false` leaves the item to the
    /// classic code (a ghost and an afterimage are drawn as ever).
    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) -> bool {
        let Some(Item::Hand) = payload.downcast_ref::<Item>() else {
            return false;
        };
        // STOPPED HORN: the copper hand, big and solid: up in front of the bell, jolted out white by a block,
        // sinking as the shield runs down
        let (k, cam, t) = (self.k, f.cam, f.t);
        let st = &f.fs[k];
        let (hk, hf) = self.hand_k(p, t, st);
        let pc = cam.to_cam(&[scene::to_world(st.p[0], st.p[1], R)])[0];
        let s = cam.proj(&[pc])[0];
        let rp = R * cam.f / pc[2] * st.scale;
        let q = f.fs[1 - k].p;
        let so = cam.wp(&[scene::to_world(q[0], q[1], R)])[0];
        let (c, side) = Self::hand_at(s, rp, so, hf, self.hand_sunk(p, t, st.shield));
        let (mut arr, depth) = (fr.array(), fr.depth());
        // (a lighter copper than the classic hand's, so it reads against its own orange ball)
        draw_hand(&mut arr, c, rp * (1.5 + 0.22 * hf), HAND, hk.min(1.0), hf, side);
        fr.set_array(&arr, Some(&depth));
        true
    }

    pub(super) fn bold_glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame) {
        for gh in self.stands(p, f.t) {
            let s = f.cam.wp(&[scene::to_world(gh.x, gh.z, R)])[0];
            fr.glow(
                s.0,
                s.1,
                12.0,
                self.col.map(|c| c * 0.7 + 60.0),
                (0.10 + 0.28 * gh.fl) * gh.a,
            );
        }
    }

    /// The blast arcs of one ring: stacked arcs toward the enemy, the call's three and bold, an echo's two and
    /// thinner.
    fn blast_arcs(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, r: &RingAt, enemy: [f64; 2]) {
        let dir = dmath::atan2(enemy[1] - r.z, enemy[0] - r.x);
        let col = p.col[r.owner];
        let (arcs, fade): (&[(f64, f64, f64, V3)], f64) = if r.j == 0 {
            (
                &[
                    (0.0, 0.75, 3.6, bold::pale(GOLD)),
                    (0.17, 0.58, 2.8, GOLD),
                    (0.34, 0.42, 2.2, col),
                ],
                1.0,
            )
        } else {
            (
                &[(0.0, 0.5, 2.4, bold::pale(col)), (0.15, 0.36, 1.8, col)],
                dmath::pow(0.88, (r.j - 1) as f64),
            )
        };
        for &(back, half, w, shade) in arcs {
            let rr = r.r - back;
            if rr > R * 0.55 {
                px::ink_polyline(
                    img,
                    &arc_pts(cam, [r.x, r.z], rr, dir, half, 0.32),
                    w,
                    shade,
                    r.a * fade,
                );
            }
        }
    }

    /// The glove drawn giant. Cocked, its cuff stays where it was (it grows toward the enemy) and it trembles
    /// harder than the fight's own wobble; flicked, its fingertip stays where it was (on the enemy's surface) and
    /// it is only as much bigger as leaves its cuff outside the horn's ball.
    fn giant(g: Glove) -> Glove {
        if g.pose < 0.5 {
            let scale = g.scale * GIANT;
            Glove {
                scale,
                d: g.d + 0.66 * (scale - g.scale),
                wob: g.wob * 2.2,
                ..g
            }
        } else {
            let tip = g.d + 0.64 * g.scale;
            let scale = ((tip - R * 0.85) / 1.3).clamp(g.scale, g.scale * GIANT);
            Glove {
                scale,
                d: tip - 0.64 * scale,
                ..g
            }
        }
    }

    /// FINAL FLICK's glove, crisp and copper: the spring, the brass cuff, the hand and the flicking finger.
    fn bold_glove(img: &mut Pic, cam: &Cam, g: &Glove, col: V3) {
        let (parts, stitches) = glove_world(g);
        let u = unit(g.aim);
        let nrm = [-u[1], u[0]];
        let (s0, s1) = (R * 0.85, g.d - 0.66 * g.scale);
        if s1 > s0 + 0.02 {
            let ncoil = 9;
            let coil: Vec<(f64, f64)> = (0..2 * ncoil + 1)
                .map(|j| {
                    let along = s0 + (s1 - s0) * j as f64 / (2 * ncoil) as f64;
                    let off = if 0 < j && j < 2 * ncoil {
                        0.13 * g.scale * if j % 2 == 1 { 1.0 } else { -1.0 }
                    } else {
                        0.0
                    };
                    bold::at(
                        cam,
                        g.p[0] + u[0] * along + nrm[0] * off,
                        g.p[1] + u[1] * along + nrm[1] * off,
                        R,
                    )
                })
                .collect();
            px::ink_polyline(img, &coil, 2.6, COPPER_DK, 1.0);
            px::polyline(img, &coil, 1.0, bold::pale(COPPER), 1.0);
        }
        let body: V3 = std::array::from_fn(|c| COPPER[c] * 0.8 + col[c] * 0.2);
        for (q, sh, kind) in &parts {
            let base = match kind {
                Part::Cuff => BRASS,
                Part::Finger => bold::pale(body),
                _ => body,
            };
            let shade = base.map(|v| (v * sh * g.light).min(255.0));
            px::ink_poly(img, &cam.wp(q), shade, 1.0);
        }
        for st in &stitches {
            let sp = cam.wp(st);
            px::line(img, sp[0], sp[1], 1.0, COPPER_DK, 1.0);
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let (k, t, cam) = (self.k, f.t, f.cam);
        let o = 1 - k;
        let col = p.col[k];
        let me = drawn.iter().find(|d| d.k == k);
        let foe = drawn.iter().find(|d| d.k == o);
        // ECHO CALL: every ghost's ring drains until it repeats the blast
        for gh in self.stands(p, t) {
            let Some(left) = gh.left else { continue };
            let s = bold::at(cam, gh.x, gh.z, R);
            let rp = R * bold::px_per_unit(cam, gh.x, gh.z, R) * 1.3;
            px::ring(img, s, rp, 1.0, bold::INK, 0.4 * gh.a);
            let a0 = -dmath::PI / 2.0;
            px::ink_arc(img, s, rp, 1.8, a0, a0 + dmath::TAU * left, GOLD, gh.a.min(1.0));
        }
        // the blasts: arcs toward the enemy
        for r in self.rings_at(p, t) {
            self.blast_arcs(p, img, cam, &r, f.fs[o].p);
        }
        let st = &f.fs[k];
        let alive = st.vis && !st.shattered && f.in_fight;
        // STOPPED HORN: a block throws a burst of ticks off the hand
        if alive
            && st.shield > 0.0
            && let (Some(d), Some(e)) = (me, foe)
            && let Some((tb, _, _)) = p.bold_last_block(k, t)
            && t - tb < TICKS
        {
            let u = (t - tb) / TICKS;
            let (c, _) = Self::hand_at(d.s, d.rp, e.s, 1.0, self.hand_sunk(p, t, st.shield));
            for n in 0..7 {
                let a = dmath::TAU * (n as f64 + 0.5) / 7.0;
                let (r0, r1) = (d.rp * (1.0 + 0.5 * u), d.rp * (1.35 + 0.75 * u));
                let (ca, sa) = (dmath::cos(a), dmath::sin(a));
                px::ink_line(
                    img,
                    (c.0 + ca * r0, c.1 + sa * r0 * 0.9),
                    (c.0 + ca * r1, c.1 + sa * r1 * 0.9),
                    2.0,
                    WHITE,
                    1.0 - u,
                );
            }
        }
        // HORN RIP: gold streaks flicker round the ball, a burst of them as it starts
        if alive
            && st.haste > 0.0
            && let Some(d) = me
        {
            let pop = bold::since(p, k, &["ability:haste"], t, RIP_POP).map_or(0.0, |(age, _)| 1.0 - age / RIP_POP);
            let seed = (f.f / 2) as u64;
            for n in 0..7u64 {
                let a0 = dmath::TAU * px::rnd(seed, n);
                let r = d.rp * (1.45 + 0.5 * px::rnd(seed, n + 20) + 0.5 * pop);
                let sweep = 0.3 + 0.45 * px::rnd(seed, n + 40) + 0.5 * pop;
                let shade = if n % 3 == 0 { bold::pale(col) } else { GOLD };
                px::ink_arc(img, d.s, r, 1.6 + 1.2 * pop, a0, a0 + sweep, shade, 0.9);
            }
        }
        // FINAL FLICK: the glove, bright over the dimmed ring
        if let Some(g) = self.glove_at(p, t).filter(|g| g.scale > 0.02) {
            let wind = bold::until(p, k, &["flick"], t, p.show.grid.beat).map(|(u, _)| u);
            let g = Self::giant(g);
            Self::bold_glove(img, cam, &g, col);
            let au = unit(g.aim);
            let at = |along: f64, across: f64| {
                bold::at(
                    cam,
                    g.p[0] + au[0] * (g.d + along * g.scale) - au[1] * across * g.scale,
                    g.p[1] + au[1] * (g.d + along * g.scale) + au[0] * across * g.scale,
                    R,
                )
            };
            if let (Some(u), true) = (wind, g.pose < 0.5) {
                // tremble marks either side of it, and the glint on the curled finger as the flick nears
                if f.f.is_multiple_of(2) {
                    for side in [1.0, -1.0] {
                        for (n, along) in [-0.25, 0.05].into_iter().enumerate() {
                            let out = 0.36 + 0.05 * n as f64 + 0.08 * u;
                            px::ink_line(
                                img,
                                at(along - 0.07, side * out),
                                at(along + 0.07, side * (out + 0.03)),
                                1.6,
                                WHITE,
                                0.5 + 0.5 * u,
                            );
                        }
                    }
                }
                if u > 0.45 {
                    px::glint(
                        img,
                        at(0.24, -0.05),
                        4.0 + 11.0 * bold::ease_in((u - 0.45) / 0.55),
                        WHITE,
                        1.0,
                    );
                }
            } else if g.pose > 0.5
                && let Some((age, _)) = bold::since(p, k, &["flick"], t, SWOOSH)
            {
                // the flick: a white swoosh round the fingertip
                let u = age / SWOOSH;
                let (tip, back) = (at(0.64, -0.065), at(0.2, -0.065));
                px::speed_lines(img, tip, 5.0, (tip.0 - back.0, tip.1 - back.1), 30.0, 6, WHITE, 1.0 - u);
                let rp = bold::px_per_unit(cam, g.p[0], g.p[1], R) * 0.3 * (1.0 + u);
                let dir = dmath::atan2(tip.1 - back.1, tip.0 - back.0);
                px::ink_arc(img, tip, rp, 2.6, dir - 1.1, dir + 1.1, WHITE, 1.0 - u);
            }
        }
        // the flicked enemy rockets off on a copper streak, rope to rope
        let tcf = p.fx_clock(t);
        for &(tf, vic) in &self.flung {
            let age = tcf - tf;
            if !(0.0..FLUNG).contains(&age) || !f.fs[vic].vis || f.fs[vic].shattered {
                continue;
            }
            let Some(d) = drawn.iter().find(|d| d.k == vic) else {
                continue;
            };
            let fade = 1.0 - bold::ease_in(age / FLUNG);
            let now = py_min(t, p.t_ko);
            let steps = 8;
            let way: Vec<(f64, f64)> = (0..=steps)
                .map(|n| {
                    let back = STREAK.min(age) * (1.0 - n as f64 / steps as f64);
                    let q = p.fighter(vic, now - back).p;
                    bold::at(cam, q[0], q[1], R)
                })
                .collect();
            for (n, pair) in way.windows(2).enumerate() {
                // (not over the ball itself)
                if dmath::hypot(pair[1].0 - d.s.0, pair[1].1 - d.s.1) < d.rp * 0.9 {
                    break;
                }
                let w = d.rp * 1.5 * (n as f64 + 1.0) / steps as f64;
                let shade = if n + 2 >= steps { bold::pale(col) } else { col };
                px::line(
                    img,
                    pair[0],
                    pair[1],
                    w,
                    shade,
                    0.75 * fade * (n as f64 + 1.0) / steps as f64,
                );
            }
            let dir = (way[steps].0 - way[steps - 2].0, way[steps].1 - way[steps - 2].1);
            if dmath::hypot(dir.0, dir.1) > 1.5 {
                px::speed_lines(img, d.s, d.rp, dir, d.rp * 2.0, 7, WHITE, fade);
            }
        }
    }
}
