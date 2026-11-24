//! DRUMS with the bold effects (`picture.effects = "bold"`; the owner's picks 14a to 14d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **Drumhead pulse** (14a): two hickory sticks over the ball lift before the beat and strike on it; the pulse
//!   is a drum skin rolling out (an outlined hoop with chrome lugs round a cream skin) over a quake of lifted
//!   floor tiles; a dart it sweeps is swatted with a pop and tumbles away.
//! - **DRUM ROLL** (14b): the sticks blur into fans and the pulses of the roll are their own crisp, thinner
//!   skins, four to a beat (the draining ring is the ring's own speed-up look).
//! - **CRASH CYMBAL** (14c): a gold cymbal swings round from behind the ball to stand between it and the enemy;
//!   a blocked hit turns it white, makes it shiver and sends a sizzling ring out; it fades and blinks as the
//!   shield runs down.
//! - **DOUBLE KICK** (14d): a landing mark closes in on the enemy's spot; a giant felt beater lands on the eighth
//!   before, lifts, and lands again on the beat, each time with a quake.

use bf_core::dmath::{self, PI, TAU};
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::BallState;
use bf_picture::scene::{self, R_BALL as R};

use crate::fighters::shared::older::{Poly, blend_into, mask_of_all, mask_of_lines};
use crate::fighters::shared::reeds::Num;

use super::{BRONZE, BRONZE_DK, CHROME, DrumsLook, FELT, Pulse, RING_Y, SHAFT, SKIN, pulse_state, ring_polys, slams};

/// The sticks' wood.
const HICKORY: V3 = [226.0, 186.0, 122.0];
/// Seconds a stick lifts before its stroke.
const LIFT_S: f64 = 0.13;
/// Seconds a stick stays down after its stroke.
const DOWN_S: f64 = 0.09;
/// Seconds the cymbal takes to swing round to the front.
const SWING_S: f64 = 0.3;
/// Seconds the cymbal flares and shivers after a block.
const FLARE_S: f64 = 0.3;
/// Seconds a swatted dart's pop lasts.
const SWAT_S: f64 = 0.3;
/// Seconds a landing of the beater quakes.
const QUAKE_S: f64 = 0.35;

/// The standing cymbal of the painter pass.
struct Cymbal {
    /// Its middle: x, z and height.
    c: V3,
    /// The direction it faces (toward the enemy).
    dir: [f64; 2],
    /// Its radius.
    r: f64,
    /// How far it leans back, radians.
    tilt: f64,
    /// How white it is (a block), 0 to 1.
    flare: f64,
    /// How solid it is, 0 to 1.
    alpha: f64,
}

impl Cymbal {
    /// The screen outline of the circle of `k` of its radius.
    fn circle(&self, cam: &Cam, k: f64, n: usize) -> Poly {
        let (dx, dz) = (self.dir[0], self.dir[1]);
        let (st, ct) = (dmath::sin(self.tilt), dmath::cos(self.tilt));
        let pts: Vec<V3> = (0..n)
            .map(|j| {
                let a = TAU * j as f64 / n as f64;
                let (ca, sa) = (dmath::cos(a) * self.r * k, dmath::sin(a) * self.r * k);
                scene::to_world(
                    self.c[0] - dz * ca - dx * st * sa,
                    self.c[1] + dx * ca - dz * st * sa,
                    self.c[2] + ct * sa,
                )
            })
            .collect();
        cam.wp(&pts)
    }

    fn draw(&self, arr: &mut Img<f32>, cam: &Cam, t: f64) {
        let a = self.alpha;
        let white = |col: V3| -> V3 { std::array::from_fn(|ch| col[ch] + (bold::WHITE[ch] - col[ch]) * self.flare) };
        let rim = self.circle(cam, 1.0, 28);
        let mut closed = rim.clone();
        closed.push(rim[0]);
        blend_into(arr, &mask_of_all(std::slice::from_ref(&rim)), white(BRONZE), Num::Py(a));
        // the glint: a pale wedge that sweeps round
        let spin = t * 5.0;
        let c = self.circle(cam, 0.0, 1)[0];
        let n = rim.len();
        let j = ((spin / TAU).rem_euclid(1.0) * n as f64) as usize % n;
        let wedge: Poly = vec![c, rim[j], rim[(j + 1) % n], rim[(j + 2) % n]];
        blend_into(arr, &mask_of_all(&[wedge]), bold::WHITE, Num::Py(0.7 * a));
        // the lathe grooves, the bell and the dark edge
        for k in [0.78, 0.55] {
            let mut g = self.circle(cam, k, 24);
            g.push(g[0]);
            blend_into(arr, &mask_of_lines(&[g], Some(1)), white(BRONZE_DK), Num::Py(0.8 * a));
        }
        let bell = self.circle(cam, 0.26, 12);
        let mut bell_edge = bell.clone();
        bell_edge.push(bell[0]);
        blend_into(arr, &mask_of_all(&[bell]), white(bold::pale(BRONZE)), Num::Py(a));
        blend_into(arr, &mask_of_lines(&[bell_edge], Some(1)), bold::INK, Num::Py(0.8 * a));
        blend_into(arr, &mask_of_lines(&[closed], Some(2)), bold::INK, Num::Py(a));
    }
}

/// A drum skin rolling out at radius `r`: the dark edge, the hoop in the drum's colour, the cream skin inside it
/// and, on a main pulse, eight chrome lugs. A pulse of the roll is thinner and has no lugs.
fn draw_skin(arr: &mut Img<f32>, cam: &Cam, q: &Pulse, r: f64, a: f64, col: V3) {
    let (w_hoop, w_skin) = if q.main { (0.09, 0.24) } else { (0.06, 0.10) };
    let ring = |r_in: f64, r_out: f64| mask_of_all(&ring_polys(cam, q.cx, q.cz, r_in.max(R * 0.9), r_out, RING_Y));
    blend_into(arr, &ring(r - w_hoop - 0.03, r + 0.03), bold::INK, Num::Py(a));
    blend_into(
        arr,
        &ring(r - w_hoop - w_skin, r - w_hoop - 0.03),
        SKIN,
        Num::Py(0.85 * a),
    );
    // the collar: a darker line where the skin bends over the edge
    if q.main {
        blend_into(
            arr,
            &ring(r - w_hoop - w_skin - 0.03, r - w_hoop - w_skin),
            bold::deep(SKIN),
            Num::Py(0.5 * a),
        );
    }
    let hoop = col.map(|c| (c * 1.1 + 30.0).min(255.0));
    blend_into(arr, &ring(r - w_hoop, r), hoop, Num::Py(a));
    if q.main && a > 0.3 {
        for j in 0..8 {
            let ang = f64::from(j) * PI / 4.0 + 0.3;
            let s = bold::at(
                cam,
                q.cx + (r - w_hoop * 0.5) * dmath::cos(ang),
                q.cz + (r - w_hoop * 0.5) * dmath::sin(ang),
                RING_Y + 0.02,
            );
            px::ink_disc(arr, s, 2.4, CHROME, a);
        }
    }
}

/// The giant beater's head height above the canvas: it hovers, lands an eighth before the hit, lifts, lands again
/// on the hit, stays down and lifts away; `None` outside that.
fn beater_height(t: f64, t0: f64, t_slam: f64, beat: f64) -> Option<f64> {
    let tap = t_slam - beat / 2.0;
    if t < t0 {
        return None;
    }
    Some(if t < tap - 0.10 {
        3.0 - 0.4 * ((t - t0) / 0.3).min(1.0)
    } else if t < tap {
        2.6 * (1.0 - bold::ease_in((t - (tap - 0.10)) / 0.10))
    } else if t < tap + 0.06 {
        0.0
    } else if t < t_slam - 0.08 {
        2.2 * bold::ease_out((t - tap - 0.06) / (t_slam - 0.08 - tap - 0.06).max(1e-6))
    } else if t < t_slam {
        2.2 * (1.0 - bold::ease_in((t - (t_slam - 0.08)) / 0.08))
    } else if t < t_slam + 0.25 {
        0.0
    } else if t < t_slam + 0.6 {
        3.0 * bold::ease_in((t - t_slam - 0.25) / 0.35)
    } else {
        return None;
    })
}

/// The beater on the finished picture: a chrome shaft up to the pedal's pivot, a squat felt drum of a head with a
/// worn red spot, all outlined.
fn draw_beater(img: &mut Pic, cam: &Cam, cx: f64, cz: f64, head_r: f64, h: f64, a: f64) {
    let thick = 0.5;
    let top = bold::at(cam, cx, cz, h + thick);
    let pivot = bold::at(cam, cx - 1.0, cz + 1.0, h + 3.6);
    px::ink_line(img, top, pivot, 5.0, SHAFT, a);
    let disc = |r: f64, y: f64| -> Vec<(f64, f64)> {
        (0..20)
            .map(|j| {
                let u = TAU * f64::from(j) / 20.0;
                bold::at(cam, cx + r * dmath::cos(u), cz + r * dmath::sin(u), y)
            })
            .collect()
    };
    let (low, high) = (disc(head_r, h), disc(head_r, h + thick));
    // the side: between the widest points of the two rims
    let widest = |pts: &[(f64, f64)]| -> ((f64, f64), (f64, f64)) {
        let l = pts.iter().copied().fold(pts[0], |m, q| if q.0 < m.0 { q } else { m });
        let r = pts.iter().copied().fold(pts[0], |m, q| if q.0 > m.0 { q } else { m });
        (l, r)
    };
    let ((l0, r0), (l1, r1)) = (widest(&low), widest(&high));
    px::ink_poly(img, &low, FELT.map(|v| v * 0.62), a);
    px::ink_poly(img, &[l1, r1, r0, l0], FELT.map(|v| v * 0.74), a);
    px::ink_poly(img, &high, FELT, a);
    px::poly(img, &disc(head_r * 0.42, h + thick), [208.0, 62.0, 54.0], 0.75 * a);
    px::ring(img, bold::at(cam, cx, cz, h + thick), 2.0, 1.0, SHAFT, a);
}

impl DrumsLook {
    /// The pulse that last fired at or before `t` (its number and age) and the seconds until the next.
    fn strokes(&self, t: f64) -> (Option<(usize, f64)>, Option<f64>) {
        let j = self.pulse_t.partition_point(|&v| v <= t);
        (
            j.checked_sub(1).map(|i| (i, t - self.pulse_t[i])),
            self.pulse_t.get(j).map(|&v| v - t),
        )
    }

    /// Its slams at recording frame `i`: `(cx, cz, t0, t_slam, radius)`.
    fn my_slams(&self, p: &PictureCtx, i: usize) -> Vec<(f64, f64, f64, f64, f64)> {
        slams(p, i)
            .into_iter()
            .filter(|s| s.0 == self.k as i64)
            .map(|s| (s.1, s.2, s.3, s.4, s.5))
            .collect()
    }

    /// The cymbal while the shield is up.
    fn cymbal(&self, p: &PictureCtx, fs: &[BallState; 2], t: f64) -> Option<Cymbal> {
        let (st, dur) = (&fs[self.k], self.shield?);
        if st.shield <= 0.0 || st.shattered || !st.vis {
            return None;
        }
        let left = p.bold_shield_left(self.k, t).unwrap_or(st.shield / dur).clamp(0.0, 1.0);
        let up = dur - st.shield;
        let e = fs[1 - self.k].p;
        let face = dmath::atan2(e[1] - st.p[1], e[0] - st.p[0]);
        // it swings round from behind the ball
        let ang = face + PI * (1.0 - bold::ease_out(up / SWING_S));
        let dir = [dmath::cos(ang), dmath::sin(ang)];
        let age = p
            .bold_last_block(self.k, t)
            .map_or(FLARE_S, |(tb, _, _)| (t - tb).min(FLARE_S));
        let flare = (1.0 - age / FLARE_S).max(0.0);
        // the last fifth of the shield: it blinks
        let blink = left < 0.2 && (t * 15.0).floor().rem_euclid(2.0) == 0.0;
        Some(Cymbal {
            c: [st.p[0] + dir[0] * R * 1.45, st.p[1] + dir[1] * R * 1.45, R * 1.1],
            dir,
            r: R * (0.85 + 0.3 * left),
            tilt: 0.8 + 0.3 * flare * dmath::sin(age * 70.0),
            flare,
            alpha: if blink { 0.25 } else { 0.45 + 0.55 * left },
        })
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let col = p.col[k];
        let st = &c.fs[k];
        // the drum's reach: the dashed ring, as ever
        if c.in_fight && st.vis {
            let tint = col.map(|v| v / 255.0 + 0.2);
            for (j, light) in ic.iter_mut().enumerate() {
                let (dx, dz) = (c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                let ring = (1.0 - (dmath::hypot(dx, dz) - (R + st.blen)).abs() / 0.05).clamp(0.0, 1.0);
                if ring > 0.0 && ((dmath::atan2(dz, dx) + PI) * 9.0).floor().rem_euclid(2.0) == 0.0 {
                    for ch in 0..3 {
                        light[ch] += ring * 0.20 * tint[ch];
                    }
                }
            }
        }
        // each pulse: a quake of lifted tiles under the skin
        let t_rings = c.t.min(p.t_ko);
        let tint = bold::light(col);
        for q in self.pulse_list(p, c.t) {
            if let Some((r, a)) = pulse_state(t_rings, q.t0, q.reach, self.speed) {
                floor::band(
                    ic,
                    c,
                    Family::Quake,
                    q.cx,
                    q.cz,
                    r,
                    a,
                    c.t,
                    tint,
                    if q.main { 0.6 } else { 0.3 },
                );
            }
        }
        if !c.in_fight {
            return;
        }
        // CRASH CYMBAL: a block sends a sizzling gold ring out
        if let Some((tb, _, _)) = p.bold_last_block(k, c.t) {
            let u = (c.t - tb) / FLARE_S;
            if (0.0..1.0).contains(&u) && st.shield > 0.0 {
                floor::band(
                    ic,
                    c,
                    Family::Noise,
                    st.p[0],
                    st.p[1],
                    R * 1.3 + 1.3 * bold::ease_out(u),
                    1.0 - u,
                    c.t,
                    bold::light(BRONZE),
                    1.1,
                );
            }
        }
        // DOUBLE KICK: the landing mark closes in, the beater's shadow, a quake on each landing
        let beat = p.show.grid.beat;
        for (cx, cz, t0, t_slam, rad) in self.my_slams(p, c.i) {
            let tap = t_slam - beat / 2.0;
            if c.t < t_slam {
                let u = ((c.t - t0) / (t_slam - t0).max(1e-6)).clamp(0.0, 1.0);
                floor::target(ic, c, cx, cz, rad, u, tint);
            }
            if let Some(h) = beater_height(c.t, t0, t_slam, beat) {
                floor::shade(ic, c, cx, cz, rad * 0.62 * (1.1 - 0.12 * h), 0.5 - 0.1 * h);
            }
            for (t_land, k_land, reach) in [(tap, 0.7, rad * 0.9), (t_slam, 1.2, rad * 1.5)] {
                let u = (c.t - t_land) / QUAKE_S;
                if (0.0..1.0).contains(&u) {
                    floor::band(
                        ic,
                        c,
                        Family::Quake,
                        cx,
                        cz,
                        rad * 0.5 + (reach - rad * 0.5) * bold::ease_out(u),
                        1.0 - u,
                        c.t,
                        tint,
                        k_land,
                    );
                }
            }
        }
    }

    /// The drum skins, on the lit canvas.
    pub(super) fn bold_canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let col = p.col[self.k];
        for q in self.pulse_list(p, f.t) {
            let tt = if q.t0 < p.t_ko { f.t.min(p.t_ko) } else { f.t };
            if let Some((r, a)) = pulse_state(tt, q.t0, q.reach, self.speed) {
                draw_skin(arr, f.cam, &q, r, a, col);
            }
        }
    }

    pub(super) fn bold_items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        match self.cymbal(p, f.fs, f.t) {
            Some(cy) => {
                let depth = f.cam.depth(&[scene::to_world(cy.c[0], cy.c[1], cy.c[2])])[0];
                vec![(depth, Box::new(cy) as Payload)]
            }
            None => Vec::new(),
        }
    }

    pub(super) fn bold_draw_item(&self, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if let Some(cy) = payload.downcast_ref::<Cymbal>() {
            let (mut arr, depth) = (fr.array(), fr.depth());
            cy.draw(&mut arr, f.cam, f.t);
            fr.set_array(&arr, Some(&depth));
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let k = self.k;
        let o = 1 - k;
        let st = &f.fs[k];
        let me = drawn.iter().find(|d| d.k == k);
        // the sticks
        if let Some(d) = me
            && !st.shattered
        {
            self.sticks(p, img, f, d);
        }
        // CRASH CYMBAL: a block throws glints off the cymbal
        if let (Some(cy), Some((tb, _, _))) = (self.cymbal(p, f.fs, f.t), p.bold_last_block(k, f.t)) {
            let u = (f.t - tb) / FLARE_S;
            if (0.0..1.0).contains(&u) {
                let s = bold::at(f.cam, cy.c[0], cy.c[1], cy.c[2]);
                let ppu = bold::px_per_unit(f.cam, cy.c[0], cy.c[1], cy.c[2]);
                let reach = cy.r * ppu * (1.0 + 0.9 * bold::ease_out(u));
                for j in 0..8 {
                    let a = TAU * f64::from(j) / 8.0 + 0.2;
                    let (ca, sa) = (dmath::cos(a), dmath::sin(a));
                    px::ink_line(
                        img,
                        (s.0 + ca * reach * 0.75, s.1 + sa * reach * 0.75),
                        (s.0 + ca * reach, s.1 + sa * reach),
                        1.6,
                        bold::GOLD,
                        1.0 - u,
                    );
                }
                px::glint(img, s, 5.0 + 9.0 * (1.0 - u), bold::WHITE, 1.0 - u);
            }
        }
        // a swatted dart: a pop where the skin caught it, and the dart tumbling away
        for e in bold::events_in(p, o, &["cleared"], f.t - SWAT_S, f.t).filter(|e| e.i(3) == k as i64) {
            let (age, u) = (f.t - e.t, (f.t - e.t) / SWAT_S);
            let s = bold::at(f.cam, e.f(4), e.f(5), 0.5);
            let from = bold::at(f.cam, st.p[0], st.p[1], 0.5);
            let n = dmath::hypot(s.0 - from.0, s.1 - from.1).max(1e-6);
            let (ux, uy) = ((s.0 - from.0) / n, (s.1 - from.1) / n);
            px::ring(
                img,
                s,
                4.0 + 12.0 * bold::ease_out(u),
                1.6,
                bold::pale(p.col[k]),
                1.0 - u,
            );
            px::ink_star(
                img,
                s,
                5.0 + 6.0 * bold::ease_out(u * 2.0),
                2.5,
                6,
                e.t,
                bold::WHITE,
                (1.0 - u * 1.6).max(0.0),
            );
            px::chips(img, s, age, SWAT_S, 3, 18.0, (e.t * 1000.0) as u64, p.col[o]);
            let fly = 26.0 * bold::ease_out(u);
            let c = (s.0 + ux * fly, s.1 + uy * fly - 14.0 * dmath::sin(PI * u));
            let rot = e.t * 7.0 + age * 30.0;
            let (hx, hy) = (5.0 * dmath::cos(rot), 5.0 * dmath::sin(rot));
            px::ink_line(
                img,
                (c.0 - hx, c.1 - hy),
                (c.0 + hx, c.1 + hy),
                2.0,
                p.col[o],
                1.0 - u * u,
            );
        }
        // DOUBLE KICK: the giant beater, and the dust of each landing
        let beat = p.show.grid.beat;
        for (cx, cz, t0, t_slam, rad) in self.my_slams(p, f.i) {
            let head_r = rad * 0.62;
            let foot = bold::at(f.cam, cx, cz, 0.05);
            // the mark on the spot, crisp under the spotlight: a ring closing in on the circle it will hit
            if f.t < t_slam {
                let u = ((f.t - t0) / (t_slam - t0).max(1e-6)).clamp(0.0, 1.0);
                let circle = |r: f64| -> Vec<(f64, f64)> {
                    (0..=32)
                        .map(|j| {
                            let a = TAU * f64::from(j) / 32.0;
                            bold::at(f.cam, cx + r * dmath::cos(a), cz + r * dmath::sin(a), 0.03)
                        })
                        .collect()
                };
                let mark = bold::pale(p.col[k]);
                px::ink_polyline(img, &circle(rad), 1.6, mark, 0.9);
                px::polyline(img, &circle(rad * (2.2 - 1.2 * u)), 1.4, mark, 0.4 + 0.5 * u);
                for j in 0..4 {
                    let a = TAU * f64::from(j) / 4.0;
                    let (ca, sa) = (dmath::cos(a), dmath::sin(a));
                    px::ink_line(
                        img,
                        bold::at(f.cam, cx + rad * 0.75 * ca, cz + rad * 0.75 * sa, 0.03),
                        bold::at(f.cam, cx + rad * 1.2 * ca, cz + rad * 1.2 * sa, 0.03),
                        1.6,
                        mark,
                        0.9,
                    );
                }
            }
            for (t_land, n, reach) in [(t_slam - beat / 2.0, 6, 30.0), (t_slam, 10, 46.0)] {
                px::chips(
                    img,
                    foot,
                    f.t - t_land,
                    QUAKE_S,
                    n,
                    reach,
                    (t_land * 1000.0) as u64,
                    bold::pale(FELT),
                );
            }
            if let Some(h) = beater_height(f.t, t0, t_slam, beat) {
                let fade = (1.0 - (f.t - t_slam - 0.4) / 0.2).clamp(0.0, 1.0);
                let a = ((f.t - t0) / 0.15).min(1.0) * fade;
                draw_beater(img, f.cam, cx, cz, head_r, h, a);
            }
        }
    }

    /// The two sticks over the ball: the one whose stroke is next lifts before it and strikes on the pulse; in a
    /// DRUM ROLL both blur into fans.
    fn sticks(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, d: &DrawnBall) {
        let tt = f.t.min(p.t_ko);
        let (last, next) = self.strokes(tt);
        let roll = f.fs[self.k].haste > 0.0;
        let (rest, down, high) = (-0.3, 0.42, -1.0);
        for hand in 0..2usize {
            // the angle of the stick from level, positive tips it down onto the drum
            let mut ang = rest;
            let mut struck = None;
            if let Some((i, age)) = last
                && i % 2 == hand
            {
                if age < DOWN_S {
                    ang = down;
                    struck = Some(age);
                } else {
                    ang = down + (rest - down) * bold::ease_out((age - DOWN_S) / 0.1);
                }
            }
            if let Some(dt) = next
                && last.map_or(0, |(i, _)| i + 1) % 2 == hand
                && dt < LIFT_S
            {
                ang = rest + (high - rest) * bold::ease_out(1.0 - dt / LIFT_S);
            }
            let side = if hand == 0 { -1.0 } else { 1.0 };
            let grip = (d.s.0 + side * d.rp * 1.2, d.s.1 - d.rp * 1.25);
            let len = d.rp * 1.15;
            let tip_at = |a: f64| (grip.0 - side * len * dmath::cos(a), grip.1 + len * dmath::sin(a));
            if roll {
                // the blur: ghosts of the stroke fanned between up and down
                for (g, a) in [(high * 0.8, 0.3), (rest, 0.4), (down, 0.5)] {
                    px::line(img, grip, tip_at(g), 3.0, bold::pale(HICKORY), a);
                }
            }
            let tip = tip_at(ang);
            px::ink_line(img, grip, tip, 3.6, HICKORY, 1.0);
            px::ink_disc(img, tip, 2.4, bold::pale(HICKORY), 1.0);
            if let Some(age) = struck {
                px::star(img, tip, 7.0, 2.5, 4, 0.4, bold::WHITE, 1.0 - age / DOWN_S * 0.6);
            }
        }
    }

    /// A main pulse is a solid hit, a pulse of the roll a tap; the kick is always heavy.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match bold::how(e) {
            "slam" => Some(HitSize::Heavy),
            _ => Some(by_damage),
        }
    }
}
