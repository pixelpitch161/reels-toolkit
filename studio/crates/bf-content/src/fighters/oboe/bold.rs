//! OBOE with the bold effects (`picture.effects = "bold"`; the owner's picks 27a to 27d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **TUNING A** (27a): while the aim locks a dashed line runs out along it and a glint gathers at the reed; the
//!   A is a needle-straight beam, wide at the reed and a point at its end, with ping rings running out along it
//!   and a double ping where it lands. The enemy it knocks out of tune goes sour green with the ring's tuner
//!   gauge over it (S12), a wobbling sour ring under it and a harder wobble of its outline; every sting pops a
//!   little sour note off it.
//! - **DOUBLE REED** (27b): a wedge on the floor shows the bite while two big cane blades scissor open on a
//!   green rivet; they snap shut and hold with a glint at the tips, and a hit leaves a white X slashed across
//!   the enemy.
//! - **ENDLESS NOTE** (27c): the beam flares as it switches on and stays a thick straight bar that follows the
//!   enemy, bright pulses running down it; a ring round the oboe drains with the time left and ticks on every
//!   sting.
//! - **ILL WIND** (27d): sour streaks are drawn in to the ball as it breathes in; the gust rolls out as sickly
//!   green puffed rings with curls riding the front; an enemy fully out of tune has its gauge needle pinned at
//!   the red stop.

use bf_core::dmath;
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, Status, StatusKind, WHITE};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::Wave;
use bf_picture::scene::{self, A, R_BALL as R};

use super::{BEAM_Y, OboeLook, draw_reed, extra, unit, wobble_rim};
use crate::fighters::shared::reeds::{self, CANE, CANE_D, THREAD};
use bf_core::roster::FighterCfg;

/// The sickly green of everything out of tune.
const SOUR_G: V3 = [176.0, 222.0, 64.0];
/// The tuner's needle.
const RED: V3 = [255.0, 96.0, 80.0];
/// The white of a dial.
const DIAL: V3 = [240.0, 240.0, 244.0];
/// From this depth the enemy is fully out of tune and its needle is pinned.
const PINNED: f64 = 0.95;
/// Seconds DOUBLE REED's blades stay shut where they snapped, and then take to draw back.
const SHUT_S: f64 = 0.1;
const BACK_S: f64 = 0.15;
/// Seconds the X slash of a snip shows.
const SLASH_S: f64 = 0.4;
/// Seconds ILL WIND breathes in before the gust.
const INHALE_S: f64 = 0.45;
/// Seconds a sting's little note shows.
const STING_S: f64 = 0.2;

/// One blade of DOUBLE REED as a world polygon: hinged on the ball's rim at `aim`, swung `sgn * opening` off it,
/// `length` long, its straight cutting edge toward the aim.
fn blade(p: [f64; 2], aim: f64, sgn: f64, opening: f64, length: f64) -> (Vec<V3>, [V3; 2]) {
    let ua = unit(aim);
    let d = unit(aim + sgn * opening);
    let n = [-d[1] * sgn, d[0] * sgn];
    let b = [p[0] + ua[0] * R * 0.9, p[1] + ua[1] * R * 0.9];
    let y = BEAM_Y + 0.04 * sgn;
    let pt =
        |along: f64, out: f64| scene::to_world(b[0] + d[0] * along + n[0] * out, b[1] + d[1] * along + n[1] * out, y);
    (
        vec![
            pt(0.0, -0.03),
            pt(length, 0.0),
            pt(length * 0.85, 0.17),
            pt(length * 0.4, 0.27),
            pt(0.0, 0.12),
        ],
        [pt(0.08, 0.0), pt(length * 0.97, 0.01)],
    )
}

/// A small curl of wind at the screen point `c`, turned by `rot`.
fn curl(img: &mut Pic, c: (f64, f64), r: f64, rot: f64, col: V3, a: f64) {
    px::ink_arc(img, c, r, 1.6, rot, rot + 4.4, col, a);
    let q = (c.0 + dmath::cos(rot) * r * 0.45, c.1 + dmath::sin(rot) * r * 0.45);
    px::arc(img, q, r * 0.5, 1.4, rot + 2.6, rot + 6.4, col, a);
}

/// A ring of wind on the floor after the grade: twelve arcs of the circle of radius `r` round `c0`, turned by
/// `rot`, left out where they would be outside the ropes.
fn wind_ring(img: &mut Pic, cam: &Cam, c0: [f64; 2], r: f64, rot: f64, col: V3, a: f64) {
    for seg in 0..12 {
        let pts: Vec<(f64, f64)> = (0..=3)
            .filter_map(|i| {
                let ang = rot + dmath::TAU * f64::from(seg * 4 + i) / 48.0;
                let (x, z) = (c0[0] + r * dmath::cos(ang), c0[1] + r * dmath::sin(ang));
                (x.abs() <= A && z.abs() <= A).then(|| bold::at(cam, x, z, 0.05))
            })
            .collect();
        if pts.len() >= 2 {
            px::ink_polyline(img, &pts, 1.8, col, a);
        }
    }
}

/// The roster numbers the bold moves read, taken in `setup_as` from its `cfg`: for its own side that is
/// `p.cfg[k]`, and EUPHONIUM's copy of this look gets the copied fighter's numbers.
#[derive(Clone, Debug, Default)]
pub(super) struct Nums {
    /// ILL WIND's gust: its speed and radius.
    gust: (f64, f64),
}

impl Nums {
    /// The numbers of the roster entry `cfg`.
    pub(super) fn of(cfg: &FighterCfg) -> Nums {
        Nums {
            gust: (
                cfg.low.get("speed").unwrap_or(4.5),
                cfg.low.get("radius").unwrap_or(9.0),
            ),
        }
    }
}

impl OboeLook {
    /// The aim lock under way at `t`: how far it is (0 to 1), its aim and the length of the beam to come.
    fn lock(&self, t: f64) -> Option<(f64, f64, f64)> {
        let &(t0, aim) = self.locks.iter().rev().find(|&&(t0, _)| t0 <= t)?;
        let nxt = self.tunes.iter().find(|q| q.t >= t0).filter(|q| t < q.t)?;
        Some(((t - t0) / (nxt.t - t0).max(1e-6), aim, nxt.len))
    }

    /// DOUBLE REED's bold blades at `t`: the aim, the opening in radians, the blades' length and the seconds
    /// since the snap (negative while they open: minus the share of the opening still to come).
    fn bold_snip(&self, p: &PictureCtx, t: f64) -> Option<(f64, f64, f64, f64)> {
        let k = self.k;
        let (half, reach) = (self.snip_half?, self.snip_reach?);
        let &(t0, t_snap, aim) = self
            .snip_open
            .iter()
            .find(|&&(t0, t_snap, _)| t0 <= t && t < t_snap + SHUT_S + BACK_S && t0 < p.t_ko)?;
        let half = half * dmath::PI / 180.0;
        if t < t_snap {
            let a = match p.run.row("SNIP", p.sim_i(t.min(p.t_ko)), k) {
                Some(row) if !row[1].is_nan() => row[1],
                _ => aim,
            };
            let u = (t - t0) / (t_snap - t0).max(1e-6);
            // they fly open, then quiver wide
            let quiver = if u > 0.6 { 0.05 * dmath::sin(t * 50.0) } else { 0.0 };
            return Some((
                a,
                half * 0.9 * bold::ease_out(u / 0.6) + quiver,
                0.34 + (reach - 0.34) * bold::ease_out(u / 0.5),
                u - 1.0,
            ));
        }
        // shut where they snapped (the `snip` event has the aim), then back in
        let a = bold::since(p, k, &["snip"], t, SHUT_S + BACK_S + 0.05).map_or(aim, |(_, e)| e.f(4));
        let v = t - t_snap;
        let back = ((v - SHUT_S) / BACK_S).clamp(0.0, 1.0);
        Some((a, 0.0, reach + (0.34 - reach) * back, v))
    }

    /// The enemy's out-of-tune state at `t`: who, the share of its time left and its depth.
    fn sour(&self, p: &PictureCtx, t: f64) -> Option<(usize, f64, f64)> {
        if !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return None;
        }
        let vic = 1 - self.k;
        let row = p.run.row("DET", p.sim_i(t), vic)?;
        let (left, dep) = (row[0], row[1]);
        if left <= 0.0 || dep <= 0.0 {
            return None;
        }
        let dur = bold::since(p, vic, &["detune"], t, 60.0).map_or(left, |(_, e)| e.f(5));
        Some((vic, (left / dur.max(1e-6)).clamp(0.0, 1.0), dep))
    }

    /// ILL WIND's gust at `t`: where it started, its radius now and how far it has faded (1 fresh, 0 gone).
    fn gust(&self, p: &PictureCtx, t: f64) -> Option<([f64; 2], f64, f64)> {
        let k = self.k;
        let (speed, radius) = self.nums.gust;
        let (age, e) = bold::since(p, k, &["ability:wave"], t, (radius - R) / speed.max(1e-6))?;
        let r = R + speed * age;
        Some((p.run.p(p.sim_i(e.t), k), r, 1.0 - r / radius.max(1e-6)))
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let tint = bold::light(p.col[k]);
        let sour = bold::light(SOUR_G);
        let me = c.fs[k].p;
        // the aim locks: a dashed line runs out along it
        if c.in_fight
            && c.t < p.t_ko
            && let Some((u, aim, len)) = self.lock(c.t)
        {
            let d = unit(aim);
            let o = [me[0] + d[0] * (R + 0.04), me[1] + d[1] * (R + 0.04)];
            let len = len.min(reeds::ring_exit(o, d, None));
            let end = [o[0] + d[0] * len, o[1] + d[1] * len];
            floor::dashes(ic, c, o, end, 0.06, 0.32, c.t * 5.0, tint, 0.5 + 0.7 * u);
        }
        // the beams' lit lanes
        for b in self.live_beams(p, c.t) {
            let d = unit(b.aim);
            let len = b.len.min(reeds::ring_exit(b.o, d, None));
            let end = [b.o[0] + d[0] * len, b.o[1] + d[1] * len];
            let half = if b.held { 0.2 } else { 0.1 };
            floor::lane(ic, c, b.o, end, half, tint, 0.55 * b.k);
        }
        if !c.in_fight {
            return;
        }
        // the sour ring under a detuned ball: it wobbles like a string out of tune
        for st in c.fs {
            let det = extra(st, "det");
            if det > 0.02 {
                floor::band(
                    ic,
                    c,
                    Family::Strings,
                    st.p[0],
                    st.p[1],
                    R * 1.4,
                    0.5 + 0.5 * det.min(1.0),
                    c.t * 1.7,
                    sour,
                    0.5 + 0.6 * det.min(1.0),
                );
            }
        }
        // DOUBLE REED opening: a wedge on the floor where the blades will bite
        if let Some((aim, _, _, v)) = self.bold_snip(p, c.t)
            && v < 0.0
            && let (Some(half), Some(reach)) = (self.snip_half, self.snip_reach)
        {
            let u = 1.0 + v;
            let half = half * dmath::PI / 180.0;
            for (j, light) in ic.iter_mut().enumerate() {
                let (dx, dz) = (c.sxp[j] - me[0], c.szp[j] - me[1]);
                let rd = dmath::hypot(dx, dz);
                if rd < R * 0.9 || rd > R + reach {
                    continue;
                }
                let da = (dmath::atan2(dz, dx) - aim + dmath::PI).rem_euclid(dmath::TAU) - dmath::PI;
                if da.abs() > half {
                    continue;
                }
                let edge = (half - da.abs()) * rd < 0.07 || rd > R + reach - 0.08;
                let v = if edge { 0.95 } else { 0.22 } * (0.35 + 0.65 * u);
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
        // ILL WIND breathes in: a ring of puffs closes on the ball
        if let Some((u, _)) = bold::until(p, k, &["ability:wave"], c.t, INHALE_S) {
            floor::band(
                ic,
                c,
                Family::Air,
                me[0],
                me[1],
                R + 2.4 * (1.0 - bold::ease_in(u)),
                0.3 + 0.7 * u,
                -c.t,
                sour,
                1.1,
            );
        }
    }

    pub(super) fn bold_canvas_wave(&self, _p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind == "stun" || w.kind == "pulse" {
            return;
        }
        // ILL WIND: sickly green wind rings, the front and two more rolling behind it
        let sour = [0.55, 1.35, 0.15];
        floor::wave(ic, c, Family::Air, w, c.t, sour, 1.6);
        let fade = 1.0 - w.r / w.rmax.max(1e-6);
        for (n, back) in [0.6, 1.2].into_iter().enumerate() {
            if w.r > back + R {
                floor::band(
                    ic,
                    c,
                    Family::Air,
                    w.cx,
                    w.cz,
                    w.r - back,
                    fade * (0.7 - 0.25 * n as f64),
                    c.t + 1.3 * (n + 1) as f64,
                    sour,
                    0.9,
                );
            }
        }
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _payload: &Payload) {
        let k = self.k;
        let st = &f.fs[k];
        let sn = if f.in_fight { self.bold_snip(p, f.t) } else { None };
        let Some((aim, opening, length, _)) = sn else {
            // the reed as it always is
            let (mut arr, depth) = (fr.array(), fr.depth());
            let mut g = if f.in_fight && f.t < p.t_ko {
                extra(st, "lock")
            } else {
                0.0
            };
            if k == p.win && f.t >= p.t_hs {
                g = p.solo_env(f.t);
            }
            draw_reed(&mut arr, f.cam, st.p, st.th, BEAM_Y, 0.0, 0.34, g, p.col[k]);
            fr.set_array(&arr, Some(&depth));
            return;
        };
        // DOUBLE REED: two big cane blades on a green rivet
        for (sgn, col) in [(-1.0, CANE.map(|v| v * 0.8)), (1.0, CANE)] {
            let (poly, edge) = blade(st.p, aim, sgn, opening, length);
            bold::outlined_poly(fr, f.cam, &poly, col);
            fr.line(f.cam, &[poly[4], poly[3], poly[2]], CANE_D, 1);
            fr.line(f.cam, &edge, WHITE, 1);
        }
        let ua = unit(aim);
        let hinge = scene::to_world(st.p[0] + ua[0] * R * 0.93, st.p[1] + ua[1] * R * 0.93, BEAM_Y + 0.05);
        bold::blob(fr, f.cam, hinge, 0.08, THREAD);
    }

    /// One of its beams after the grade: TUNING A's needle with its ping rings, or ENDLESS NOTE's bar with its
    /// pulses. `age` is the seconds since it came on.
    #[allow(clippy::too_many_arguments)]
    fn draw_bold_beam(
        img: &mut Pic,
        cam: &Cam,
        o: [f64; 2],
        aim: f64,
        len: f64,
        k_beam: f64,
        held: bool,
        age: f64,
        col: V3,
    ) {
        let d = unit(aim);
        let len = len.min(reeds::ring_exit(o, d, None));
        if len <= 0.05 || k_beam <= 0.01 {
            return;
        }
        let at = |s: f64| bold::at(cam, o[0] + d[0] * s, o[1] + d[1] * s, BEAM_Y);
        let (a, b) = (at(0.0), at(len));
        let (sx, sy) = (b.0 - a.0, b.1 - a.1);
        let sl = (sx * sx + sy * sy).sqrt().max(1e-6);
        let (nx, ny) = (-sy / sl, sx / sl);
        let pale = bold::pale(col);
        if held {
            // a thick straight bar, flaring as it switches on
            let w = 5.0 + 6.0 * (1.0 - age / 0.14).max(0.0);
            px::ink_line(img, a, b, w, pale, k_beam);
            px::line(img, a, b, 2.0, WHITE, k_beam);
            // pulses run down it
            let gap = 0.8;
            let n = (len / gap) as usize + 1;
            for j in 0..n {
                let s = (age * 5.0).rem_euclid(gap) + gap * j as f64;
                if s < len {
                    px::ink_disc(img, at(s), 3.6, WHITE, k_beam);
                }
            }
            px::glint(img, a, 7.0 + 5.0 * (1.0 - age / 0.14).max(0.0), WHITE, k_beam);
            return;
        }
        // a needle: wide at the reed, a point at its end
        let w0 = 4.5;
        let needle = [(a.0 + nx * w0, a.1 + ny * w0), b, (a.0 - nx * w0, a.1 - ny * w0)];
        px::ink_poly(img, &needle, pale, k_beam);
        px::line(img, a, at(len * 0.85), 1.0, WHITE, k_beam);
        // ping rings run out along it
        for j in 0..4 {
            let s = age * 11.0 - 1.1 * f64::from(j);
            if s > 0.15 && s < len {
                px::ink_ring(img, at(s), 4.5 + 1.2 * f64::from(j), 1.3, pale, k_beam);
            }
        }
        if age < 0.1 {
            px::glint(img, a, 9.0 * (1.0 - age / 0.1) + 3.0, WHITE, 1.0);
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let k = self.k;
        let o = 1 - k;
        let t = f.t;
        let col = p.col[k];
        // OUT OF TUNE outlines, wobbling harder
        for d in drawn {
            let det = (extra(&f.fs[d.k], "det") * 1.6).min(1.6);
            match img {
                Pic::F32(a) => wobble_rim(a, d.s.0, d.s.1, d.rp, det, t, col, d.k as f64),
                Pic::F64(a) => wobble_rim(a, d.s.0, d.s.1, d.rp, det, t, col, d.k as f64),
            }
        }
        if t > p.t_ko {
            return;
        }
        let me = f.fs[k].p;
        let mine = drawn.iter().find(|d| d.k == k);
        let foe = drawn.iter().find(|d| d.k == o);
        // the aim locks: a glint gathers at the reed's tip
        if let Some((u, aim, _)) = self.lock(t) {
            let d = unit(aim);
            let tip = bold::at(f.cam, me[0] + d[0] * (R + 0.34), me[1] + d[1] * (R + 0.34), BEAM_Y);
            px::glint(img, tip, 3.0 + 6.0 * u, bold::pale(col), 0.5 + 0.5 * u);
        }
        // the beams
        for b in self.live_beams(p, t) {
            let age = if b.held {
                self.endless
                    .iter()
                    .rev()
                    .find(|&&(t0, _)| t0 <= t)
                    .map_or(1.0, |&(t0, _)| t - t0)
            } else {
                bold::since(p, k, &["tune"], t, 0.5).map_or(1.0, |(age, _)| age)
            };
            Self::draw_bold_beam(img, f.cam, b.o, b.aim, b.len, b.k, b.held, age, col);
        }
        // the A lands: a double ping where it struck
        for e in bold::events_in(p, k, &["hit:tune"], t - 0.32, t) {
            let u = (t - e.t) / 0.32;
            let sp = bold::at(f.cam, e.f(5), e.f(6), BEAM_Y);
            for (n, r1) in [22.0, 34.0].into_iter().enumerate() {
                let un = (u - 0.2 * n as f64).max(0.0);
                px::ink_ring(img, sp, 5.0 + r1 * bold::ease_out(un), 1.6, SOUR_G, 1.0 - u);
            }
        }
        // every sting pops a little sour note off the enemy
        if let Some(d) = foe {
            for e in bold::events_in(p, k, &["hit:sting", "hit:endless"], t - STING_S, t) {
                let u = (t - e.t) / STING_S;
                let side = if (e.t * 1000.0) as i64 % 2 == 0 { 1.0 } else { -1.0 };
                let at = (
                    d.s.0 + side * d.rp * (0.75 + 0.3 * u),
                    d.s.1 - d.rp * (0.35 + 0.7 * bold::ease_out(u)),
                );
                px::note(img, at, SOUR_G, 1.0 - bold::ease_in(u));
                px::ink_line(
                    img,
                    (d.s.0 + side * d.rp * 0.45, d.s.1 - d.rp * 0.3),
                    (d.s.0 + side * d.rp * 0.8, d.s.1 - d.rp * 0.55),
                    1.4,
                    SOUR_G,
                    1.0 - u,
                );
            }
        }
        // DOUBLE REED: a glint where the tips snap together, and an X slashed across the enemy
        if let Some((aim, _, length, v)) = self.bold_snip(p, t)
            && (0.0..SHUT_S).contains(&v)
        {
            let d = unit(aim);
            let tip = bold::at(
                f.cam,
                me[0] + d[0] * (R * 0.9 + length),
                me[1] + d[1] * (R * 0.9 + length),
                BEAM_Y,
            );
            px::glint(img, tip, 12.0 * (1.0 - v / SHUT_S) + 4.0, WHITE, 1.0);
        }
        if let Some(d) = foe {
            for e in bold::events_in(p, k, &["hit:snip"], t - SLASH_S, t) {
                let u = (t - e.t) / SLASH_S;
                let grow = bold::ease_out(u / 0.15);
                let r = d.rp * 1.25 * grow;
                let a = (1.0 - (u - 0.5) / 0.5).clamp(0.0, 1.0);
                px::ink_cross(img, d.s, r, 4.0, WHITE, a);
                px::cross(img, d.s, r * 0.92, 1.4, bold::pale(col), a);
            }
        }
        // ENDLESS NOTE: a ring round the oboe drains with the time left and ticks on every sting
        if let Some(d) = mine
            && let Some(&(t0, t1)) = self.endless.iter().rev().find(|&&(t0, t1)| t0 <= t && t < t1)
        {
            let left = ((t1 - t) / (t1 - t0).max(1e-6)).clamp(0.0, 1.0);
            let tick = bold::since(p, k, &["hit:endless"], t, 0.12).map_or(0.0, |(age, _)| 1.0 - age / 0.12);
            let a0 = -dmath::PI / 2.0;
            let a1 = a0 + dmath::TAU * left.max(0.02);
            let r = d.rp + 6.0;
            px::ink_arc(img, d.s, r, 2.2 + 1.6 * tick, a0, a1, bold::pale(col), 1.0);
            // the tick: a notch at the ring's running end
            let q0 = (d.s.0 + dmath::cos(a1) * (r - 4.0), d.s.1 + dmath::sin(a1) * (r - 4.0));
            let q1 = (
                d.s.0 + dmath::cos(a1) * (r + 4.0 + 4.0 * tick),
                d.s.1 + dmath::sin(a1) * (r + 4.0 + 4.0 * tick),
            );
            px::ink_line(img, q0, q1, 1.6, WHITE, 1.0);
        }
        // ILL WIND: a sour breath in, then curls riding the gust
        if let Some(d) = mine
            && let Some((u, _)) = bold::until(p, k, &["ability:wave"], t, INHALE_S)
        {
            for j in 0..12u64 {
                let ang = dmath::TAU * (j as f64 + px::rnd(7, j) * 0.6) / 12.0;
                let far = d.rp * (1.25 + 2.6 * (1.0 - u) * (0.7 + 0.5 * px::rnd(11, j)));
                let len = 6.0 + 7.0 * u;
                let (cx, cy) = (dmath::cos(ang), dmath::sin(ang) * 0.8);
                px::ink_line(
                    img,
                    (d.s.0 + cx * (far + len), d.s.1 + cy * (far + len)),
                    (d.s.0 + cx * far, d.s.1 + cy * far),
                    1.4,
                    SOUR_G,
                    0.35 + 0.65 * u,
                );
            }
        }
        if let Some((c0, r, fade)) = self.gust(p, t) {
            let a = (fade * 3.0).min(1.0);
            for (n, back) in [0.0, 0.6, 1.2].into_iter().enumerate() {
                if r - back > R {
                    let turn = if n % 2 == 0 { 0.3 } else { -0.3 } * r;
                    wind_ring(img, f.cam, c0, r - back, turn, SOUR_G, a * (1.0 - 0.28 * n as f64));
                }
            }
            for j in 0..12 {
                let ang = dmath::TAU * f64::from(j) / 12.0 + 0.25 * r;
                let (x, z) = (c0[0] + r * dmath::cos(ang), c0[1] + r * dmath::sin(ang));
                if x.abs() > A || z.abs() > A {
                    continue;
                }
                let sp = bold::at(f.cam, x, z, 0.15);
                curl(img, sp, 8.0 + 2.0 * fade, ang + t * 7.0, bold::pale(SOUR_G), a);
            }
        }
        // fully out of tune: the gauge's needle is pinned at the red stop
        if let Some((vic, left, dep)) = self.sour(p, t)
            && dep >= PINNED
            && let Some(d) = drawn.iter().find(|d| d.k == vic)
        {
            px::disc(img, d.s, d.rp, SOUR_G, 0.34);
            let g = (d.s.0, d.s.1 - d.rp - 4.0);
            let blink = dmath::sin(t * 22.0) > 0.0;
            px::ink_arc(img, g, 10.0, 1.8, dmath::PI, dmath::TAU, DIAL, 1.0);
            // the red zone and the stop at the dial's end
            px::arc(img, g, 10.0, 1.8, dmath::TAU - 0.55, dmath::TAU, RED, 1.0);
            px::ink_disc(img, (g.0 + 10.0, g.1), if blink { 3.0 } else { 2.2 }, RED, 1.0);
            let ang = -0.12 + 0.06 * dmath::sin(t * 60.0);
            let tip = (g.0 + dmath::cos(ang) * 9.5, g.1 + dmath::sin(ang) * 9.5);
            px::ink_line(img, g, tip, 1.8, RED, 1.0);
            px::ink_disc(img, g, 1.6, DIAL, 1.0);
            let a0 = -dmath::PI / 2.0;
            px::ink_arc(
                img,
                d.s,
                d.rp + 1.5,
                1.4,
                a0,
                a0 + dmath::TAU * left.clamp(0.02, 1.0),
                SOUR_G,
                0.95,
            );
        }
    }

    /// The enemy out of tune is marked sour (S12); once it is fully out of tune this look draws the pinned gauge
    /// itself.
    pub(super) fn bold_statuses(&self, p: &PictureCtx, t: f64) -> Vec<Status> {
        match self.sour(p, t) {
            Some((who, left, dep)) if dep < PINNED => vec![Status {
                who,
                kind: StatusKind::Sour,
                left: Some(left),
            }],
            _ => Vec::new(),
        }
    }

    /// The snap and the gust are heavy hits whatever is left of their damage; the stings are taps.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match bold::how(e) {
            "snip" | "wave" => Some(HitSize::Heavy),
            "tune" => Some(by_damage.max(HitSize::Solid)),
            "sting" | "endless" => Some(HitSize::Tap),
            _ => Some(by_damage),
        }
    }
}
