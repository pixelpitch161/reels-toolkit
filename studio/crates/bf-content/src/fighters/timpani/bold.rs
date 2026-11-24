//! TIMPANI with the bold effects (`picture.effects = "bold"`; the owner's picks 30a to 30d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **TWO KETTLES** (30a): the kettles are outlined copper bowls with a hoop in their own colour (the low one
//!   purple, the high one yellow); the ball hops higher and trails rings through its arc, and lands in a puff;
//!   the low kettle sends a wide, slow purple quake, the high one a tight double yellow ring; the interval hit
//!   bursts in both kettles' colours at once.
//! - **PEDAL SLIDE** (30b): the gliding kettles leave dark skid marks, arrows run ahead of them at the enemy, and
//!   the squeeze pinches the enemy thin between two arrowheads.
//! - **CROSS-STICKING** (30c): on top of the ring's speed-up look, two crossed sticks sit over the ball and tap
//!   in turn on every 8th.
//! - **FULL SET** (30d): the two new kettles fall in over growing shadows and land in dust; each of the four has
//!   its own colour of ring; on every bar line all four heads ring out together.

use bf_core::dmath;
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Phase, Pic};
use bf_picture::ring::state::BallState;
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::{R_BALL as R, to_world};

use super::{CHROME, COPPER, COPPER_DK, COPPER_HI, DUST, HEAD, TimpaniLook, band, circle, hop_state, land_squash};

/// The low kettle's purple.
const LOW: V3 = [176.0, 112.0, 255.0];
/// The high kettle's yellow.
const HIGH: V3 = [255.0, 226.0, 84.0];
/// FULL SET's two kettles: teal and coral.
const MID: [V3; 2] = [[84.0, 222.0, 208.0], [255.0, 140.0, 104.0]];
/// A stick's wood.
const WOOD: V3 = [228.0, 188.0, 124.0];
/// How much higher the ball hops in the bold picture.
const HOP_K: f64 = 1.5;
/// The height of a kettle's head above the floor in the bold picture.
const KETTLE_H: f64 = 0.24;
/// Seconds FULL SET's kettles take to fall in, and the height they fall from.
const DROP_S: f64 = 0.28;
const DROP_H: f64 = 4.0;
/// Seconds the interval's two-colour burst lasts.
const BURST_S: f64 = 0.32;
/// Seconds the squeeze pinches the enemy.
const PINCH_S: f64 = 0.2;
/// Seconds the four heads ring out together on a bar line.
const TUTTI_S: f64 = 0.32;

/// The colour of kettle `i`: 0 low, 1 high, 2 and 3 FULL SET's.
fn pitch_col(i: i64) -> V3 {
    match i {
        0 => LOW,
        1 => HIGH,
        n => MID[(n.rem_euclid(2)) as usize],
    }
}

/// `a` to `b`, `u` of the way.
fn mix(a: V3, b: V3, u: f64) -> V3 {
    std::array::from_fn(|ch| a[ch] + (b[ch] - a[ch]) * u.clamp(0.0, 1.0))
}

/// A broken dark streak on the canvas from `a` to `b`: a skid mark.
fn skid(ic: &mut [V3], c: &CanvasCtx<'_>, a: [f64; 2], b: [f64; 2], half_w: f64, k: f64) {
    let (vx, vz) = (b[0] - a[0], b[1] - a[1]);
    let len2 = vx * vx + vz * vz;
    if len2 < 1e-6 || k <= 0.0 {
        return;
    }
    let len = len2.sqrt();
    for (j, light) in ic.iter_mut().enumerate() {
        let (x, z) = (c.sxp[j] - a[0], c.szp[j] - a[1]);
        let u = ((x * vx + z * vz) / len2).clamp(0.0, 1.0);
        let d = dmath::hypot(x - u * vx, z - u * vz);
        if d < half_w && (u * len / 0.22).rem_euclid(1.0) < 0.7 {
            for ch in light.iter_mut() {
                *ch *= 1.0 - k.min(1.0);
            }
        }
    }
}

/// One kettle in the bold picture, its head `y` above the floor: a shadow, an outlined copper bowl that narrows
/// to its foot, a hoop in its own colour `col`, the head (lit in that colour by `flash`) and the rod heads.
#[allow(clippy::too_many_arguments)]
fn draw_bold_kettle(arr: &mut Img<f32>, cam: &Cam, x: f64, z: f64, r: f64, y: f64, flash: f64, col: V3) {
    let disc = |arr: &mut Img<f32>, rr: f64, h: f64, c: V3| {
        ring::layer_blend(arr, &[circle(cam, x, z, rr, h, 32)], c, 1.0, None);
    };
    disc(arr, r * 0.8, y + 0.005, bold::INK);
    disc(arr, r * 0.72, y + 0.01, COPPER_DK);
    disc(arr, r * 1.08, y + KETTLE_H * 0.6, bold::INK);
    disc(arr, r * 0.98, y + KETTLE_H * 0.6, COPPER);
    disc(arr, r * 1.09, y + KETTLE_H, bold::INK);
    disc(arr, r, y + KETTLE_H, COPPER_HI);
    disc(arr, r * 0.94, y + KETTLE_H, mix(col, CHROME, 0.15));
    disc(arr, r * 0.83, y + KETTLE_H, bold::INK);
    disc(arr, r * 0.8, y + KETTLE_H, mix(HEAD, bold::pale(col), flash));
    let rods: Vec<Vec<(f64, f64)>> = (0..8)
        .map(|j| {
            let a = dmath::PI / 8.0 + dmath::PI * f64::from(j) / 4.0;
            let (cx, cz) = (x + r * 0.92 * dmath::cos(a), z + r * 0.92 * dmath::sin(a));
            let d = r * 0.07;
            let h = y + KETTLE_H + 0.01;
            ring::screen(
                cam,
                &[
                    to_world(cx - d, cz - d, h),
                    to_world(cx + d, cz - d, h),
                    to_world(cx + d, cz + d, h),
                    to_world(cx - d, cz + d, h),
                ],
            )
        })
        .collect();
    ring::layer_blend(arr, &rods, CHROME, 1.0, None);
}

impl TimpaniLook {
    /// Its kettles in recording frame `i`: `(index, x, z, radius)`.
    fn kettles(&self, p: &PictureCtx, i: usize) -> Vec<(i64, f64, f64, f64)> {
        self.rows(p, "KETTLES", i)
            .iter()
            .filter_map(|r| match r[..] {
                [_, ki, x, z, kr] => Some((ki as i64, x, z, kr)),
                _ => None,
            })
            .collect()
    }

    /// When FULL SET's kettles land: [`DROP_S`] after the move, or sooner when one of them already booms.
    fn full_land(&self) -> Option<(f64, f64)> {
        let t0 = self.full?;
        let first = self
            .booms
            .iter()
            .find(|b| b.t >= t0 && b.kettle >= 2)
            .map_or(f64::INFINITY, |b| b.t);
        Some((t0, (t0 + DROP_S).min(first).max(t0 + 0.08)))
    }

    /// The height FULL SET's kettle `ki` still has to fall at `t` (0 once it stands, and for the first two).
    fn drop_y(&self, ki: i64, t: f64) -> f64 {
        match self.full_land() {
            Some((t0, t1)) if ki >= 2 && t < t1 => DROP_H * (1.0 - bold::ease_in((t - t0) / (t1 - t0))),
            _ => 0.0,
        }
    }

    /// The seconds since the last bar line on which all four kettles ring out together (FULL SET, from its
    /// second beat on).
    fn tutti(&self, p: &PictureCtx, t: f64) -> Option<f64> {
        let t0 = self.full?;
        let g = &p.show.grid;
        let bar = 4.0 * g.beat;
        let age = (t - g.t_fight).rem_euclid(bar);
        (t - age > t0 + g.beat * 0.5 && age < TUTTI_S && t < p.t_ko).then_some(age)
    }

    /// The hop higher and the landing harder; the enemy pinched thin by the squeeze.
    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase) {
        if !matches!(phase, Phase::Fight) || t >= p.t_ko {
            return;
        }
        if kk != self.k {
            if let Some((age, _)) = bold::since(p, self.k, &["hit:squeeze"], t, PINCH_S) {
                let k = 1.0 - age / PINCH_S;
                st.sx = Some(st.sx.unwrap_or(1.0) * (1.0 - 0.3 * k));
                st.sy = Some(st.sy.unwrap_or(1.0) * (1.0 + 0.22 * k));
            }
            return;
        }
        if let Some((h, u)) = self.hop_at(t) {
            let i0 = p.sim_i(h.t);
            let i1 = p.sim_i(h.t1.min(p.t_ko));
            let (a, b) = (p.run.p(i0, kk), p.run.p(i1, kk));
            let (lift, sx, sy) = hop_state(u, dmath::hypot(b[0] - a[0], b[1] - a[1]));
            st.lift = lift * HOP_K;
            st.sx = Some(sx);
            st.sy = Some(sy);
            if lift > 0.02 {
                st.air_bias = 0.6;
                st.paint_h = Some(R);
            }
        } else {
            let (sx, sy) = land_squash(self.since_land(t));
            st.sx = Some(1.0 + (sx - 1.0) * 1.2);
            st.sy = Some(1.0 + (sy - 1.0) * 1.2);
        }
    }

    /// The light on the floor: skid marks, the kettles' rings each in its own colour, landing puffs, the shadows
    /// of FULL SET's falling kettles.
    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight || c.t >= p.t_hs {
            return;
        }
        let t = c.t.min(p.t_ko);
        let i = p.sim_i(t);
        let beat = p.show.grid.beat;
        // PEDAL SLIDE: two dark skid marks behind each gliding kettle, gone two beats after the slide
        for &(t0, t1) in &self.slides {
            let fade = 1.0 - ((c.t - t1) / (2.0 * beat)).max(0.0);
            if c.t < t0 || fade <= 0.0 {
                continue;
            }
            let end = t.min(t1);
            let stops = [t0, (t0 + end) / 2.0, end].map(|ts| self.kettles(p, p.sim_i(ts)));
            for pair in stops.windows(2) {
                for &(ki, x0, z0, kr) in &pair[0] {
                    let Some(&(_, x1, z1, _)) = pair[1].iter().find(|q| q.0 == ki) else {
                        continue;
                    };
                    let (dx, dz) = (x1 - x0, z1 - z0);
                    let n = dmath::hypot(dx, dz);
                    if n < 0.03 {
                        continue;
                    }
                    let (nx, nz) = (-dz / n * kr * 0.6, dx / n * kr * 0.6);
                    for s in [-1.0, 1.0] {
                        skid(
                            ic,
                            c,
                            [x0 + nx * s, z0 + nz * s],
                            [x1 + nx * s, z1 + nz * s],
                            0.08,
                            0.65 * fade,
                        );
                    }
                }
            }
        }
        // FULL SET: the shadows of the falling kettles close in on where they land
        if let Some((t0, t1)) = self.full_land()
            && (t0..t1).contains(&c.t)
        {
            let u = (c.t - t0) / (t1 - t0);
            for (ki, x, z, kr) in self.kettles(p, i).into_iter().filter(|q| q.0 >= 2) {
                floor::shade(ic, c, x, z, kr * (2.2 - 1.0 * u), 0.25 + 0.45 * u);
                floor::target(ic, c, x, z, kr, u, bold::light(pitch_col(ki)));
            }
        }
        // the rings: the low kettle's a wide, slow purple quake, the high one's a tight double yellow ring,
        // FULL SET's two in their own colours
        for r in self.rows(p, "KRINGS", i) {
            let [_, ki, x, z, rad, reach] = r[..] else { continue };
            let fade = 0.3 + 0.7 * (1.0 - rad / reach.max(1e-6)).max(0.0);
            let tint = bold::light(pitch_col(ki as i64));
            match ki as i64 {
                0 => {
                    floor::band(ic, c, Family::Quake, x, z, rad, fade, c.t, tint, 1.1);
                    floor::ring(ic, c, x, z, rad, 0.13, tint, 1.2 * fade);
                }
                1 => {
                    floor::ring(ic, c, x, z, rad, 0.09, tint, 0.9 * fade);
                }
                _ => {
                    floor::band(ic, c, Family::Quake, x, z, rad, fade, c.t, tint, 0.6);
                    floor::ring(ic, c, x, z, rad, 0.08, tint, 1.3 * fade);
                }
            }
        }
        // on a bar line of FULL SET all four heads ring out together: a short ring from each (a flourish of
        // the picture; the fight's rings are the ones above)
        if let Some(age) = self.tutti(p, c.t) {
            let u = age / TUTTI_S;
            for (ki, x, z, kr) in self.kettles(p, i) {
                let tint = bold::light(pitch_col(ki));
                floor::ring(ic, c, x, z, kr + 0.75 * bold::ease_out(u), 0.06, tint, 1.5 * (1.0 - u));
            }
        }
        // every landing: a puff of dust round the ball
        let dust = bold::light(DUST[0]);
        for h in self.hops.iter().rev().take_while(|h| h.t1 > c.t - 0.25) {
            let u = (c.t - h.t1) / 0.25;
            if (0.0..1.0).contains(&u) {
                let at = p.run.p(p.sim_i(h.t1.min(p.t_ko)), self.k);
                floor::band(
                    ic,
                    c,
                    Family::Air,
                    at[0],
                    at[1],
                    R * 0.9 + 0.7 * u,
                    1.0 - u,
                    c.t,
                    dust,
                    0.8,
                );
            }
        }
        if let Some((_, t1)) = self.full_land() {
            let u = (c.t - t1) / 0.3;
            if (0.0..1.0).contains(&u) {
                for (_, x, z, kr) in self.kettles(p, i).into_iter().filter(|q| q.0 >= 2) {
                    floor::band(ic, c, Family::Air, x, z, kr + 0.9 * u, 1.0 - u, c.t, dust, 1.0);
                }
            }
        }
    }

    /// The kettles, bolder, on the floor under the balls.
    pub(super) fn bold_canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        if !f.in_fight || f.t >= p.t_hs {
            return;
        }
        let i = p.sim_i(f.t.min(p.t_ko));
        // each ring again as a crisp band in its kettle's colour, over the light on the floor
        for r in self.rows(p, "KRINGS", i) {
            let [_, ki, x, z, rad, reach] = r[..] else { continue };
            let fade = (1.0 - rad / reach.max(1e-6)).max(0.0);
            let col = pitch_col(ki as i64);
            let widths: &[(f64, f64)] = match ki as i64 {
                0 => &[(0.0, 0.09)],
                1 => &[(0.0, 0.045), (0.17, 0.03)],
                _ => &[(0.0, 0.06)],
            };
            for &(back, w) in widths {
                let at = rad - back;
                if at > w {
                    let quads = band(f.cam, x, z, at - w, at + w, 0.015, 40);
                    ring::layer_blend(arr, &quads, col, 0.45 + 0.5 * fade, None);
                }
            }
        }
        let tutti = self.tutti(p, f.t).map_or(0.0, |age| 1.0 - age / TUTTI_S);
        for (ki, x, z, kr) in self.kettles(p, i) {
            let age = self.since_boom(ki, f.t);
            let flash = if age < 0.25 { 1.0 - age / 0.25 } else { 0.0 };
            let y = self.drop_y(ki, f.t);
            draw_bold_kettle(arr, f.cam, x, z, kr, y, flash.max(tutti), pitch_col(ki));
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight || f.t >= p.t_ko {
            return;
        }
        let k = self.k;
        let o = 1 - k;
        let (cam, t) = (f.cam, f.t);
        let i = p.sim_i(t);
        let me = drawn.iter().find(|d| d.k == k);
        let foe = drawn.iter().find(|d| d.k == o);
        // the hop: rings trail the ball through its arc
        if let (Some((h, u)), Some(d)) = (self.hop_at(t), me) {
            let (a, b) = (p.run.p(p.sim_i(h.t), k), p.run.p(p.sim_i(h.t1.min(p.t_ko)), k));
            let len = dmath::hypot(b[0] - a[0], b[1] - a[1]);
            for j in 1..=3 {
                let uu = u - 0.1 * f64::from(j);
                if uu <= 0.0 {
                    break;
                }
                let (lift, _, _) = hop_state(uu, len);
                let s = bold::at(
                    cam,
                    a[0] + (b[0] - a[0]) * uu,
                    a[1] + (b[1] - a[1]) * uu,
                    R + lift * HOP_K,
                );
                px::ring(
                    img,
                    s,
                    d.rp * (1.0 - 0.1 * f64::from(j)),
                    2.0,
                    bold::pale(COPPER_HI),
                    0.75 / f64::from(j),
                );
            }
        }
        // FULL SET: streaks over the falling kettles, and dust flies where they land
        if let Some((_, t1)) = self.full_land() {
            for (ki, x, z, kr) in self.kettles(p, i).into_iter().filter(|q| q.0 >= 2) {
                let y = self.drop_y(ki, t);
                if y > 0.0 {
                    for s in [-0.6, 0.0, 0.6] {
                        let len = if s == 0.0 { 1.5 } else { 1.0 };
                        px::ink_line(
                            img,
                            bold::at(cam, x + s * kr, z, y + 0.5),
                            bold::at(cam, x + s * kr, z, y + 0.5 + len),
                            2.0,
                            bold::pale(pitch_col(ki)),
                            1.0,
                        );
                    }
                }
                px::chips(
                    img,
                    bold::at(cam, x, z, 0.1),
                    t - t1,
                    0.4,
                    10,
                    34.0,
                    (t1 * 1000.0) as u64 + ki as u64,
                    DUST[0],
                );
            }
        }
        // PEDAL SLIDE: arrows run ahead of the gliding kettles at the enemy
        if self.slides.iter().any(|&(t0, t1)| (t0..t1).contains(&t)) {
            let e = f.fs[o].p;
            for (ki, x, z, kr) in self.kettles(p, i) {
                let (dx, dz) = (e[0] - x, e[1] - z);
                let n = dmath::hypot(dx, dz);
                let gap = n - kr - R;
                if gap < 0.25 {
                    continue;
                }
                let (ux, uz) = (dx / n, dz / n);
                let run = 0.12 * (t * 5.0).rem_euclid(1.0);
                let d0 = kr + 0.12 + run;
                let d1 = d0 + gap.min(0.7) * 0.75;
                px::ink_arrow(
                    img,
                    bold::at(cam, x + ux * d0, z + uz * d0, 0.1),
                    bold::at(cam, x + ux * d1, z + uz * d1, 0.1),
                    3.0,
                    9.0,
                    pitch_col(ki),
                    1.0,
                );
            }
        }
        // the squeeze: two arrowheads pinch the enemy along the line from the kettle that caught it
        if let (Some((age, e)), Some(d)) = (bold::since(p, k, &["hit:squeeze"], t, PINCH_S + 0.1), foe) {
            let u = age / (PINCH_S + 0.1);
            let kettles = self.kettles(p, p.sim_i(e.t));
            let from = kettles
                .iter()
                .find(|q| q.0 == e.i(8))
                .map_or((d.s.0 - 1.0, d.s.1), |&(_, x, z, _)| bold::at(cam, x, z, R));
            let (dx, dy) = (d.s.0 - from.0, d.s.1 - from.1);
            let n = dmath::hypot(dx, dy).max(1e-6);
            let (ux, uy) = (dx / n, dy / n);
            let col = pitch_col(e.i(8));
            let far = d.rp * (2.3 - 0.9 * bold::ease_out(u * 2.0));
            for s in [-1.0, 1.0] {
                px::ink_arrow(
                    img,
                    (d.s.0 - ux * far * s, d.s.1 - uy * far * s),
                    (d.s.0 - ux * d.rp * 0.95 * s, d.s.1 - uy * d.rp * 0.95 * s),
                    4.0,
                    11.0,
                    col,
                    (1.0 - u) * 2.0,
                );
            }
        }
        // the interval: a burst in both kettles' colours at once, half and half, with rays in turn
        for e in bold::events_in(p, k, &["hit:interval"], t - BURST_S, t) {
            let u = (t - e.t) / BURST_S;
            let second = e.i(8);
            let first = bold::events_in(p, k, &["hit:kettle"], e.t - 1.0, e.t)
                .filter(|q| q.i(8) != second)
                .last()
                .map_or(if second == 0 { 1 } else { 0 }, |q| q.i(8));
            let (ca, cb) = (pitch_col(first), pitch_col(second));
            let c = bold::at(cam, e.f(5), e.f(6), R);
            let r = 12.0 + 24.0 * bold::ease_out(u);
            let a = (1.0 - u) * 1.6;
            let q = dmath::PI / 2.0;
            px::ink_arc(img, c, r, 4.5, q, 3.0 * q, ca, a);
            px::ink_arc(img, c, r, 4.5, -q, q, cb, a);
            for j in 0..8 {
                let ang = dmath::TAU * (f64::from(j) + 0.5) / 8.0;
                let (ux, uy) = (dmath::cos(ang), dmath::sin(ang));
                px::ink_line(
                    img,
                    (c.0 + ux * r * 1.2, c.1 + uy * r * 1.2),
                    (c.0 + ux * r * 1.65, c.1 + uy * r * 1.65),
                    2.4,
                    if j % 2 == 0 { ca } else { cb },
                    a,
                );
            }
        }
        // CROSS-STICKING: two crossed sticks over the ball, tapping in turn on every 8th
        if let Some(d) = me
            && f.fs[k].haste > 0.0
            && f.fs[k].vis
        {
            let g = &p.show.grid;
            let eighth = (t - g.t_fight) / g.beat * 2.0;
            let tap = 1.0 - eighth.rem_euclid(1.0);
            let left = (eighth.floor() as i64).rem_euclid(2) == 0;
            let c0 = (d.s.0, d.s.1 - d.rp - 15.0);
            for (side, on) in [(-1.0, left), (1.0, !left)] {
                // the stick's butt out to its side, its head across the middle; the tapping one dips
                let dip = if on { 7.0 * tap * tap } else { 0.0 };
                let butt = (c0.0 + side * 17.0, c0.1 + 9.0);
                let head = (c0.0 - side * 15.0, c0.1 - 8.0 + dip);
                px::ink_line(img, butt, head, 3.0, WOOD, 1.0);
                px::ink_disc(img, head, 3.6, HEAD, 1.0);
            }
        }
    }

    /// The interval and the squeeze are solid hits whatever their damage.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match bold::how(e) {
            "interval" | "squeeze" => Some(by_damage.max(HitSize::Solid)),
            _ => Some(by_damage),
        }
    }
}
