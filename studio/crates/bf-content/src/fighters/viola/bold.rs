//! VIOLA with the bold effects (`picture.effects = "bold"`; the owner's picks 38a to 38d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **COUNTING RESTS** (38a): rest pips over the ball light up, one a beat (the newest pops); through the last
//!   rest a dashed lane shows where the note will roll; the whole note is a wheel with a thick teal rim that
//!   leaves a tread on the canvas, swells as it crushes and sends a wobbling string ring out from the spot.
//! - **DOUBLE STOP** (38b): two dashed lanes before, two bright lanes as the notes roll out, two crushes.
//! - **MELODY AT LAST** (38c): a warm spotlight (a pool on the canvas and a beam from above), a single pip,
//!   eighth notes floating up off the ball and a ring that drains with the time left.
//! - **VIOLA JOKE** (38d): two stick taps and a cymbal over the viola, smiles bobbing in the crowd; the enemy
//!   frozen in brackets, blushing, a sweat drop running down; a closing target under the biggest note, which
//!   drops with a bright rim and speed lines and lands in a quake ring with chips flying.

use bf_core::dmath;
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, Status, StatusKind};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::ss;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::title;
use bf_picture::ui::DrawLog;

use super::{
    DRAW, HOLE_A, HOLE_B, NOTE_A, NOTE_B, NoteItem, NoteView, THICK, ViolaLook, draw_note, hole_tilt, note_axes,
    note_centre, oval,
};

/// The spotlight of MELODY AT LAST, as canvas light and as paint.
const WARM_LIGHT: V3 = [1.5, 1.4, 1.1];
const WARM: V3 = [255.0, 244.0, 214.0];
/// The freeze brackets.
const ICE: V3 = [200.0, 232.0, 255.0];
/// The frozen enemy's blush and its sweat drop.
const BLUSH: V3 = [255.0, 120.0, 150.0];
const SWEAT: V3 = [130.0, 200.0, 255.0];
/// A drumstick.
const STICK: V3 = [214.0, 170.0, 110.0];
/// Seconds a launched note's lane stays lit, and a crush's ring runs.
const LANE_S: f64 = 0.5;
const CRUSH_S: f64 = 0.45;
/// Seconds the pips take to fly off as the note fires.
const PIPS_OFF_S: f64 = 0.3;
/// Seconds the rimshot shows.
const RIMSHOT_S: f64 = 0.9;

/// The outer oval and the hole of a note on its front face, as closed world polylines.
fn note_rings(cam: &Cam, n: &NoteView, scale: f64) -> (Vec<V3>, Vec<V3>) {
    let (a, b, nrm) = note_axes(cam);
    let rr = n.r * scale * DRAW;
    let cc = note_centre(n.c, rr, n.phi, n.lift, b);
    let th = THICK * rr / 2.0;
    let ring = |pts: Vec<[f64; 2]>| -> Vec<V3> {
        let mut out: Vec<V3> = pts
            .iter()
            .map(|q| std::array::from_fn(|i| cc[i] + a[i] * q[0] * rr + b[i] * q[1] * rr + nrm[i] * th))
            .collect();
        out.push(out[0]);
        out
    };
    (
        ring(oval(NOTE_A, NOTE_B, 0.0, n.phi)),
        ring(oval(HOLE_A, HOLE_B, hole_tilt(), n.phi)),
    )
}

/// An outlined eighth note about 13 pixels high at `c` (the tune the viola finally gets).
fn eighth(img: &mut Pic, c: (f64, f64), col: V3, a: f64) {
    let (top, flag) = ((c.0 + 3.0, c.1 - 9.0), (c.0 + 8.0, c.1 - 5.0));
    let head = (c.0, c.1 + 2.0);
    px::line(img, (c.0 + 3.0, c.1 + 2.0), top, 4.0, bold::INK, a);
    px::line(img, top, flag, 4.0, bold::INK, a);
    px::disc(img, head, 4.6, bold::INK, a);
    px::line(img, (c.0 + 3.0, c.1 + 2.0), top, 2.0, col, a);
    px::line(img, top, flag, 2.0, col, a);
    px::disc(img, head, 3.6, col, a);
}

/// A flat ellipse of half-sizes `rx`, `ry` round `c`, outlined.
fn ellipse(img: &mut Pic, c: (f64, f64), rx: f64, ry: f64, col: V3, a: f64) {
    let pts: Vec<(f64, f64)> = (0..14)
        .map(|i| {
            let ang = i as f64 / 14.0 * dmath::TAU;
            (c.0 + rx * dmath::cos(ang), c.1 + ry * dmath::sin(ang))
        })
        .collect();
    px::ink_poly(img, &pts, col, a);
}

impl ViolaLook {
    /// The lane of the note of a `note` event: from where it appears, along its aim, as far as it rolls in its
    /// whole note or to the ropes.
    fn lane_of(p: &PictureCtx, k: usize, e: &Event) -> ([f64; 2], [f64; 2], f64) {
        let w = &p.cfg[k].raw["weapon"];
        let roll = w["roll"].as_f64().unwrap_or(2.0);
        let hold = w["hold"].as_f64().unwrap_or(4.0);
        let (a, aim, r) = ([e.f(4), e.f(5)], e.f(6), e.f(7));
        let u = [dmath::cos(aim), dmath::sin(aim)];
        let lim = scene::A - r;
        let mut len = roll * hold * p.show.grid.beat;
        for ax in 0..2 {
            if u[ax].abs() > 1e-9 {
                let wall = if u[ax] > 0.0 { lim } else { -lim };
                len = len.min(((wall - a[ax]) / u[ax]).max(0.0));
            }
        }
        (a, [a[0] + u[0] * len, a[1] + u[1] * len], r)
    }

    /// The MELODY AT LAST on at `t`: when it began and when it ends.
    fn melody_on(&self, p: &PictureCtx, t: f64) -> Option<(f64, f64)> {
        let tt = t.min(p.t_ko);
        self.melodies
            .iter()
            .copied()
            .find(|&(t0, until)| t0 <= tt && tt < until)
    }

    /// How many pips the count has: the rests before a note (one under MELODY AT LAST).
    fn pips(p: &PictureCtx, k: usize, melody: bool) -> i64 {
        let raw = &p.cfg[k].raw;
        if melody {
            raw["abilities"]["unlock2"]["rest"].as_f64().unwrap_or(1.0) as i64
        } else {
            raw["weapon"]["every"].as_f64().unwrap_or(4.0) as i64 - 1
        }
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let (k, t) = (self.k, c.t);
        let g = &p.show.grid;
        let tint = bold::light(self.col);
        let st = &c.fs[k];
        let (resting, _, _) = self.rest_state(p, t);
        let mk = self.melody_k(p, t);
        // the resting viola's pool, pulsing on each count
        if resting || mk > 0.0 {
            let ph = ((t - g.t_fight) / g.beat).rem_euclid(1.0);
            floor::disc(ic, c, st.p[0], st.p[1], R * 1.5, tint, 0.10 * (1.0 + mk));
            floor::ring(
                ic,
                c,
                st.p[0],
                st.p[1],
                R * (1.15 + 0.7 * ph),
                0.07,
                tint,
                (1.0 - ph) * if resting { 0.6 } else { 0.4 },
            );
        }
        // MELODY AT LAST: the spotlight's pool
        if mk > 0.0 {
            floor::disc(ic, c, st.p[0], st.p[1], R * 2.5, WARM_LIGHT, 0.34 * mk);
            floor::ring(ic, c, st.p[0], st.p[1], R * 2.5, 0.06, WARM_LIGHT, 0.55 * mk);
        }
        // through the last rest: a dashed lane where each coming note will roll (two for a DOUBLE STOP)
        for e in bold::events_in(p, k, &["note"], t, t + g.beat).filter(|e| e.t > t && e.i(8) != 2) {
            let u = 1.0 - (e.t - t) / g.beat;
            let (a, b, r) = Self::lane_of(p, k, e);
            floor::dashes(ic, c, a, b, r * 0.45, 0.4, t * 3.0, tint, 0.25 + 0.5 * u);
        }
        // as it rolls out: its lane, lit and fading
        for e in bold::events_in(p, k, &["note"], t - LANE_S, t).filter(|e| e.i(8) != 2) {
            let fade = 1.0 - (t - e.t) / LANE_S;
            let (a, b, r) = Self::lane_of(p, k, e);
            floor::lane(ic, c, a, b, r * 0.55, tint, 0.6 * fade);
        }
        // the tread a rolling note leaves behind it
        let fi = p.sim_i(t.min(p.t_ko));
        let back = (0.3 * f64::from(g.fps)) as usize;
        for d in self.tracks(p) {
            if !(d.f0 <= fi && fi <= d.f1) || t < d.tl {
                continue;
            }
            let j = fi - d.f0;
            let j0 = j
                .saturating_sub(back)
                .max(((d.tl - d.t0) * f64::from(g.fps)) as usize)
                .min(j);
            floor::lane(ic, c, d.p[j0], d.p[j], d.r * 0.3, tint, 0.3);
        }
        // a crush: a string ring wobbles out from the spot (the big note's quake is in `bold_canvas_after`)
        for e in bold::events_in(p, k, &["hit:note"], t - CRUSH_S, t).filter(|e| e.i(8) != 2) {
            let u = (t - e.t) / CRUSH_S;
            floor::band(
                ic,
                c,
                Family::Strings,
                e.f(5),
                e.f(6),
                0.3 + 1.4 * bold::ease_out(u),
                1.0 - u,
                t,
                tint,
                1.1,
            );
        }
    }

    pub(super) fn bold_canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let t = c.t;
        if t >= p.t_hs {
            return;
        }
        let tint = bold::light(self.col);
        for &(t0, _, tl, x, z) in &self.jokes {
            if t0 <= t && t < tl {
                // VIOLA JOKE: the target closes in under the falling note, its shadow growing
                let u = (t - t0) / (tl - t0);
                let rad = self.low_size * p.run.f("BL", p.sim_i(t0), self.k).expect("BL");
                floor::shade(ic, c, x, z, rad * (0.5 + 0.7 * u), 0.55 * u);
                floor::target(ic, c, x, z, rad, u, tint);
            } else if (0.0..0.6).contains(&(t - tl)) {
                // it lands: the canvas quakes
                let u = (t - tl) / 0.6;
                floor::band(ic, c, Family::Quake, x, z, 0.9 + 2.3 * u, 1.0 - u, t, tint, 1.2);
                floor::band(ic, c, Family::Strings, x, z, 0.5 + 1.5 * u, 1.0 - u, t, tint, 1.0);
            }
        }
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(NoteItem(n)) = payload.downcast_ref::<NoteItem>() else {
            return;
        };
        let col = p.col[n.owner];
        let glow = n.glow.max(0.5 * self.melody_k(p, f.t));
        // it swells as it appears and as it crushes
        let sc = n.sc * (1.0 + 0.16 * n.glow);
        draw_note(fr, f.cam, f.l, n.c, n.r, n.phi, col, n.lift, sc, glow);
        // the wheel's thick rim (white for the moment it crushes) and the edge of its hole
        let (outer, hole) = note_rings(f.cam, n, sc);
        let rim = if n.glow > 0.45 { bold::WHITE } else { col };
        bold::thick_line(fr, f.cam, &outer, rim, 3);
        bold::thin_line(fr, f.cam, &hole, bold::pale(col));
    }

    pub(super) fn bold_after_post(
        &self,
        p: &PictureCtx,
        img: &mut Img<u8>,
        log: &mut DrawLog,
        f: &FrameCtx<'_>,
        drawn: &[DrawnBall],
    ) {
        // in the fight the pips and the floating notes are drawn in `bold_after_grade`; the champion's solo
        // keeps its eighths
        if f.t >= p.t_hs && title::door_offset(&p.show.grid, f.t) != 0.0 {
            self.draw_rests(p, img, log, f.t, drawn);
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, t) = (self.k, f.t);
        let g = &p.show.grid;
        let col = self.col;
        let pale = bold::pale(col);
        if let Some(d) = drawn.iter().find(|d| d.k == k)
            && !f.fs[k].shattered
        {
            let ((sx, sy), rp) = (d.s, d.rp);
            let (resting, count, mel) = self.rest_state(p, t);
            let melody = self.melody_on(p, t);
            if let Some((t0, until)) = melody {
                let mk = self.melody_k(p, t);
                // the spotlight's beam from above
                let (top, low) = (sy - rp - 170.0, sy + rp * 0.55);
                px::poly(
                    img,
                    &[
                        (sx - rp * 0.45, top),
                        (sx + rp * 0.45, top),
                        (sx + rp * 1.7, low),
                        (sx - rp * 1.7, low),
                    ],
                    WARM,
                    0.13 * mk,
                );
                // eighth notes float up off the ball, one every half beat
                let period = g.beat / 2.0;
                let newest = ((t - t0) / period).floor() as i64;
                for i in (newest - 3).max(0)..=newest {
                    let age = t - (t0 + i as f64 * period);
                    let side = if i % 2 == 0 { 1.0 } else { -1.0 };
                    let x = sx + side * (rp * 0.55 + 4.0 * (i % 3) as f64) + 5.0 * dmath::sin(age * 7.0 + i as f64);
                    let y = sy - rp * 0.5 - 52.0 * age;
                    eighth(
                        img,
                        (x, y),
                        if i % 3 == 0 { bold::WHITE } else { pale },
                        1.0 - ss(0.55, 0.9, age),
                    );
                }
                // the time left, draining round the ball
                let left = ((until - t) / (until - t0).max(1e-6)).clamp(0.02, 1.0);
                let a0 = -dmath::PI / 2.0;
                px::ink_arc(img, d.s, rp + 4.0, 1.6, a0, a0 + dmath::TAU * left, pale, 0.95);
            }
            // the rest pips over the ball: one lights up a beat, the newest pops
            let total = Self::pips(p, k, mel || melody.is_some()).max(count).max(1);
            let pip_at = |j: i64| (sx + (j as f64 - (total - 1) as f64 / 2.0) * 14.0, sy - rp - 13.0);
            if resting && count > 0 {
                let t_beat = g.t_fight + ((t - g.t_fight) / g.beat + 1e-6).floor() * g.beat;
                let pop = 1.0 - ((t - t_beat) / 0.14).clamp(0.0, 1.0);
                for j in 0..total {
                    let c = pip_at(j);
                    if j >= count {
                        px::ink_disc(img, c, 3.4, bold::deep(col), 0.9);
                    } else if j == count - 1 {
                        px::ink_disc(img, c, 5.0 + 3.5 * pop, if pop > 0.3 { bold::WHITE } else { pale }, 1.0);
                        px::disc(img, c, 1.8, bold::WHITE, 1.0);
                    } else {
                        px::ink_disc(img, c, 5.0, pale, 1.0);
                    }
                }
                // the last rest: a glint, the note is coming
                if count == total
                    && let Some((u, _)) = bold::until(p, k, &["note"], t, g.beat * 0.5)
                {
                    px::glint(img, (sx, sy - rp - 13.0 - 12.0), 4.0 + 5.0 * u, bold::WHITE, 0.9);
                }
            } else if let Some((age, _)) = bold::since(p, k, &["note"], t, PIPS_OFF_S).filter(|(_, e)| e.i(8) != 2) {
                // the note fires: the pips fly off
                let u = age / PIPS_OFF_S;
                for j in 0..total {
                    let c = pip_at(j);
                    px::ink_disc(
                        img,
                        (c.0 + (c.0 - sx) * 0.8 * u, c.1 - 16.0 * bold::ease_out(u)),
                        5.0,
                        pale,
                        1.0 - u,
                    );
                }
            }
            // VIOLA JOKE's rimshot over the viola: two stick taps and a cymbal
            for &(t0, ..) in self.jokes.iter().filter(|j| j.0 <= t && t < j.0 + RIMSHOT_S) {
                let age = t - t0;
                let fade = 1.0 - ss(RIMSHOT_S - 0.3, RIMSHOT_S, age);
                let base = (sx + rp * 0.2, sy - rp - 34.0);
                for (n, dx) in [(0.0, -22.0), (1.0, -6.0)] {
                    let a = age - 0.11 * n;
                    if a < 0.0 {
                        continue;
                    }
                    let c = (base.0 + dx, base.1 + 6.0);
                    let down = bold::ease_out(a / 0.06);
                    px::ink_line(
                        img,
                        (c.0 - 9.0 + 3.0 * down, c.1 - 16.0 + 6.0 * down),
                        (c.0 - 1.0, c.1 - 2.0 * down),
                        2.0,
                        STICK,
                        fade,
                    );
                    let r = 4.0 + 5.0 * bold::ease_out(a / 0.1);
                    px::ink_star(img, c, r, r * 0.4, 4, 0.4, bold::WHITE, fade * (1.0 - ss(0.2, 0.5, a)));
                }
                let a = age - 0.22;
                if a >= 0.0 {
                    let c = (
                        base.0 + 16.0,
                        base.1 + 2.0 + 3.0 * dmath::sin(a * 40.0) * (1.0 - a / 0.7).max(0.0),
                    );
                    let grow = bold::ease_out(a / 0.1);
                    for i in 0..7 {
                        let ang = -dmath::PI * (0.08 + 0.84 * f64::from(i) / 6.0);
                        let (r0, r1) = (13.0, 13.0 + 9.0 * grow);
                        px::ink_line(
                            img,
                            (c.0 + r0 * dmath::cos(ang), c.1 + r0 * 0.6 * dmath::sin(ang)),
                            (c.0 + r1 * dmath::cos(ang), c.1 + r1 * 0.6 * dmath::sin(ang)),
                            1.4,
                            bold::GOLD,
                            fade * (1.0 - ss(0.25, 0.6, a)),
                        );
                    }
                    ellipse(img, c, 11.0, 3.6, bold::GOLD, fade);
                    px::disc(img, c, 1.6, bold::INK, fade);
                }
            }
        }
        // the crowd laughs: smiles bobbing round the ring
        let laugh = self.laugh_k(t);
        if laugh > 0.0 {
            for i in 0..18u64 {
                let ang = px::rnd(38, i) * dmath::TAU;
                let far = scene::BARRIER_R + 0.5 + 2.2 * px::rnd(39, i);
                let c = bold::at(
                    f.cam,
                    far * dmath::cos(ang),
                    far * dmath::sin(ang),
                    0.9 + 1.2 * px::rnd(40, i),
                );
                let bob = 3.0 * dmath::sin(t * 17.0 + i as f64 * 1.7).abs();
                let c = (c.0, c.1 - bob);
                px::ink_arc(img, c, 4.5, 1.6, 0.12 * dmath::PI, 0.88 * dmath::PI, bold::GOLD, laugh);
            }
        }
        for &(t0, v, tl, x, z) in &self.jokes {
            if !(t0 <= t && t < tl + 0.6) {
                continue;
            }
            // the frozen enemy: brackets lock on, it blushes, a sweat drop runs down
            let frz = self.frz(p, v, t);
            if frz > 0.0
                && let Some(d) = drawn.iter().find(|d| d.k == v)
            {
                let ((ex, ey), rp) = (d.s, d.rp);
                let lock = 1.0 - bold::ease_out((t - t0) / 0.18);
                px::brackets(img, d.s, rp * (1.4 + 0.7 * lock), 1.8, ICE, frz);
                for side in [-1.0, 1.0] {
                    ellipse(
                        img,
                        (ex + side * rp * 0.5, ey + rp * 0.2),
                        rp * 0.24,
                        rp * 0.15,
                        BLUSH,
                        0.85 * frz,
                    );
                }
                let run = ((t - t0) / 0.45).rem_euclid(1.0);
                let s = (ex + rp * 0.86, ey - rp * 0.75 + rp * 0.7 * run);
                px::ink_poly(
                    img,
                    &[
                        (s.0, s.1 - 6.5),
                        (s.0 + 3.2, s.1),
                        (s.0 + 2.2, s.1 + 3.0),
                        (s.0, s.1 + 4.0),
                        (s.0 - 2.2, s.1 + 3.0),
                        (s.0 - 3.2, s.1),
                    ],
                    SWEAT,
                    frz * (1.0 - ss(0.75, 1.0, run)),
                );
            }
            // the biggest note drops: a bright rim (the ring is dimmed round the viola) and speed lines
            if t < tl {
                for n in self.notes(p, f).iter().filter(|n| n.lift > 0.0) {
                    let (outer, hole) = note_rings(f.cam, n, n.sc);
                    px::ink_polyline(img, &f.cam.wp(&outer), 2.4, pale, 1.0);
                    px::polyline(img, &f.cam.wp(&hole), 1.2, bold::WHITE, 0.9);
                    let top = outer.iter().fold(outer[0], |m, w| if w[1] > m[1] { *w } else { m });
                    let s = f.cam.wp(&[top])[0];
                    let wide = n.r * DRAW * bold::px_per_unit(f.cam, n.c[0], n.c[1], n.lift);
                    for i in 0..5 {
                        let dx = (f64::from(i) - 2.0) / 2.0 * wide * 0.8;
                        let len = 30.0 - 9.0 * (f64::from(i) - 2.0).abs();
                        px::line(
                            img,
                            (s.0 + dx, s.1 - 4.0),
                            (s.0 + dx, s.1 - 4.0 - len),
                            1.4,
                            bold::WHITE,
                            0.8,
                        );
                    }
                }
            } else {
                // it lands: ink and teal chips fly
                let c = bold::at(f.cam, x, z, 0.15);
                let age = t - tl;
                px::chips(img, c, age, 0.5, 16, 58.0, (tl * 1000.0) as u64, pale);
                px::chips(img, c, age, 0.4, 9, 36.0, (tl * 1000.0) as u64 + 7, [70.0, 76.0, 96.0]);
            }
        }
    }

    pub(super) fn bold_statuses(&self, p: &PictureCtx, t: f64) -> Vec<Status> {
        let tt = t.min(p.t_ko);
        self.stuns
            .iter()
            .filter(|&&(t0, _, d)| t0 <= tt && tt < t0 + d)
            .map(|&(t0, who, d)| Status {
                who,
                kind: StatusKind::Stun,
                left: Some(1.0 - (tt - t0) / d.max(1e-6)),
            })
            .collect()
    }

    /// A whole note's crush is a solid hit whatever its damage; VIOLA JOKE's big note is a heavy one.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        if bold::how(e) != "note" {
            return Some(by_damage);
        }
        Some(if e.i(8) == 2 {
            HitSize::Heavy
        } else {
            by_damage.max(HitSize::Solid)
        })
    }
}
