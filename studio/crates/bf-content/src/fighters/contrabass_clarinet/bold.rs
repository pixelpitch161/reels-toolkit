//! CONTRABASS CLARINET with the bold effects (`picture.effects = "bold"`; the owner's picks 31a to 31d of the
//! 2026-11-24 review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it
//! nothing here runs.
//!
//! - **FOGHORN** (31a): before each blast a dashed cone on the floor shows where it will go; the blast is three
//!   outlined banks of fog rolling out along the lit cone; the shoved enemy trails streaks, and its slam into the
//!   rope is the heavy hit, in a burst of fog.
//! - **FOG BANK** (31b): outlined puffs rise round the ball and a veil fades it; a blocked attack is shown passing
//!   straight through, a dart out of the far side; as the shield ends the puffs drift apart and thin out.
//! - **MOTORBOAT** (31c): on top of the ring's speed-up look, a dashed V of wake lines trails on the floor behind
//!   the ball, with outlined puffs popping in it.
//! - **SHIP'S HORN** (31d): while it charges, rings close on the ball, the bell glints and a lane with running
//!   dashes shows the aim; the blast is a wide pale beam with bowed bands marching down it over a lit floor; it
//!   thins out and fades.

use bf_core::dmath;
use bf_core::event::Event;
use bf_picture::bold::floor;
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Pic};
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::R_BALL as R;

use super::{BEAM_FADE, Beam, ContraLook, FOG, Horn, circle, toward_white, wedge, world};
use bf_core::roster::FighterCfg;

/// Seconds before a blast its cone shows on the floor.
const WARN_S: f64 = 0.5;
/// Units a second the fog banks roll out.
const ROLL: f64 = 4.0;
/// Seconds the fog takes to rise as FOG BANK starts, and to clear when it ends.
const RISE_S: f64 = 0.35;
const CLEAR_S: f64 = 0.4;
/// Seconds a blocked attack takes to pass through the fog.
const PASS_S: f64 = 0.26;
/// Seconds the fog bursts where the enemy slams the rope.
const SLAM_S: f64 = 0.4;

/// `v` (0 to 1) cut into three dithered steps at canvas pixel `j`, as all light on the canvas is.
fn steps(c: &CanvasCtx<'_>, j: usize, v: f64) -> f64 {
    (v.clamp(0.0, 1.0) * 3.0 + BAYER[c.ys[j] % 4][c.xs[j] % 4] * 0.9).floor() / 3.0
}

/// A cone of light on the floor from `at` along `aim`, `half` radians each side, from radius `r0` to `r1`: a
/// fill of strength `fill`, an edge of strength `edge` (dashes running outward when `dashed`), and bright arcs
/// at the radii `fronts`.
#[allow(clippy::too_many_arguments)]
fn cone_light(
    ic: &mut [V3],
    c: &CanvasCtx<'_>,
    at: [f64; 2],
    aim: f64,
    half: f64,
    r0: f64,
    r1: f64,
    tint: V3,
    fill: f64,
    edge: f64,
    dashed: bool,
    fronts: &[(f64, f64)],
) {
    for (j, light) in ic.iter_mut().enumerate() {
        let (dx, dz) = (c.sxp[j] - at[0], c.szp[j] - at[1]);
        let rd = dmath::hypot(dx, dz);
        if rd < r0 || rd > r1 {
            continue;
        }
        let da = ((dmath::atan2(dz, dx) - aim + dmath::PI).rem_euclid(dmath::TAU) - dmath::PI).abs();
        if da > half {
            continue;
        }
        let mut v = fill * (1.0 - 0.5 * (rd - r0) / (r1 - r0).max(1e-6));
        let on_side = (half - da) * rd < 0.07;
        let on_rim = r1 - rd < 0.08;
        if (on_side && (!dashed || (rd / 0.34 - c.t * 3.0).rem_euclid(1.0) < 0.6)) || on_rim {
            v = v.max(edge);
        }
        for &(r, k) in fronts {
            v = v.max((1.0 - (rd - r).abs() / 0.13) * k);
        }
        let s = steps(c, j, v);
        if s > 0.0 {
            for ch in 0..3 {
                light[ch] += s * tint[ch];
            }
        }
    }
}

/// A bank of fog: the union of the puffs `(centre, radius, height)`, pale inside a dark outline.
fn fog_bank(arr: &mut Img<f32>, cam: &Cam, puffs: &[([f64; 2], f64, f64)], col: V3, a: f64) {
    if a <= 0.0 || puffs.is_empty() {
        return;
    }
    let rim: Vec<Vec<(f64, f64)>> = puffs
        .iter()
        .map(|&(q, r, y)| cam.wp(&circle(q, r + 0.035, y, 14)))
        .collect();
    let body: Vec<Vec<(f64, f64)>> = puffs.iter().map(|&(q, r, y)| cam.wp(&circle(q, r, y, 14))).collect();
    ring::layer_blend(arr, &rim, bold::INK, (0.75 * a).min(1.0), None);
    ring::layer_blend(arr, &body, col, a.min(1.0), None);
}

/// The roster numbers the bold moves read, taken in `setup_as` from its `cfg`: for its own side that is
/// `p.cfg[k]`, and EUPHONIUM's copy of this look gets the copied fighter's numbers.
#[derive(Clone, Debug, Default)]
pub(super) struct Nums {
    /// FOG BANK's beats.
    fog_beats: f64,
}

impl Nums {
    /// The numbers of the roster entry `cfg`.
    pub(super) fn of(cfg: &FighterCfg) -> Nums {
        Nums {
            fog_beats: cfg.raw["abilities"]["unlock1"]["dur_beats"].as_f64().unwrap_or(6.0),
        }
    }
}

impl ContraLook {
    /// The blast as three banks of fog rolling out along the cone.
    fn draw_bold_horn(arr: &mut Img<f32>, cam: &Cam, c: [f64; 2], h: Horn, fade: f64, t: f64, col: V3) {
        let y = R * 0.7;
        let (r0, r1) = (R * 0.9, 2.0 * R + h.reach);
        let open = ((t - h.t0) / 0.08).min(1.0);
        let r1 = r0 + (r1 - r0) * open;
        let body = cam.wp(&wedge(c, h.aim, h.half, r0, r1, y));
        ring::layer_blend(arr, &[body], toward_white(col, 0.3), 0.3 * fade, None);
        let span = (r1 - r0).max(1e-3);
        for j in 0..3 {
            let r = r0 + ((t - h.t0) * ROLL + span * f64::from(j) / 3.0) % span;
            let far = (r - r0) / span;
            let n = 3 + (r * h.half * 2.0 / 0.34) as usize;
            let puffs: Vec<([f64; 2], f64, f64)> = (0..n)
                .map(|i| {
                    let a = h.aim - h.half + 2.0 * h.half * (i as f64 + 0.5) / n as f64;
                    let roll = 0.03 * dmath::sin(t * 11.0 + i as f64 * 1.7 + f64::from(j));
                    let pr = 0.17 + 0.08 * far + roll;
                    ([c[0] + r * dmath::cos(a), c[1] + r * dmath::sin(a)], pr, y)
                })
                .collect();
            fog_bank(arr, cam, &puffs, toward_white(FOG, 0.35), fade * (1.0 - 0.45 * far));
            // the crest of the bank, bright
            let crest = cam.wp(&wedge(c, h.aim, h.half * 0.9, r + 0.05, r + 0.11, y + 0.02));
            ring::layer_blend(arr, &[crest], bold::WHITE, 0.8 * fade * (1.0 - 0.6 * far), None);
        }
    }

    /// FOG BANK: outlined puffs rising round the ball and a veil over it. `rise` is how far the fog has come up
    /// (0 to 1), `clear` how far it has drifted apart (0 to 1).
    fn draw_bold_fog(arr: &mut Img<f32>, cam: &Cam, c: [f64; 2], rise: f64, clear: f64, t: f64) {
        let k = rise * (1.0 - clear);
        if k <= 0.0 {
            return;
        }
        let veil = cam.wp(&circle(c, R * 1.2, R, 18));
        ring::layer_blend(arr, &[veil], FOG, 0.5 * k, None);
        let puffs: Vec<([f64; 2], f64, f64)> = (0..9)
            .map(|j| {
                let jf = f64::from(j);
                let a = 0.7 * jf + 0.35 * t;
                // each puff climbs from the floor and thins as it goes, over and over
                let up = (0.45 * t + 0.37 * jf).rem_euclid(1.0);
                let d = (0.62 + 0.12 * f64::from(j * 3 % 5)) * (1.0 + 1.3 * clear);
                let r =
                    (0.3 + 0.07 * f64::from(j % 3)) * (0.35 + 0.65 * rise) * (1.0 - 0.45 * up) * (1.0 - 0.6 * clear);
                (
                    [c[0] + d * dmath::cos(a), c[1] + d * dmath::sin(a)],
                    r,
                    R * (0.2 + 1.1 * up * rise + 0.8 * clear),
                )
            })
            .collect();
        fog_bank(arr, cam, &puffs, toward_white(FOG, 0.3), 0.9 * k.sqrt());
    }

    /// MOTORBOAT's puffs where the ball has just been, outlined.
    fn draw_bold_wake(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let puffs: Vec<([f64; 2], f64, f64)> = (1..=5usize)
            .filter_map(|j| {
                let q = p.run.p(f.i.checked_sub(j * 5)?, self.k);
                let pulse = 0.7 + 0.3 * dmath::sin(f.t * 40.0 + j as f64);
                Some((q, (0.12 + 0.05 * j as f64) * pulse, R * 0.3))
            })
            .collect();
        fog_bank(arr, f.cam, &puffs, toward_white(p.col[self.k], 0.6), 0.6);
    }

    /// SHIP'S HORN: the wide pale beam with bowed bands marching down it. `k_` fades it and `thin` (1 to 0)
    /// narrows it as it dies.
    #[allow(clippy::too_many_arguments)]
    fn draw_bold_beam(arr: &mut Img<f32>, cam: &Cam, b: Beam, k_: f64, thin: f64, ext: f64, age: f64, col: V3) {
        let d = [dmath::cos(b.a), dmath::sin(b.a)];
        let nrm = [-d[1], d[0]];
        let y = R * 0.8;
        let s1 = (b.len + R) * ext;
        let at = |s: f64, v: f64| world([b.o[0] + d[0] * s + nrm[0] * v, b.o[1] + d[1] * s + nrm[1] * v], y);
        let shaft = |w: f64| cam.wp(&[at(0.0, w * 0.4), at(s1, w), at(s1, -w), at(0.0, -w * 0.4)]);
        let half = b.wd / 2.0 * thin;
        ring::layer_blend(arr, &[shaft(half * 1.5 + 0.04)], bold::INK, 0.35 * k_, None);
        for (wk, c, a) in [
            (1.5, toward_white(col, 0.45), 0.45),
            (1.0, toward_white(col, 0.75), 0.55),
            (0.4, bold::WHITE, 0.85),
        ] {
            ring::layer_blend(arr, &[shaft(half * wk)], c, a * k_, None);
        }
        // the ring bands: bowed bars across the shaft, marching out
        let gap = 0.75;
        let n = (s1 / gap) as usize + 1;
        let bands: Vec<Vec<(f64, f64)>> = (0..n)
            .filter_map(|j| {
                let s = (age * 7.0).rem_euclid(gap) + gap * j as f64;
                if s + 0.3 > s1 {
                    return None;
                }
                let w = half * 1.5 * (0.4 + 0.6 * s / s1.max(1e-6));
                let bow = |v: f64| 0.22 * (1.0 - v * v);
                let front = (-4..=4).map(|i| f64::from(i) / 4.0);
                let mut pts: Vec<V3> = front.clone().map(|v| at(s + bow(v) + 0.16, w * v)).collect();
                pts.extend(front.rev().map(|v| at(s + bow(v), w * v)));
                Some(cam.wp(&pts))
            })
            .collect();
        ring::layer_blend(arr, &bands, col, 0.9 * k_, None);
    }

    /// The beam's fade and width at `u` seconds after it fired: `(strength, width share)`.
    fn beam_shape(&self, u: f64) -> (f64, f64) {
        if u <= self.hold {
            (1.0, 1.0)
        } else {
            let v = ((u - self.hold) / BEAM_FADE).clamp(0.0, 1.0);
            (1.0 - v, 1.0 - bold::ease_in(v) * 0.85)
        }
    }

    /// The SHIP'S HORN charge under way at `t`: how far it is (0 to 1), its aim, length and width.
    fn charge(&self, p: &PictureCtx, t: f64) -> Option<(f64, f64, f64, f64)> {
        let (age, e) = bold::since(p, self.k, &["charge"], t, 3.0)?;
        if e.fields.len() < 5 {
            return None;
        }
        let tc = e.f(4).max(1e-6);
        (age < tc && t < p.t_ko).then(|| (age / tc, e.f(3), e.f(5), e.f(6)))
    }

    /// FOG BANK at `t`: how far the fog has risen and how far it has cleared.
    fn fog(&self, p: &PictureCtx, shield: f64, t: f64) -> Option<(f64, f64)> {
        let beats = self.nums.fog_beats;
        let dur = beats * p.show.grid.beat;
        let (age, _) = bold::since(p, self.k, &["ability:shield"], t, dur + CLEAR_S)?;
        if shield > 0.0 {
            // it starts to drift apart a little before the shield drops
            let clear = (1.0 - shield / (CLEAR_S * 0.5)).max(0.0) * 0.5;
            Some((bold::ease_out(age / RISE_S), clear))
        } else if age >= dur - 0.05 {
            Some((1.0, 0.5 + 0.5 * ((age - dur) / (CLEAR_S * 0.5)).clamp(0.0, 1.0)))
        } else {
            None
        }
    }

    /// The light on the floor: the cone before and under a blast, MOTORBOAT's wake, SHIP'S HORN's aim and glow.
    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight || c.t > p.t_ko {
            return;
        }
        let k = self.k;
        let st = &c.fs[k];
        if !st.vis || st.shattered {
            return;
        }
        let col = p.col[k];
        let tint = bold::light(toward_white(col, 0.45));
        // the cone where the next blast will go, brightening as it comes
        if let Some((u, e)) = bold::until(p, k, &["horn"], c.t, WARN_S)
            && e.fields.len() >= 5
        {
            let (aim, reach, half) = (e.f(3), e.f(4), e.f(5));
            cone_light(
                ic,
                c,
                st.p,
                aim,
                half,
                R * 0.9,
                2.0 * R + reach,
                tint,
                0.12 + 0.2 * u,
                0.5 + 0.5 * u,
                true,
                &[],
            );
        }
        // the blast lights its cone, a bright arc under each bank of fog
        if let Some((h, fade)) = self.horn_at(c.t) {
            let (r0, r1) = (R * 0.9, 2.0 * R + h.reach);
            let span = (r1 - r0).max(1e-3);
            let fronts: Vec<(f64, f64)> = (0..3)
                .map(|j| {
                    let r = r0 + ((c.t - h.t0) * ROLL + span * f64::from(j) / 3.0) % span;
                    (r, 0.9 * fade)
                })
                .collect();
            cone_light(
                ic,
                c,
                st.p,
                h.aim,
                h.half,
                r0,
                r1,
                tint,
                0.35 * fade,
                0.7 * fade,
                false,
                &fronts,
            );
        }
        // MOTORBOAT: a V of wake lines on the floor behind the ball
        if st.haste > 0.0 {
            let pts: Vec<[f64; 2]> = (0..=6usize).map(|j| p.run.p(c.i.saturating_sub(j * 3), k)).collect();
            let arm = |j: usize, side: f64| -> Option<[f64; 2]> {
                if j == 0 {
                    return Some(pts[0]);
                }
                let (dx, dz) = (pts[j - 1][0] - pts[j][0], pts[j - 1][1] - pts[j][1]);
                let n = dmath::hypot(dx, dz);
                let w = side * (0.12 + 0.12 * j as f64);
                (n > 0.02).then(|| [pts[j][0] - dz / n * w, pts[j][1] + dx / n * w])
            };
            let wake = bold::light(toward_white(col, 0.7));
            for side in [-1.0, 1.0] {
                for j in 0..6 {
                    if let (Some(a), Some(b)) = (arm(j, side), arm(j + 1, side)) {
                        floor::dashes(ic, c, b, a, 0.05, 0.2, c.t * 9.0, wake, 1.2 * (1.0 - j as f64 / 7.0));
                    }
                }
            }
        }
        // SHIP'S HORN charging: the lane the blast will fill, widening, with dashes running down it
        if let Some((u, aim, len, wd)) = self.charge(p, c.t) {
            let d = [dmath::cos(aim), dmath::sin(aim)];
            let a = [st.p[0] + d[0] * R, st.p[1] + d[1] * R];
            let b = [st.p[0] + d[0] * (R + len), st.p[1] + d[1] * (R + len)];
            floor::lane(ic, c, a, b, wd / 2.0 * (0.25 + 0.75 * u), tint, 0.2 + 0.4 * u);
            floor::dashes(
                ic,
                c,
                a,
                b,
                0.06,
                0.5,
                c.t * 7.0,
                bold::light(bold::WHITE),
                0.8 + 0.4 * u,
            );
        }
        // and the blast lights the floor under it
        for b in &self.beams {
            let u = c.t - b.t;
            if !(0.0..self.hold + BEAM_FADE).contains(&u) {
                continue;
            }
            let (k_, thin) = self.beam_shape(u);
            let d = [dmath::cos(b.a), dmath::sin(b.a)];
            let s1 = (b.len + R) * (u / 0.06).min(1.0);
            floor::lane(
                ic,
                c,
                b.o,
                [b.o[0] + d[0] * s1, b.o[1] + d[1] * s1],
                b.wd * 0.8 * thin,
                tint,
                0.9 * k_,
            );
        }
    }

    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let (cam, t, k) = (f.cam, f.t, self.k);
        let st = &f.fs[k];
        if !st.vis || st.shattered {
            return;
        }
        let col = p.col[k];
        if f.in_fight
            && let Some((rise, clear)) = self.fog(p, st.shield, t)
        {
            Self::draw_bold_fog(arr, cam, st.p, rise, clear, t);
        }
        if f.in_fight && st.haste > 0.0 {
            self.draw_bold_wake(p, f, arr);
        }
        if let Some((h, fade)) = self.horn_at(t).filter(|_| t <= p.t_ko) {
            Self::draw_bold_horn(arr, cam, st.p, h, fade, t, col);
        }
        for b in &self.beams {
            let u = t - b.t;
            if u < 0.0 || u > self.hold + BEAM_FADE {
                continue;
            }
            let (k_, thin) = self.beam_shape(u);
            Self::draw_bold_beam(arr, cam, *b, k_, thin, (u / 0.06).min(1.0), u, col);
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight || f.t > p.t_ko {
            return;
        }
        let k = self.k;
        let o = 1 - k;
        let (cam, t) = (f.cam, f.t);
        let me = drawn.iter().find(|d| d.k == k);
        let foe = drawn.iter().find(|d| d.k == o);
        let col = p.col[k];
        let pale = toward_white(FOG, 0.4);
        // the shoved enemy trails streaks until it meets the rope
        if let Some(d) = foe
            && let Some((age, e)) = bold::since(p, k, &["hit:foghorn"], t, 0.5)
            && bold::events_in(p, k, &["hit:wall"], e.t, t).next().is_none()
        {
            let prev = p.run.p(p.sim_i((t - 2.0 / 30.0).max(e.t)), o);
            let now = f.fs[o].p;
            let (s0, s1) = (bold::at(cam, prev[0], prev[1], R), bold::at(cam, now[0], now[1], R));
            px::speed_lines(
                img,
                d.s,
                d.rp,
                (s1.0 - s0.0, s1.1 - s0.1),
                24.0,
                7,
                pale,
                1.0 - age / 0.5,
            );
        }
        // the slam into the rope: a burst of fog
        for e in bold::events_in(p, k, &["hit:wall"], t - SLAM_S, t) {
            let age = t - e.t;
            let u = age / SLAM_S;
            let c = bold::at(cam, e.f(5), e.f(6), R);
            for j in 0..6u64 {
                let a = dmath::TAU * (j as f64 + px::rnd(j, 5)) / 6.0;
                let far = (14.0 + 8.0 * px::rnd(j, 9)) * bold::ease_out(u);
                px::ink_disc(
                    img,
                    (c.0 + dmath::cos(a) * far, c.1 + dmath::sin(a) * far * 0.7 - 6.0 * u),
                    (6.0 + 3.0 * px::rnd(j, 2)) * (1.0 - 0.6 * u),
                    pale,
                    ((1.0 - u) * 2.0).min(1.0),
                );
            }
            px::chips(
                img,
                c,
                age,
                SLAM_S,
                8,
                38.0,
                (e.t * 1000.0) as u64,
                toward_white(col, 0.5),
            );
        }
        // FOG BANK: a blocked attack passes straight through, a dart out of the far side
        if let (Some(d), Some(a)) = (me, foe)
            && let Some((tb, _, _)) = p.bold_last_block(k, t)
            && t - tb < PASS_S
        {
            let u = (t - tb) / PASS_S;
            let (dx, dy) = (d.s.0 - a.s.0, d.s.1 - a.s.1);
            let n = dmath::hypot(dx, dy).max(1e-6);
            let (ux, uy) = (dx / n, dy / n);
            let on = |s: f64| (d.s.0 + ux * d.rp * s, d.s.1 + uy * d.rp * s);
            // the way it went, dashed, and the dart itself
            let head = -1.2 + 3.6 * bold::ease_out(u);
            let mut s = -1.4;
            while s < head - 0.5 {
                px::ink_line(img, on(s), on((s + 0.3).min(head - 0.5)), 1.6, bold::WHITE, 1.0 - u);
                s += 0.55;
            }
            px::ink_arrow(
                img,
                on(head - 0.9),
                on(head),
                3.0,
                9.0,
                bold::pale(p.col[o]),
                ((1.0 - u) * 3.0).min(1.0),
            );
            // the fog curls where it leaves
            if u > 0.25 {
                let v = (u - 0.25) / 0.75;
                for side in [-1.0, 1.0] {
                    let q = on(1.15);
                    px::ink_disc(
                        img,
                        (
                            q.0 - uy * side * (5.0 + 9.0 * v) + ux * 6.0 * v,
                            q.1 + ux * side * (5.0 + 9.0 * v) + uy * 6.0 * v,
                        ),
                        5.0 * (1.0 - 0.6 * v),
                        pale,
                        1.0 - v,
                    );
                }
            }
        }
        // SHIP'S HORN charging: rings close on the ball and the bell glints
        if let (Some(d), Some((u, aim, len, _))) = (me, self.charge(p, t)) {
            for j in 0..3 {
                let ph = (u * 3.0 + f64::from(j) / 3.0).rem_euclid(1.0);
                px::ring(img, d.s, d.rp * (2.7 - 1.6 * ph), 2.0, toward_white(col, 0.7), ph);
            }
            let at = f.fs[k].p;
            let tip = bold::at(cam, at[0] + dmath::cos(aim) * R, at[1] + dmath::sin(aim) * R, R);
            // the aim, as dashes running out from the bell (the lane on the floor ends at the ropes)
            let on = |s: f64| bold::at(cam, at[0] + dmath::cos(aim) * s, at[1] + dmath::sin(aim) * s, R * 0.8);
            let mut s = R + 0.2 + 0.5 * (t * 4.0).rem_euclid(1.0);
            while s + 0.25 < R + len {
                px::ink_line(img, on(s), on(s + 0.25), 1.6, toward_white(col, 0.75), 0.45 + 0.5 * u);
                s += 0.5;
            }
            px::glint(img, tip, 5.0 + 9.0 * u, bold::WHITE, 1.0);
        }
        // the blast's bright core, over the spotlight's dim, and a flare at the bell as it fires
        for b in &self.beams {
            let u = t - b.t;
            if !(0.0..self.hold + BEAM_FADE).contains(&u) {
                continue;
            }
            let (k_, thin) = self.beam_shape(u);
            let d = [dmath::cos(b.a), dmath::sin(b.a)];
            let s1 = (b.len + R) * (u / 0.06).min(1.0);
            let (a, e) = (
                bold::at(cam, b.o[0], b.o[1], R * 0.8),
                bold::at(cam, b.o[0] + d[0] * s1, b.o[1] + d[1] * s1, R * 0.8),
            );
            px::line(img, a, e, 1.0 + 5.0 * thin, bold::WHITE, 0.85 * k_);
            if u < 0.15 {
                px::ink_star(img, a, 16.0 * (1.0 - u / 0.15) + 6.0, 5.0, 8, u * 9.0, bold::WHITE, 1.0);
            }
        }
    }

    /// The slam into the rope is the heavy hit; the blast's own touch is a tap.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match bold::how(e) {
            "wall" => Some(HitSize::Heavy),
            "foghorn" => Some(HitSize::Tap),
            _ => Some(by_damage),
        }
    }
}
