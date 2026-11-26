//! TURNTABLES with the bold effects (`picture.effects = "bold"`; the owner's picks 36a to 36d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **THE RING IS THE RECORD** (36a): the record floor as it was, but each jerk snaps it round (further, and in
//!   two frames instead of a slow swing) and back on the second jerk; streak arcs of light run along the grooves
//!   in the jerk's direction, a curved arrow on the floor shows where the enemy is carried, and speed lines trail
//!   the enemy on its way to the rope.
//! - **SCRATCH** (36b): two scratch arcs with arrowheads flick back and forth over the DJ's ball at twice the
//!   beat while it lasts (the draining ring is the ring's own speed-up look).
//! - **REWIND** (36c): for the two seconds a rewind will undo, the DJ's path is dropped behind it as dots and a
//!   row of pips over the ball goes out as it is hit; on the rewind the ball zips back along the dots, eating
//!   them, with its afterimages ahead of it and chevrons on the path pointing back; the pips refill green, each
//!   with a plus sign.
//! - **BASS DROP** (36d): a landing mark closes in on the spot under the growing shadow of a giant record that
//!   hangs over it, spinning; it drops flat, squashes, and three jagged bass rings roll out.

use bf_core::dmath::{self, PI, TAU};
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px::{self, Surface};
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::scene::R_BALL as R;

use super::{GROOVE, Item, LABEL_R, REC_R, SCR_AMP, TurntablesLook, VINYL, circle_world, record_albedo, slams};

/// A refilled pip.
const GREEN: V3 = [84.0, 255.0, 132.0];
/// A pip that went out.
const OUT: V3 = [64.0, 58.0, 76.0];
/// Seconds a jerk's streak arcs and its arrow show.
const STREAK_S: f64 = 0.26;
/// The share of a scratch the floor takes to snap round (and to snap back).
const SNAP: f64 = 0.14;
/// Seconds between the dots of the remembered path.
const DOT_S: f64 = 0.07;
/// The least distance between two dots of the path.
const DOT_GAP: f64 = 0.2;
/// The pips of a REWIND.
const PIPS: usize = 6;
/// Seconds the refilled pips stay after the ball is back.
const REFILL_S: f64 = 0.6;
/// Seconds the giant record takes to fall, and lies there.
const FALL_S: f64 = 0.28;
const LIE_S: f64 = 0.55;
/// Seconds the bass rings roll.
const BASS_S: f64 = 0.8;

/// The height of BASS DROP's record: it hangs over the spot, bobbing, then drops.
fn hang(t: f64, t0: f64, t_slam: f64) -> f64 {
    if t < t_slam - FALL_S {
        3.3 - 0.5 * bold::ease_out((t - t0) / 0.4) + 0.06 * dmath::sin(t * 9.0)
    } else {
        let u = ((t - (t_slam - FALL_S)) / FALL_S).clamp(0.0, 1.0);
        2.8 * (1.0 - u * u) + 0.06
    }
}

/// The giant record seen from the camera: flat at height `h` over `(cx, cz)`, radius `r`, turned by `spin`.
#[allow(clippy::too_many_arguments)]
fn record<S: Surface + ?Sized>(s: &mut S, cam: &Cam, cx: f64, cz: f64, r: f64, h: f64, label: V3, spin: f64, a: f64) {
    let ring = |rr: f64| cam.wp(&circle_world(cx, cz, rr, h, 32, spin));
    px::ink_poly(s, &ring(r), VINYL.map(|c| c * 1.25), a);
    for (rr, k) in [(0.9, 1.5), (0.77, 1.2), (0.64, 1.35), (0.51, 1.1)] {
        let mut groove = ring(r * rr);
        groove.push(groove[0]);
        px::polyline(s, &groove, 1.0, GROOVE.map(|c| c * k), a);
    }
    // the sheen: two wedges of light that stay put while the record turns
    for side in [0.0, PI] {
        let at = |rr: f64, ang: f64| {
            bf_picture::scene::to_world(
                cx + r * rr * dmath::cos(ang + side),
                cz + r * rr * dmath::sin(ang + side),
                h,
            )
        };
        let wedge = cam.wp(&[at(0.42, 0.5), at(0.96, 0.4), at(0.96, 0.66), at(0.42, 0.62)]);
        px::poly(s, &wedge, [150.0, 154.0, 182.0], 0.55 * a);
    }
    px::ink_poly(s, &ring(r * 0.36), label, a);
    let mut band = ring(r * 0.25);
    band.push(band[0]);
    px::polyline(s, &band, 1.0, bold::deep(label), a);
    // a printed mark, so the spin shows
    let mark = cam.wp(&circle_world(cx, cz, r * 0.3, h, 3, spin));
    px::disc(s, mark[0], 2.0, bold::INK, a);
    px::disc(
        s,
        cam.wp(&circle_world(cx, cz, 0.0, h, 1, 0.0))[0],
        1.6,
        [214.0, 218.0, 230.0],
        a,
    );
}

impl TurntablesLook {
    /// The floor's angle with the bold scratch: the slow spin, and each scratch snapped round on its first jerk
    /// and back on its second.
    fn bold_floor_angle(&self, p: &PictureCtx, t: f64) -> f64 {
        let spin = TAU * t / self.spin_p;
        let tc = p.fx_clock(t);
        let Some(j) = self.scr.partition_point(|s| s.0 <= tc).checked_sub(1) else {
            return spin;
        };
        if tc >= p.t_ko + 0.01 && t >= p.t_hs {
            return spin;
        }
        let (t0, d, stg, sgn) = self.scr[j];
        let u = (tc - t0) / d;
        if !(0.0..1.0).contains(&u) {
            return spin;
        }
        let shape = if u < 0.5 {
            bold::ease_out(u / SNAP)
        } else {
            1.0 - bold::ease_out((u - 0.5) / SNAP)
        };
        spin + sgn * SCR_AMP * 1.5 * stg / 1.3 * shape
    }

    /// The REWIND `t` belongs to: the time it rewinds to (the path is remembered from then), when it fires, when
    /// the ball is back, and the HP it undoes.
    fn rewind_at(&self, p: &PictureCtx, t: f64) -> Option<(f64, f64, f64, f64)> {
        let k = self.k;
        p.rec
            .events
            .iter()
            .filter(|e| e.kind == "heal" && e.who() == Some(k))
            .find_map(|e| {
                let t_then = e.f(4);
                let t_back = self
                    .rw_win
                    .iter()
                    .find(|w| w.0 == e.t)
                    .map_or(e.t + p.show.grid.beat, |w| w.1);
                (t_then < e.t && t_then <= t && t < t_back + REFILL_S).then_some((t_then, e.t, t_back, e.f(3)))
            })
    }

    pub(super) fn bold_canvas_albedo(&self, p: &PictureCtx, alb: &mut [V3], c: &CanvasCtx<'_>) {
        let phi = self.bold_floor_angle(p, c.t);
        for (j, a) in alb.iter_mut().enumerate() {
            *a = record_albedo(*a, c.sxp[j], c.szp[j], phi, self.col).0;
        }
    }

    /// A jerk's streak arcs: light running along four grooves in the jerk's direction.
    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let Some((age, e)) = bold::since(p, self.k, &["jerk"], c.t, STREAK_S) else {
            return;
        };
        let u = age / STREAK_S;
        let sgn = if e.f(3) < 0.0 { -1.0 } else { 1.0 };
        let lime = bold::light(self.col);
        let len = 0.75 * (1.0 - 0.5 * u);
        let turn = sgn * 0.6 * bold::ease_out(u);
        let sector = TAU / 5.0;
        for (j, light) in ic.iter_mut().enumerate() {
            let r = dmath::hypot(c.sxp[j], c.szp[j]);
            if r <= LABEL_R + 0.1 || r >= REC_R - 0.2 {
                continue;
            }
            let n = ((r - 1.2) / 0.46).round();
            if !(0.0..=3.0).contains(&n) || (r - (1.2 + n * 0.46)).abs() >= 0.06 {
                continue;
            }
            let th = dmath::atan2(c.szp[j], c.sxp[j]);
            let behind = (-(th - turn - n * 0.7) * sgn).rem_euclid(sector);
            if behind < len {
                let v = ((1.0 - behind / len) * 3.0).ceil() / 3.0 * (1.0 - u) * 1.2;
                for ch in 0..3 {
                    light[ch] += v * lime[ch];
                }
            }
        }
    }

    /// BASS DROP: the landing mark under the record's growing shadow, then the bass rings.
    pub(super) fn bold_canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let lime = bold::light(self.col);
        for s in slams(p, c.i) {
            if s.owner != self.k as i64 || c.t >= s.t_slam {
                continue;
            }
            let u = ((c.t - s.t0) / (s.t_slam - s.t0).max(1e-9)).clamp(0.0, 1.0);
            floor::shade(ic, c, s.cx, s.cz, s.rad * (0.5 + 0.6 * u), 0.25 + 0.4 * u);
            floor::target(ic, c, s.cx, s.cz, s.rad, u, lime);
        }
        for &(ts, cx, cz) in &self.slam_ev {
            let age = c.t - ts;
            if !(0.0..BASS_S).contains(&age) {
                continue;
            }
            for n in 0..3 {
                let u = (age - 0.12 * f64::from(n)) / (BASS_S - 0.24);
                if (0.0..1.0).contains(&u) {
                    floor::band(
                        ic,
                        c,
                        Family::Noise,
                        cx,
                        cz,
                        self.slam_rad * 0.7 + 2.4 * u,
                        1.0 - u,
                        c.t + f64::from(n),
                        lime,
                        1.5 - 0.3 * f64::from(n),
                    );
                }
            }
        }
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        match payload.downcast_ref::<Item>() {
            // the afterimages lead the zip instead (`bold_after_grade`)
            Some(Item::Ghosts) | None => {}
            Some(Item::Drop) => {
                // the giant record lying under the balls: squashed wide as it lands, then fading
                if !f.in_fight {
                    return;
                }
                for &(ts, cx, cz) in &self.slam_ev {
                    let age = f.t - ts;
                    if !(0.0..LIE_S).contains(&age) {
                        continue;
                    }
                    let wide = 1.1 * (1.0 + 0.14 * (1.0 - age / 0.1).max(0.0));
                    let a = ((LIE_S - age) / 0.2).clamp(0.0, 1.0);
                    let (mut arr, depth) = (fr.array(), fr.depth());
                    record(
                        &mut arr,
                        f.cam,
                        cx,
                        cz,
                        self.slam_rad * wide,
                        0.06,
                        self.col,
                        ts * 5.0,
                        a,
                    );
                    fr.set_array(&arr, Some(&depth));
                }
            }
            Some(Item::Arm) => self.draw_tonearm(fr, f.cam, f.l, self.arm_state(p, f.t)),
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, t, cam) = (self.k, f.t, f.cam);
        let o = 1 - k;
        let lime = bold::pale(self.col);
        let me = drawn.iter().find(|d| d.k == k);
        // a jerk: the arrow on the floor that the enemy is carried along, and speed lines behind it
        if let Some((age, e)) = bold::since(p, k, &["jerk"], t, STREAK_S) {
            let (x, z) = (e.f(4), e.f(5));
            let sgn = if e.f(3) < 0.0 { -1.0 } else { 1.0 };
            let re = dmath::hypot(x, z);
            let u = age / STREAK_S;
            if re > 0.4 {
                // it starts clear of the ball and reaches further as the jerk carries on
                let th0 = dmath::atan2(z, x) + sgn * (R * 1.2 / re).min(1.2);
                let reach = (0.4 + 0.8 * bold::ease_out(u * 2.0)) * (2.0 / re).min(1.6);
                let pts: Vec<(f64, f64)> = (0..=10)
                    .map(|n| {
                        let q = f64::from(n) / 10.0;
                        let (th, r) = (th0 + sgn * reach * q, (re * (1.08 + 0.2 * q)).min(REC_R));
                        bold::at(cam, r * dmath::cos(th), r * dmath::sin(th), 0.05)
                    })
                    .collect();
                let a = 1.0 - u * u;
                px::ink_polyline(img, &pts[..10], 3.2, lime, a);
                px::ink_arrow(img, pts[8], pts[10], 3.2, 9.0, lime, a);
            }
            if let Some(d) = drawn.iter().find(|d| d.k == o) {
                let (now, was) = (p.run.p(f.i, o), p.run.p(f.i.saturating_sub(2), o));
                if dmath::hypot(now[0] - was[0], now[1] - was[1]) > 0.08 {
                    let (a, b) = (bold::at(cam, was[0], was[1], R), bold::at(cam, now[0], now[1], R));
                    px::speed_lines(img, d.s, d.rp, (b.0 - a.0, b.1 - a.1), 16.0, 6, lime, 1.0 - u);
                }
            }
        }
        // SCRATCH: two scratch arcs flick back and forth over the ball
        if let Some(d) = me
            && p.bold_haste_left(k, t).is_some()
        {
            let phase = TAU * t / (p.show.grid.beat / 2.0);
            for (rr, dir) in [(1.42, 1.0), (1.74, -1.0)] {
                let mid = -PI / 2.0 + dir * 0.55 * dmath::sin(phase);
                let r = d.rp * rr;
                px::ink_arc(img, d.s, r, 2.0, mid - 0.5, mid + 0.5, self.col, 1.0);
                // the arrowhead on the end it is moving toward
                let way = if dir * dmath::cos(phase) >= 0.0 { 1.0 } else { -1.0 };
                let end = mid + 0.5 * way;
                let tip = (d.s.0 + r * dmath::cos(end), d.s.1 + r * dmath::sin(end));
                let tang = (-dmath::sin(end) * way, dmath::cos(end) * way);
                px::ink_arrow(
                    img,
                    (tip.0 - tang.0, tip.1 - tang.1),
                    (tip.0 + tang.0 * 5.0, tip.1 + tang.1 * 5.0),
                    2.0,
                    5.0,
                    self.col,
                    1.0,
                );
            }
        }
        // REWIND
        if let Some((t_then, t_heal, t_back, undo)) = self.rewind_at(p, t) {
            let zip = ((t - t_heal) / (t_back - t_heal).max(1e-6)).clamp(0.0, 1.0);
            // the remembered path: dots behind the ball, eaten from the far end as it zips back
            if t < t_back {
                let upto = if t < t_heal {
                    t
                } else {
                    t_then + (1.0 - zip) * (t_heal - t_then)
                };
                let n_dots = ((upto - t_then) / DOT_S).floor().max(0.0) as usize;
                // one dot every so far along the path, none where the ball stood still
                let mut dots: Vec<[f64; 2]> = Vec::new();
                for n in 0..n_dots {
                    let q = p.run.p(p.sim_i(t_then + n as f64 * DOT_S), k);
                    if dots
                        .last()
                        .is_none_or(|l| dmath::hypot(q[0] - l[0], q[1] - l[1]) >= DOT_GAP)
                    {
                        dots.push(q);
                    }
                }
                let on_floor = |q: [f64; 2]| bold::at(cam, q[0], q[1], 0.06);
                for (n, &q) in dots.iter().enumerate() {
                    if t >= t_heal && n % 3 == 2 {
                        // a chevron pointing back down the path
                        px::ink_arrow(img, on_floor(q), on_floor(dots[n - 1]), 1.8, 6.0, lime, 1.0);
                    } else {
                        px::ink_disc(img, on_floor(q), 1.8, lime, 1.0);
                    }
                }
            }
            // the afterimages run ahead of the zipping ball
            if let Some(d) = me
                && (t_heal..t_back).contains(&t)
            {
                for g in (1..=4).rev() {
                    let tg = t + 0.04 * f64::from(g);
                    if tg >= t_back {
                        continue;
                    }
                    let q = p.run.p(p.sim_i(tg), k);
                    let r = d.rp * (1.0 - 0.07 * f64::from(g));
                    px::ring(
                        img,
                        bold::at(cam, q[0], q[1], R),
                        r,
                        1.8,
                        lime,
                        0.8 - 0.15 * f64::from(g),
                    );
                }
            }
            // the pips: they go out as it is hit, and refill green with plus signs as it rewinds
            if let Some(d) = me {
                let hp_then = p.run.f("HP", p.sim_i(t_then), k).unwrap_or(0.0);
                let hp_now = p.run.f("HP", f.i, k).unwrap_or(hp_then);
                let lost = (((hp_then - hp_now) / undo.max(1e-6)) * PIPS as f64)
                    .round()
                    .clamp(0.0, PIPS as f64) as usize;
                let refilled = (zip * PIPS as f64).ceil() as usize;
                let fade = if t < t_back {
                    1.0
                } else {
                    (1.0 - (t - t_back) / REFILL_S).clamp(0.0, 1.0)
                };
                let y = (d.s.1 - d.rp - 21.0).max(8.0);
                for n in 0..PIPS {
                    let at = (d.s.0 + (n as f64 - (PIPS - 1) as f64 / 2.0) * 8.0, y);
                    if t >= t_heal {
                        if n < refilled {
                            px::ink_disc(img, at, 3.2, GREEN, fade);
                            // its plus sign, rising
                            let since = t - (t_heal + n as f64 / PIPS as f64 * (t_back - t_heal));
                            let up = at.1 - 8.0 - 5.0 * bold::ease_out(since / 0.3);
                            px::ink_line(img, (at.0 - 2.5, up), (at.0 + 2.5, up), 1.4, GREEN, fade);
                            px::line(img, (at.0, up - 2.5), (at.0, up + 2.5), 1.4, GREEN, fade);
                        } else {
                            px::ink_disc(img, at, 2.2, OUT, 1.0);
                        }
                    } else if n < PIPS - lost {
                        px::ink_disc(img, at, 3.0, self.col, 1.0);
                    } else {
                        px::ink_disc(img, at, 2.2, OUT, 1.0);
                        px::cross(img, at, 2.0, 1.0, [255.0, 90.0, 90.0], 1.0);
                    }
                }
            }
        }
        // BASS DROP: the giant record over the spot, then its splinters
        for s in slams(p, f.i) {
            if s.owner != k as i64 || !(s.t0..s.t_slam).contains(&t) {
                continue;
            }
            let grow = bold::ease_out((t - s.t0) / 0.2);
            let h = hang(t, s.t0, s.t_slam);
            record(img, cam, s.cx, s.cz, s.rad * 1.1 * grow, h, self.col, t * 5.0, 1.0);
        }
        for &(ts, cx, cz) in &self.slam_ev {
            let age = t - ts;
            if (0.0..0.3).contains(&age) {
                let at = bold::at(cam, cx, cz, 0.1);
                px::chips(img, at, age, 0.3, 12, 40.0, (ts * 1000.0) as u64, GROOVE);
            }
            // the bass rings again, crisp over the dimmed ring: jagged circles lying on the floor
            for n in 0..3 {
                let u = (age - 0.12 * f64::from(n)) / (BASS_S - 0.24);
                if !(0.0..1.0).contains(&u) {
                    continue;
                }
                let r = self.slam_rad * 0.7 + 2.4 * u;
                let mut pts: Vec<(f64, f64)> = (0..48)
                    .map(|j| {
                        let th = f64::from(j) * TAU / 48.0;
                        let rr = r + if j % 2 == 0 { 0.07 } else { -0.07 };
                        bold::at(cam, cx + rr * dmath::cos(th), cz + rr * dmath::sin(th), 0.04)
                    })
                    .collect();
                pts.push(pts[0]);
                px::ink_polyline(img, &pts, 2.2 - 0.5 * f64::from(n), self.col, (1.0 - u) * 0.95);
            }
        }
    }

    /// A rope slam always pops as a proper hit, whatever its damage.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match bold::how(e) {
            "scratch" => Some(by_damage.max(HitSize::Solid)),
            _ => Some(by_damage),
        }
    }
}
