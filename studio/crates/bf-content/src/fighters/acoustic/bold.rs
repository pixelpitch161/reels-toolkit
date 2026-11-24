//! ACOUSTIC with the bold effects (`picture.effects = "bold"`; the owner's picks 20a to 20d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **Six-string rake** (20a): six outlined strings lie as a band across everything the stroke sweeps; a big
//!   pick rakes across them where the stroke really is, each string lights and rings as the pick crosses it,
//!   the flicked enemy trails speed lines, and notes pop off the strings (no claw marks on the floor).
//! - **CAPO** (20b): a big capo drops onto the ball and bites with a glint, and springs off at the end (the
//!   double pace and the draining ring are the ring's own speed-up look).
//! - **FINGERPICK** (20c): six strings curl back behind the ball, then whip out one after another as thick
//!   waving lines with a bright tip; the first five land as taps, the last, gold and thicker, as a solid hit.
//! - **BIG STRUM** (20d): six long strings stretch out from the ball and go round the whole ring like a clock
//!   hand with a fan trail, passing the enemy as the hit lands; the strings twang and notes burst from it.

use bf_core::dmath::{self, PI, TAU};
use bf_core::event::Event;
use bf_core::roster::kinds::rake::rake_lateral;
use bf_picture::bold::floor;
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::scene::{self, R_BALL as R};

use super::state::{Rake, Sweep};
use super::{AcousticLook, STRINGS6, along, clip_segment, world};

/// Seconds before a stroke its strings lie down.
const PRE_S: f64 = 0.08;
/// Seconds after a stroke its strings ring on and draw back.
const RING_S: f64 = 0.16;
/// Seconds a note popping off a string shows.
const NOTE_S: f64 = 0.55;
/// Seconds the flicked enemy trails speed lines.
const FLICK_S: f64 = 0.2;
/// Seconds the capo takes to drop on, and to bite after that.
const CLAMP_S: f64 = 0.14;
const BITE_S: f64 = 0.14;
/// Seconds before FINGERPICK its strings curl back.
const CURL_S: f64 = 0.3;
/// Seconds a FINGERPICK string takes to draw back after it reached out.
const RETRACT_S: f64 = 0.2;
/// Seconds before BIG STRUM its strings begin to stretch out, and seconds they take to draw back after it.
const STRETCH_S: f64 = 0.2;
const DRAW_BACK_S: f64 = 0.2;
/// World units between two strings of BIG STRUM's hand.
const HAND_GAP: f64 = 0.17;
/// Seconds of the hand's travel its fan trail covers.
const FAN_S: f64 = 0.2;

/// A struck string.
const LIT: V3 = [255.0, 246.0, 214.0];
/// The pick: tortoise shell.
const PICK: V3 = [206.0, 84.0, 44.0];
/// The capo's bar, its tension screw and its rubber.
const CAPO_BAR: V3 = [60.0, 54.0, 74.0];
const CAPO_SCREW: V3 = [214.0, 214.0, 226.0];
const CAPO_RUBBER: V3 = [20.0, 16.0, 24.0];

/// One painter item of the bold look.
enum BItem {
    /// A stroke (an index into `rakes`) and the seconds since it began (negative while its strings lie down).
    Stroke(usize, f64),
    /// A FINGERPICK string flicking (an index into `whips`) and the seconds since it left.
    Whip(usize, f64),
    /// A FINGERPICK string still curled back (an index into `whips`) and how far it has curled, 0 to 1.
    Curl(usize, f64),
}

/// Where across the stroke string `j` lies: the six are spread over the whole width the stroke sweeps.
fn string_at(r: &Rake, j: usize) -> f64 {
    (j as f64 - 2.5) * r.s * 0.4
}

/// The seconds into the stroke at which the pick crosses string `j` ([`rake_lateral`]'s smoothstep, inverted).
fn struck_at(r: &Rake, j: usize) -> f64 {
    let q = ((r.dir * string_at(r, j) + r.s) / (2.0 * r.s).max(1e-9)).clamp(0.0, 1.0);
    r.dur * (0.5 - dmath::sin(dmath::asin(1.0 - 2.0 * q) / 3.0))
}

/// How far along the strings the pick rakes: level with where the enemy was as the stroke began.
fn pick_along(p: &PictureCtx, r: &Rake) -> f64 {
    let foe = p.run.p(p.sim_i(r.t0), 1 - r.owner);
    let ra = (foe[0] - r.c[0]) * r.u[0] + (foe[1] - r.c[1]) * r.u[1];
    let lo = R + 0.35;
    ra.clamp(lo, (R + r.l * 0.85).max(lo))
}

/// The pick as a flat world polygon at `q`, its point leading along `m`: a rounded triangle.
fn pick_poly(q: [f64; 2], m: [f64; 2], y: f64, k: f64) -> Vec<V3> {
    let n = [-m[1], m[0]];
    let at = |a: f64, b: f64| world(along(along(q, m, a * k), n, b * k), y);
    let mut pts = vec![at(0.3, 0.0)];
    for j in 0..=8 {
        let ang = (65.0 + 230.0 * f64::from(j) / 8.0).to_radians();
        pts.push(at(-0.06 + 0.22 * dmath::cos(ang), 0.22 * dmath::sin(ang)));
    }
    pts
}

/// BIG STRUM as a clock hand at one moment.
struct Hand {
    /// Where the hand points now, and where it pointed [`FAN_S`] ago (radians, ring frame).
    theta: f64,
    behind: f64,
    /// How far its strings are out, 0 to 1.
    grow: f64,
    /// Seconds since its hit landed, if it has.
    since_hit: Option<f64>,
    /// Where the hit landed (ring frame), if it has or will.
    hit_at: Option<[f64; 2]>,
}

impl AcousticLook {
    /// The first whip of the FINGERPICK that whip `i` belongs to.
    fn pick_start(&self, i: usize) -> f64 {
        let w = &self.whips[i];
        usize::try_from(w.j)
            .ok()
            .and_then(|j| i.checked_sub(j))
            .map_or(w.t0, |i0| self.whips[i0].t0)
    }

    /// Whether whip `i` is the last string of its FINGERPICK.
    fn last_whip(&self, i: usize) -> bool {
        self.whips.get(i + 1).is_none_or(|next| next.j <= self.whips[i].j)
    }

    /// BIG STRUM `sw` at `ta` as the clock hand: it turns once round the ball, slow at first, and points at the
    /// enemy at the moment its hit lands (the fight's own sweep is a front crossing the ring; only its hit and
    /// its timing are shown).
    fn hand(&self, p: &PictureCtx, sw: &Sweep, ta: f64) -> Option<Hand> {
        let tau = ta - sw.t0;
        if !(-STRETCH_S..=sw.dur + DRAW_BACK_S).contains(&tau) {
            return None;
        }
        let k = sw.owner;
        let hit = bold::events_in(p, k, &["hit:sweep"], sw.t0, sw.t0 + sw.dur + 0.1).next();
        let (x_hit, th_hit) = match hit {
            Some(e) => {
                let i = p.sim_i(e.t);
                let (a, b) = (p.run.p(i, k), p.run.p(i, 1 - k));
                (
                    ((e.t - sw.t0) / sw.dur).clamp(0.0, 1.0),
                    dmath::atan2(b[1] - a[1], b[0] - a[0]),
                )
            }
            None => (0.5, sw.aim),
        };
        let turn = |x: f64| TAU * dmath::pow(x.clamp(0.0, 1.0), 1.5);
        let theta_at = |tau: f64| th_hit + turn(tau / sw.dur) - turn(x_hit);
        let grow = if tau > sw.dur {
            1.0 - bold::ease_in((tau - sw.dur) / DRAW_BACK_S)
        } else {
            bold::ease_out((tau + STRETCH_S) / (STRETCH_S + 0.12))
        };
        Some(Hand {
            theta: theta_at(tau),
            behind: theta_at(tau - FAN_S),
            grow,
            since_hit: hit.map(|e| ta - e.t).filter(|&d| d >= 0.0),
            hit_at: hit.map(|e| [e.f(5), e.f(6)]),
        })
    }

    /// Where the hand's middle line ends when it points along `theta` from `c`: at the canvas's edge.
    fn hand_end(c: [f64; 2], theta: f64, grow: f64) -> [f64; 2] {
        let far = along(c, [dmath::cos(theta), dmath::sin(theta)], 9.0);
        let end = clip_segment(c, far, scene::AP).map_or(c, |(_, b)| b);
        [c[0] + (end[0] - c[0]) * grow, c[1] + (end[1] - c[1]) * grow]
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !self.show(p, c.t) {
            return;
        }
        let ta = self.ta(p, c.t);
        let tint = bold::light(p.col[self.k]);
        // each stroke: a soft band under all six strings and a bright lane under the pick
        let j0 = self.rakes.partition_point(|r| r.t0 < ta - 0.6);
        let j1 = self.rakes.partition_point(|r| r.t0 <= ta);
        for r in &self.rakes[j0..j1] {
            let tau = ta - r.t0;
            if tau > r.dur + RING_S {
                continue;
            }
            let k = if tau <= r.dur {
                1.0
            } else {
                1.0 - (tau - r.dur) / RING_S
            };
            let (a, b) = (along(r.c, r.u, R), along(r.c, r.u, R + r.l));
            floor::lane(ic, c, a, b, r.s + 0.3, tint, 0.3 * k);
            let sn = rake_lateral(tau, r.dur, r.s, r.dir);
            floor::lane(ic, c, along(a, r.n, sn), along(b, r.n, sn), 0.22, tint, 0.5 * k);
        }
        // BIG STRUM: light under the hand
        for sw in &self.sweeps {
            if let Some(h) = self.hand(p, sw, ta) {
                let at = c.fs[sw.owner].p;
                floor::lane(ic, c, at, Self::hand_end(at, h.theta, h.grow), 0.6, tint, 0.55 * h.grow);
            }
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let mut out: Vec<(f64, Payload)> = Vec::new();
        if !self.show(p, f.t) {
            return out;
        }
        let ta = self.ta(p, f.t);
        let cam = f.cam;
        let j0 = self.rakes.partition_point(|r| r.t0 < ta - 0.6);
        let j1 = self.rakes.partition_point(|r| r.t0 <= ta + PRE_S);
        for j in j0..j1 {
            let r = &self.rakes[j];
            let tau = ta - r.t0;
            if tau > r.dur + RING_S {
                continue;
            }
            let mid = along(r.c, r.u, R + r.l * 0.5);
            out.push((cam.depth(&[world(mid, 0.5)])[0], Box::new(BItem::Stroke(j, tau))));
        }
        for (i, w) in self.whips.iter().enumerate() {
            let tau = ta - w.t0;
            let at = f.fs[w.owner].p;
            if (0.0..=self.t_ext + RETRACT_S).contains(&tau) {
                out.push((cam.depth(&[world(at, 0.5)])[0] - 0.02, Box::new(BItem::Whip(i, tau))));
            } else if tau < 0.0 && ta >= self.pick_start(i) - CURL_S {
                let grow = bold::ease_out((ta - (self.pick_start(i) - CURL_S)) / CURL_S);
                let back = along(at, [dmath::cos(w.ang + PI), dmath::sin(w.ang + PI)], R + 0.3);
                out.push((cam.depth(&[world(back, 0.5)])[0], Box::new(BItem::Curl(i, grow))));
            }
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let cam = f.cam;
        match payload.downcast_ref::<BItem>() {
            Some(&BItem::Stroke(j, tau)) => self.draw_stroke(p, fr, cam, &self.rakes[j], tau),
            Some(&BItem::Whip(i, tau)) => {
                let w = &self.whips[i];
                let at = f.fs[w.owner].p;
                let ext = if tau <= self.t_ext {
                    bold::ease_out(tau / self.t_ext)
                } else {
                    1.0 - (tau - self.t_ext) / RETRACT_S
                };
                let lc = w.len * ext.max(0.0);
                if lc < 0.05 {
                    return;
                }
                let last = self.last_whip(i);
                let u = [dmath::cos(w.ang), dmath::sin(w.ang)];
                let n = [-u[1], u[0]];
                let pts: Vec<V3> = (0..=13)
                    .map(|i| {
                        let x = f64::from(i) / 13.0;
                        let lat = 0.13 * dmath::sin(PI * x * 2.5 - tau * 45.0 + w.j as f64) * x * (1.0 - 0.6 * ext);
                        world(along(along(at, u, R + lc * x), n, lat), 0.5)
                    })
                    .collect();
                let col = if last {
                    bold::GOLD
                } else {
                    bold::pale(STRINGS6[w.j.rem_euclid(6) as usize])
                };
                bold::thick_line(fr, cam, &pts, col, if last { 4 } else { 3 });
                bold::blob(fr, cam, pts[13], if last { 0.14 } else { 0.1 }, bold::WHITE);
            }
            Some(&BItem::Curl(i, grow)) => {
                let w = &self.whips[i];
                let at = f.fs[w.owner].p;
                // fanned out behind the ball, away from the enemy
                let th = w.ang + PI + (w.j as f64 - 2.5) * 0.56;
                let d = [dmath::cos(th), dmath::sin(th)];
                let cc = along(at, d, R + 0.42);
                let side = if w.j % 2 == 0 { 1.0 } else { -1.0 };
                let mut pts = vec![world(along(at, d, R * 0.95), 0.5)];
                for i in 0..=16 {
                    let u = f64::from(i) / 16.0;
                    let phi = th + PI + side * u * 1.5 * TAU * grow;
                    let rho = 0.3 * (1.0 - 0.7 * u);
                    pts.push(world(
                        [cc[0] + rho * dmath::cos(phi), cc[1] + rho * dmath::sin(phi)],
                        0.5,
                    ));
                }
                bold::thick_line(fr, cam, &pts, bold::pale(STRINGS6[w.j.rem_euclid(6) as usize]), 3);
            }
            None => {}
        }
    }

    /// One stroke `tau` seconds after it began: the six strings and the pick.
    fn draw_stroke(&self, p: &PictureCtx, fr: &mut Frame, cam: &Cam, r: &Rake, tau: f64) {
        let grow = if tau < 0.0 {
            bold::ease_out(1.0 + tau / PRE_S)
        } else if tau <= r.dur {
            1.0
        } else {
            1.0 - bold::ease_in((tau - r.dur) / RING_S)
        };
        if grow <= 0.03 {
            return;
        }
        let len = r.l * grow;
        for (j, &string) in STRINGS6.iter().enumerate() {
            let o = string_at(r, j);
            let since = tau - struck_at(r, j);
            let (hot, amp) = if since >= 0.0 {
                (dmath::exp(-since / 0.1), 0.08 * dmath::exp(-since / 0.12))
            } else {
                (0.0, 0.0)
            };
            let pts: Vec<V3> = (0..=8)
                .map(|i| {
                    let u = f64::from(i) / 8.0;
                    let lat = o + amp * dmath::sin(since * 75.0 + j as f64) * dmath::sin(PI * u);
                    world(along(along(r.c, r.u, R + len * u), r.n, lat), 0.5)
                })
                .collect();
            if hot > 0.3 {
                bold::thick_line(fr, cam, &pts, LIT, 4);
            } else {
                bold::thick_line(fr, cam, &pts, string, 3);
            }
        }
        // the pick, where the stroke really is, with a swoosh behind it
        if tau <= r.dur + 0.05 {
            let m = [r.n[0] * r.dir, r.n[1] * r.dir];
            let ra = pick_along(p, r);
            let now = rake_lateral(tau, r.dur, r.s, r.dir) * r.dir;
            let was = rake_lateral(tau - 0.05, r.dur, r.s, r.dir) * r.dir;
            let q = along(along(r.c, r.u, ra), m, now);
            if now - was > 0.04 {
                let tail = along(along(r.c, r.u, ra), m, was - 0.1);
                fr.poly(
                    cam,
                    &[
                        world(tail, 0.6),
                        world(along(q, r.u, 0.2), 0.6),
                        world(along(q, r.u, -0.2), 0.6),
                    ],
                    bold::pale(p.col[r.owner]),
                    true,
                );
            }
            bold::outlined_poly(fr, cam, &pick_poly(q, m, 0.62, 1.25), PICK);
            fr.poly(
                cam,
                &pick_poly(along(q, m, -0.04), m, 0.63, 0.55),
                bold::pale(PICK),
                true,
            );
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !self.show(p, f.t) {
            return;
        }
        let k = self.k;
        let ta = self.ta(p, f.t);
        let cam = f.cam;
        // notes pop off the strings as the pick crosses them: three of a stroke in the air, all six of one that
        // landed
        let j0 = self.rakes.partition_point(|r| r.t0 < ta - 1.0);
        let j1 = self.rakes.partition_point(|r| r.t0 <= ta);
        for j in j0..j1 {
            let r = &self.rakes[j];
            let landed = self.rake_hit[j];
            for s in (0..6).filter(|s| landed || s % 2 == 0) {
                let age = ta - r.t0 - struck_at(r, s);
                if !(0.0..NOTE_S).contains(&age) {
                    continue;
                }
                let u = age / NOTE_S;
                let from = along(along(r.c, r.u, R + r.l * (0.3 + 0.12 * s as f64)), r.n, string_at(r, s));
                let c0 = bold::at(cam, from[0], from[1], 0.55);
                let c = (
                    c0.0 + 5.0 * dmath::sin(age * 9.0 + s as f64),
                    c0.1 - 5.0 - 24.0 * bold::ease_out(u),
                );
                let col = if landed { bold::GOLD } else { bold::pale(p.col[k]) };
                px::note(img, c, col, ((1.0 - u) / 0.4).min(1.0));
            }
        }
        // the flicked enemy trails speed lines
        if let Some((age, _)) = bold::since(p, k, &["hit:rake"], ta, FLICK_S)
            && let Some(d) = drawn.iter().find(|d| d.k == 1 - k)
        {
            let (a, b) = (p.run.p(p.sim_i(ta - 0.07), 1 - k), p.run.p(p.sim_i(ta), 1 - k));
            let (sa, sb) = (bold::at(cam, a[0], a[1], R), bold::at(cam, b[0], b[1], R));
            let fade = 1.0 - age / FLICK_S;
            px::speed_lines(
                img,
                d.s,
                d.rp,
                (sb.0 - sa.0, sb.1 - sa.1),
                6.0 + 14.0 * fade,
                5,
                bold::WHITE,
                0.85 * fade,
            );
        }
        self.capo(p, img, f, drawn);
        for sw in &self.sweeps {
            self.big_strum(p, img, f, sw, ta);
        }
    }

    /// CAPO: the capo drops onto the ball, bites, holds while the haste lasts and springs off at its end.
    fn capo(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let st = &f.fs[self.k];
        if self.capo_dur <= 0.0 || st.haste <= 0.0 || !f.in_fight || st.shattered {
            return;
        }
        let Some(d) = drawn.iter().find(|d| d.k == self.k) else {
            return;
        };
        let on = self.capo_dur - st.haste;
        let drop = 1.0 - bold::ease_in(on / CLAMP_S);
        let off = bold::ease_in(1.0 - st.haste / 0.15);
        let a = (on / 0.05).clamp(0.0, 1.0) * (1.0 - off);
        let bite = if on >= CLAMP_S {
            (1.0 - (on - CLAMP_S) / BITE_S).max(0.0)
        } else {
            0.0
        };
        let (x, rp) = (d.s.0, d.rp);
        let y = d.s.1 - rp * 0.30 - rp * 1.5 * drop - rp * 1.1 * off;
        let w = rp * (1.14 + 0.14 * bite);
        let h = 3.0 + 1.5 * bite;
        let col = p.col[self.k];
        // the spring arm that reaches round under the neck
        px::ink_line(img, (x + w, y), (x + w * 0.7, y + rp * 0.6), 2.0, CAPO_BAR, a);
        px::ink_poly(
            img,
            &[(x - w, y - h), (x + w, y - h), (x + w, y + h), (x - w, y + h)],
            CAPO_BAR,
            a,
        );
        px::line(
            img,
            (x - w + 2.0, y - h + 1.0),
            (x + w - 2.0, y - h + 1.0),
            1.0,
            bold::pale(col),
            a,
        );
        px::line(img, (x - w + 1.0, y + h), (x + w - 1.0, y + h), 1.5, CAPO_RUBBER, a);
        px::ink_disc(img, (x + w + 1.0, y), h + 1.5, CAPO_SCREW, a);
        if bite > 0.0 {
            let seed = (self.capo_dur * 1000.0) as u64;
            for (side, sd) in [(-1.0, seed), (1.0, seed + 1)] {
                px::chips(
                    img,
                    (x + side * w, y),
                    on - CLAMP_S,
                    BITE_S,
                    4,
                    14.0,
                    sd,
                    bold::pale(col),
                );
            }
            px::glint(img, (x + w + 1.0, y), 5.0 + 5.0 * bite, bold::WHITE, bite);
        }
    }

    /// BIG STRUM: the hand of six long strings, its fan trail, the twang and the notes of its hit. All after
    /// the grade, so the low-HP spotlight does not grey it.
    fn big_strum(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, sw: &Sweep, ta: f64) {
        let Some(h) = self.hand(p, sw, ta) else {
            return;
        };
        let cam = f.cam;
        let at = f.fs[sw.owner].p;
        let col = p.col[sw.owner];
        // the fan trail: three wedges behind the hand, the nearest the brightest
        let swept = h.theta - h.behind;
        if swept > 0.03 && h.grow > 0.5 {
            for (n, a) in [0.46, 0.3, 0.16].into_iter().enumerate() {
                let (a0, a1) = (h.theta - swept * n as f64 / 3.0, h.theta - swept * (n + 1) as f64 / 3.0);
                let mut pts: Vec<(f64, f64)> = Vec::new();
                let mut inner: Vec<(f64, f64)> = Vec::new();
                for i in 0..=4 {
                    let th = a0 + (a1 - a0) * f64::from(i) / 4.0;
                    let e = Self::hand_end(at, th, h.grow);
                    let s = along(at, [dmath::cos(th), dmath::sin(th)], R * 1.1);
                    pts.push(bold::at(cam, e[0], e[1], 0.3));
                    inner.push(bold::at(cam, s[0], s[1], 0.3));
                }
                pts.extend(inner.into_iter().rev());
                px::poly(img, &pts, bold::pale(col), a);
            }
        }
        // the six strings, twanging once the hit has landed
        let u = [dmath::cos(h.theta), dmath::sin(h.theta)];
        let n = [-u[1], u[0]];
        let end = Self::hand_end(at, h.theta, h.grow);
        let len = dmath::hypot(end[0] - at[0], end[1] - at[1]);
        if len > R * 1.2 {
            let amp = h.since_hit.map_or(0.0, |d| 0.16 * dmath::exp(-d / 0.16));
            for (j, &string) in STRINGS6.iter().enumerate() {
                let o = (j as f64 - 2.5) * HAND_GAP;
                let pts: Vec<(f64, f64)> = (0..=10)
                    .map(|i| {
                        let x = f64::from(i) / 10.0;
                        let lat =
                            o + amp * dmath::sin(h.since_hit.unwrap_or(0.0) * 60.0 + j as f64) * dmath::sin(PI * x);
                        let q = along(along(at, u, R * 0.9 + (len - R * 0.9) * x), n, lat);
                        bold::at(cam, q[0], q[1], 0.5)
                    })
                    .collect();
                let (w, c) = if j < 3 {
                    (2.2, bold::pale(string))
                } else {
                    (1.4, bold::WHITE)
                };
                px::ink_polyline(img, &pts, w, c, 1.0);
            }
        }
        // the hit: notes burst from the enemy
        if let (Some(d), Some(q)) = (h.since_hit, h.hit_at)
            && d < NOTE_S
        {
            let c0 = bold::at(cam, q[0], q[1], R);
            let v = d / NOTE_S;
            for i in 0..8u64 {
                let ang = TAU * (i as f64 + px::rnd(77, i)) / 8.0;
                let far = (16.0 + 26.0 * px::rnd(78, i)) * bold::ease_out(v);
                let c = (
                    c0.0 + dmath::cos(ang) * far,
                    c0.1 + dmath::sin(ang) * far * 0.7 - 8.0 * v,
                );
                px::note(img, c, bold::GOLD, ((1.0 - v) / 0.4).min(1.0));
            }
        }
    }

    /// FINGERPICK's first five strings land as taps and its last as a solid hit, whatever their damage.
    pub(super) fn bold_hit(&self, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        if bold::how(e) != "whip" {
            return Some(by_damage);
        }
        let last = self
            .whips
            .partition_point(|w| w.t0 <= e.t)
            .checked_sub(1)
            .is_some_and(|i| self.last_whip(i));
        Some(if last { HitSize::Solid } else { HitSize::Tap })
    }
}
