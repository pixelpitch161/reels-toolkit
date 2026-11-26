//! CONTRABASS with the bold effects (`picture.effects = "bold"`; the owner's picks 15a to 15d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **Walking bass** (15a): every step is a real hop in an arc; the ball squashes as it lands, the quake is a
//!   ring of lifted floor tiles under a thick inked ring, and each footfall leaves a footprint that fades within
//!   two beats.
//! - **SLAP BASS** (15b): a string across the ball is pulled right back from the enemy with a glint, snaps free
//!   and shivers; the ring is a thick wobbling string with slap lines, and the enemy flies off with speed lines.
//! - **DOUBLE TIME** (15c): quick lit step marks flicker left and right under the ball, and its footprints come
//!   lit in its colour, twice as many (the draining ring and the afterimages are the ring's own speed-up look).
//! - **THE ELEPHANT** (15d): big outlined ears, a banded trunk and two tusks grow out of the ball with a bounce;
//!   it charges with afterimages, a dust trail and speed lines; a trample is a heavy hit and cracks the floor
//!   there until the stampede is over.

use bf_core::dmath::{self, PI, TAU};
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Phase, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{self, R_BALL as R};

use super::state::{self, Land};
use super::{ContrabassLook, DUST, EAR, EAR_IN, Item, jag_ring};

/// The height of a step's hop.
const HOP: f64 = 0.36;
/// Seconds the ball is squashed after a footfall.
const SQUASH_S: f64 = 0.14;
/// Seconds a footfall's quake rolls out.
const QUAKE_S: f64 = 0.3;
/// Seconds the string is pulled back before the slap.
const PULL_S: f64 = 0.4;
/// Seconds the string shivers after it.
const SNAP_S: f64 = 0.22;
/// Seconds the slapped enemy trails speed lines.
const FLY_S: f64 = 0.4;
/// Seconds the ears, trunk and tusks take to grow.
const GROW_S: f64 = 0.3;
/// The slap's string and ring.
const STRING: V3 = [235.0, 245.0, 255.0];
/// The elephant's grey, light enough to read under the low-HP spotlight.
const GREY: V3 = [164.0, 160.0, 176.0];
/// The tusks.
const IVORY: V3 = [248.0, 240.0, 214.0];

/// A footprint on the lit canvas at `(x, z)` walking along `dir`: an oval pad and three toes, outlined.
fn footprint(arr: &mut Img<f32>, cam: &Cam, x: f64, z: f64, dir: [f64; 2], col: V3, a: f64) {
    let side = [-dir[1], dir[0]];
    let oval = |along: f64, across: f64, ra: f64, rb: f64| -> Vec<(f64, f64)> {
        (0..12)
            .map(|j| {
                let th = TAU * f64::from(j) / 12.0;
                let (u, v) = (along + ra * dmath::cos(th), across + rb * dmath::sin(th));
                bold::at(cam, x + dir[0] * u + side[0] * v, z + dir[1] * u + side[1] * v, 0.01)
            })
            .collect()
    };
    px::ink_poly(arr, &oval(-0.06, 0.0, 0.25, 0.21), col, a);
    for toe in [-1.0, 0.0, 1.0] {
        px::ink_poly(arr, &oval(0.33 - 0.05 * f64::abs(toe), 0.17 * toe, 0.09, 0.07), col, a);
    }
}

/// The distance from `(x, z)` to the segment `a` to `b`.
fn to_segment(x: f64, z: f64, a: [f64; 2], b: [f64; 2]) -> f64 {
    let (vx, vz) = (b[0] - a[0], b[1] - a[1]);
    let len2 = (vx * vx + vz * vz).max(1e-12);
    let u = (((x - a[0]) * vx + (z - a[1]) * vz) / len2).clamp(0.0, 1.0);
    dmath::hypot(x - a[0] - vx * u, z - a[1] - vz * u)
}

/// Cracks in the canvas round `(x, z)`: six jagged branches about `size` long, dark with a lit edge, at strength
/// `k`. `seed` picks their shape.
fn cracks(ic: &mut [V3], c: &CanvasCtx<'_>, x: f64, z: f64, size: f64, seed: u64, k: f64) {
    if k <= 0.0 {
        return;
    }
    let mut segs: Vec<([f64; 2], [f64; 2])> = Vec::new();
    for b in 0..6u64 {
        let mut a = TAU * (b as f64 + px::rnd(seed, b)) / 6.0;
        let mut q = [x, z];
        for s in 0..4u64 {
            a += (px::rnd(seed, 10 + b * 4 + s) - 0.5) * 1.5;
            let len = size / 4.0 * (0.6 + 0.8 * px::rnd(seed, 40 + b * 4 + s));
            let n = [q[0] + len * dmath::cos(a), q[1] + len * dmath::sin(a)];
            segs.push((q, n));
            q = n;
        }
    }
    for (j, light) in ic.iter_mut().enumerate() {
        let (qx, qz) = (c.sxp[j], c.szp[j]);
        if dmath::hypot(qx - x, qz - z) > size * 1.5 {
            continue;
        }
        let d = segs
            .iter()
            .map(|&(a, b)| to_segment(qx, qz, a, b))
            .fold(f64::INFINITY, f64::min);
        if d < 0.045 {
            for v in light.iter_mut() {
                *v *= 1.0 - 0.8 * k;
            }
        } else if d < 0.08 {
            for v in light.iter_mut() {
                *v += 0.3 * k;
            }
        }
    }
}

impl ContrabassLook {
    /// The way footfall `j` was walking (from its step's start to its landing).
    fn land_dir(&self, j: usize, l: &Land) -> [f64; 2] {
        let from = self
            .steps
            .iter()
            .rev()
            .find(|s| (s.t1 - l.t).abs() < 0.05)
            .map_or([l.x - 1.0, l.y], |s| s.p0);
        let (dx, dz) = (l.x - from[0], l.y - from[1]);
        let n = dmath::hypot(dx, dz);
        if n < 1e-6 {
            let a = j as f64;
            [dmath::cos(a), dmath::sin(a)]
        } else {
            [dx / n, dz / n]
        }
    }

    /// The quakes of the footfalls rolling out at `tc`: the footfall, the ring's radius and its strength (1 to 0).
    /// The fight's own ring is out in two frames; the picture lets it roll for [`QUAKE_S`] to the same reach.
    fn quakes(&self, tc: f64) -> Vec<(&Land, f64, f64)> {
        let i1 = state::searchsorted_right(&self.land_t, tc);
        let i0 = self.land_t.partition_point(|&v| v <= tc - QUAKE_S);
        self.land[i0..i1.max(i0)]
            .iter()
            .map(|l| {
                let u = (tc - l.t) / QUAKE_S;
                (l, R + l.reach * bold::ease_out(u), 1.0 - u)
            })
            .collect()
    }

    /// The footprints and DOUBLE TIME's step marks, crisp on the lit canvas under the balls.
    pub(super) fn bold_canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        if !(self.t_fight <= f.t && f.t < p.t_hs + 0.3) {
            return;
        }
        let k = self.k;
        let col = p.col[k];
        let tc = self.ta(p, f.t);
        let life = 2.0 * self.beat;
        let i1 = state::searchsorted_right(&self.land_t, tc);
        let i0 = self.land_t.partition_point(|&v| v < tc - life);
        for (j, l) in self.land.iter().enumerate().take(i1).skip(i0) {
            // it stays whole for the first beat and fades over the second
            let fade = ((1.0 - (tc - l.t) / life) * 2.0).min(1.0);
            let dir = self.land_dir(j, l);
            // left foot, right foot
            let off = if j % 2 == 0 { 0.16 } else { -0.16 };
            let (x, z) = (l.x - dir[1] * off, l.y + dir[0] * off);
            let ink = if l.kind != 0 { bold::WHITE } else { bold::deep(col) };
            footprint(arr, f.cam, x, z, dir, ink, 0.9 * fade);
        }
        // DOUBLE TIME: quick step marks flicker left and right of the ball
        let st = &f.fs[k];
        if f.in_fight && st.haste > 0.0 && st.vis {
            let o = f.fs[1 - k].p;
            let (dx, dz) = (o[0] - st.p[0], o[1] - st.p[1]);
            let n = dmath::hypot(dx, dz).max(1e-6);
            let dir = [dx / n, dz / n];
            let tick = (f.t * 15.0).floor() as i64;
            let side = if tick.rem_euclid(2) == 0 { 1.0 } else { -1.0 };
            let back = 0.3 * (tick.rem_euclid(3) as f64 - 1.0);
            footprint(
                arr,
                f.cam,
                st.p[0] - dir[1] * 0.85 * side + dir[0] * back,
                st.p[1] + dir[0] * 0.85 * side + dir[1] * back,
                dir,
                bold::WHITE,
                0.9,
            );
        }
    }

    /// How far the ears, trunk and tusks have grown `age` seconds into THE ELEPHANT: 0 to 1 with a bounce past it.
    fn grown(age: f64) -> f64 {
        let u = (age / GROW_S).clamp(0.0, 1.0);
        bold::ease_out(u) + 0.25 * dmath::sin(PI * u)
    }

    /// A bigger hop, and a squash on every footfall.
    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase) {
        if kk != self.k || phase != Phase::Fight {
            return;
        }
        if let Some((s, u)) = self.step_at(p, t) {
            st.lift = HOP * dmath::sin(PI * u) * if s.kind != 0 { 0.7 } else { 1.0 };
            // stretched a little as it leaves the floor
            let up = dmath::sin(PI * u);
            st.sx = Some(1.0 - 0.06 * up);
            st.sy = Some(1.0 + 0.08 * up);
        }
        let tc = self.ta(p, t);
        let i = state::searchsorted_right(&self.land_t, tc);
        for l in self
            .land
            .get(i.saturating_sub(1)..(i + 1).min(self.land.len()))
            .unwrap_or(&[])
        {
            let age = tc - l.t;
            if (0.0..0.12).contains(&age) {
                st.glow = st.glow.max(0.35 * (1.0 - age / 0.12));
            }
            if (0.0..SQUASH_S).contains(&age) {
                let k = 1.0 - age / SQUASH_S;
                st.sx = Some(1.0 + 0.24 * k);
                st.sy = Some(1.0 - 0.22 * k);
            }
        }
        if let Some((t0, _t1)) = self.stamp_at(p, t) {
            st.glow = st.glow.max(0.3);
            st.lift = st.lift.max(0.05 * dmath::sin(TAU * (tc - t0) / self.beat).abs());
        }
    }

    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.owner != self.k {
            return;
        }
        let tint = bold::light(p.col[self.k]);
        match w.kind.as_str() {
            // (a step's quake is drawn from its footfall, in `bold_canvas`: the fight's ring is gone in two frames)
            "stampede" => floor::wave(ic, c, Family::Quake, w, c.t, tint, 1.2),
            "slap" => {
                // a thick string: two wobbling bands a little apart
                let fade = 1.0 - w.r / w.rmax.max(1e-6);
                let white = bold::light(STRING);
                floor::band(ic, c, Family::Strings, w.cx, w.cz, w.r, fade, c.t, white, 1.3);
                floor::band(ic, c, Family::Strings, w.cx, w.cz, w.r - 0.12, fade, c.t, white, 0.8);
            }
            _ => {}
        }
    }

    /// The footfalls' quakes and THE ELEPHANT's cracks.
    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !(self.t_fight <= c.t && c.t < p.t_hs + 0.3) {
            return;
        }
        let k = self.k;
        let tc = self.ta(p, c.t);
        let life = 2.0 * self.beat;
        let tint = bold::light(p.col[k]);
        // each footfall's quake: lifted tiles rolling out to its reach
        for (l, r, fade) in self.quakes(tc) {
            floor::band(ic, c, Family::Quake, l.x, l.y, r, fade, c.t, tint, 1.0);
        }
        // THE ELEPHANT: the floor cracks where it tramples, until the stampede is over and two beats more
        for &(t0, x, z) in &self.trample {
            if t0 > tc {
                break;
            }
            let end = self
                .stamp
                .iter()
                .find(|&&(a, b)| a <= t0 && t0 <= b)
                .map_or(t0, |&(_, b)| b);
            let fade = (1.0 - (tc - end) / life).clamp(0.0, 1.0);
            let grow = bold::ease_out((tc - t0) / 0.12);
            cracks(ic, c, x, z, 1.1 * grow, (t0 * 1000.0) as u64, fade);
        }
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(&what) = payload.downcast_ref::<Item>() else {
            return;
        };
        let Some((t0, _)) = self.stamp_at(p, f.t) else {
            return;
        };
        let cam = f.cam;
        let (mut arr, depth) = (fr.array(), fr.depth());
        let st = &f.fs[self.k];
        let at = st.p;
        let h = R + st.lift;
        let w = scene::to_world(at[0], at[1], h);
        let c = cam.wp(&[w])[0];
        let rp = R * cam.f / cam.depth(&[w])[0];
        let t = f.t;
        let g = Self::grown(self.ta(p, t) - t0);
        match what {
            Item::Ears => {
                // big flapping ears either side, outlined, pink inside
                let flap = dmath::sin(TAU * t * 2.4);
                for side in [-1.0, 1.0] {
                    let a0 = side * (0.15 + 0.16 * flap);
                    let ear = |k: f64, squeeze: f64| -> Vec<(f64, f64)> {
                        (0..11)
                            .map(|j| {
                                let th = -1.4 + 2.8 * f64::from(j) / 10.0;
                                let rr = rp * g * 1.35 * (1.55 + 0.3 * dmath::cos(th * 1.3)) * k;
                                (
                                    c.0 + side * (rp * 0.35 + rr * 0.66 * dmath::cos(th + a0)),
                                    c.1 - rp * 0.15 + rr * 0.62 * squeeze * dmath::sin(th),
                                )
                            })
                            .collect()
                    };
                    px::ink_poly(&mut arr, &ear(1.0, 1.0), GREY, 1.0);
                    px::poly(&mut arr, &ear(0.74, 0.8), EAR_IN, 0.9);
                }
            }
            Item::Trunk => {
                // the trunk swinging ahead and curling up at the end, banded; a tusk either side of it
                let v = p
                    .run
                    .row("V", p.sim_i(t.min(p.t_ko)), self.k)
                    .unwrap_or_else(|| vec![1.0, 0.0]);
                let hd = dmath::atan2(v[1], v[0]);
                let swing = 0.5 * dmath::sin(TAU * t * 1.6);
                let curl = if swing >= 0.0 { 1.0 } else { -1.0 };
                for side in [-1.0, 1.0] {
                    let tusk: Vec<(f64, f64)> = (0..5)
                        .map(|j| {
                            let u = f64::from(j) / 4.0;
                            let a = hd + side * (0.55 - 0.25 * u);
                            let d = R * 0.85 + 0.5 * g * u;
                            bold::at(
                                cam,
                                at[0] + d * dmath::cos(a),
                                at[1] + d * dmath::sin(a),
                                h - 0.3 + 0.35 * u * u,
                            )
                        })
                        .collect();
                    px::ink_polyline(&mut arr, &tusk, 3.0, IVORY, 1.0);
                }
                let trunk: Vec<(f64, f64)> = (0..12)
                    .map(|j| {
                        let u = f64::from(j) / 11.0;
                        let a = hd + swing * u + 1.4 * dmath::pow(u, 3.0) * curl;
                        let d = R * 0.8 + g * (0.85 * u - 0.25 * dmath::pow(u, 4.0));
                        bold::at(
                            cam,
                            at[0] + d * dmath::cos(a),
                            at[1] + d * dmath::sin(a),
                            h - 0.25 * u + 0.7 * dmath::pow(u, 3.0),
                        )
                    })
                    .collect();
                px::ink_polyline(&mut arr, &trunk, 8.0, GREY, 1.0);
                for pair in trunk.windows(2).skip(1).step_by(2) {
                    let mid = ((pair[0].0 + pair[1].0) / 2.0, (pair[0].1 + pair[1].1) / 2.0);
                    px::disc(&mut arr, mid, 2.2, EAR, 0.8);
                }
            }
        }
        fr.set_array(&arr, Some(&depth));
    }

    /// THE ELEPHANT's afterimages and dust trail, under the ball.
    pub(super) fn bold_ball_under(&self, p: &PictureCtx, t: f64, arr: &mut Img<f32>, cam: &Cam) {
        let Some((t0, _)) = self.stamp_at(p, t) else {
            return;
        };
        let k = self.k;
        let tc = self.ta(p, t);
        let col = p.col[k];
        // the dust it kicks up: puffs that swell and thin where it ran
        for n in 1..=7u64 {
            let back = 0.07 * n as f64;
            if tc - back < t0 {
                break;
            }
            let at = p.run.p(p.sim_i(tc - back), k);
            let seed = ((tc - back) * 30.0).round() as u64;
            let (jx, jz) = (px::rnd(seed, 1) - 0.5, px::rnd(seed, 2) - 0.5);
            let s = bold::at(cam, at[0] + 0.5 * jx, at[1] + 0.5 * jz, 0.12 + 0.05 * n as f64);
            let ppu = bold::px_per_unit(cam, at[0], at[1], 0.1);
            let u = back / 0.5;
            px::disc(
                arr,
                s,
                ppu * (0.16 + 0.22 * u),
                DUST[(seed % 3) as usize],
                0.55 * (1.0 - u),
            );
        }
        // afterimages of the ball and its ears
        for n in (1..=3u64).rev() {
            let back = 0.06 * n as f64;
            if tc - back < t0 {
                continue;
            }
            let at = p.run.p(p.sim_i(tc - back), k);
            let s = bold::at(cam, at[0], at[1], R);
            let rp = R * bold::px_per_unit(cam, at[0], at[1], R);
            let a = 0.34 - 0.08 * n as f64;
            for side in [-1.0, 1.0] {
                px::disc(arr, (s.0 + side * rp * 1.35, s.1 - rp * 0.15), rp * 0.8, GREY, a);
            }
            px::disc(arr, s, rp, bold::pale(col), a);
        }
    }

    /// The quake rings, thick and inked; the slap's ring a thick string with slap lines.
    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        let (cam, k) = (f.cam, self.k);
        let col = p.col[k];
        for (l, r, fade) in self.quakes(self.ta(p, f.t)) {
            let seed = ((l.t * 1000.0) as i64).rem_euclid(9973) as u64;
            let ring = jag_ring(l.x, l.y, r, seed, 0.05, 56, 0.05);
            let sp: Vec<(f64, f64)> = ring.iter().map(|q| bold::at(cam, q[0], q[1], q[2])).collect();
            px::ink_polyline(arr, &sp, 2.2, bold::pale(col), dmath::pow(fade, 0.7));
        }
        for w in p.run.waves(f.i) {
            if w.owner != k {
                continue;
            }
            let fade = 1.0 - w.r / w.rmax.max(1e-6);
            let kk = dmath::pow(fade.max(0.0), 0.7);
            match w.kind.as_str() {
                "stampede" => {
                    let seed = ((w.t0 * 1000.0) as i64).rem_euclid(9973) as u64;
                    let ring = jag_ring(w.cx, w.cz, w.r, seed, 0.05, 56, 0.08);
                    let sp: Vec<(f64, f64)> = ring.iter().map(|q| bold::at(cam, q[0], q[1], q[2])).collect();
                    px::ink_polyline(arr, &sp, 3.0, bold::pale(col), kk);
                }
                "slap" => {
                    let wob = |a: f64| 0.09 * dmath::sin(a * 9.0 + f.t * 38.0) * (0.4 + 0.6 * fade);
                    let sp: Vec<(f64, f64)> = (0..=72)
                        .map(|j| {
                            let a = TAU * f64::from(j) / 72.0;
                            let r = w.r + wob(a);
                            bold::at(cam, w.cx + r * dmath::cos(a), w.cz + r * dmath::sin(a), 0.05)
                        })
                        .collect();
                    px::ink_polyline(arr, &sp, 3.6, STRING, kk);
                    // slap lines: short strokes flying out ahead of the string
                    for j in 0..14 {
                        let a = TAU * (f64::from(j) + 0.5) / 14.0;
                        let (ca, sa) = (dmath::cos(a), dmath::sin(a));
                        let (r0, r1) = (w.r + 0.16, w.r + 0.42);
                        px::ink_line(
                            arr,
                            bold::at(cam, w.cx + r0 * ca, w.cz + r0 * sa, 0.05),
                            bold::at(cam, w.cx + r1 * ca, w.cz + r1 * sa, 0.05),
                            1.6,
                            STRING,
                            kk,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let k = self.k;
        let tc = self.ta(p, f.t);
        let me = drawn.iter().find(|d| d.k == k);
        let foe = drawn.iter().find(|d| d.k == 1 - k);
        // SLAP BASS: the string across the ball, pulled back from the enemy, then snapping and shivering
        if let Some(d) = me {
            let aim = foe.map_or((1.0, 0.0), |e| {
                let (dx, dy) = (e.s.0 - d.s.0, e.s.1 - d.s.1);
                let n = dmath::hypot(dx, dy).max(1e-6);
                (dx / n, dy / n)
            });
            let ends = [
                (d.s.0 - aim.1 * d.rp * 1.2, d.s.1 + aim.0 * d.rp * 1.2),
                (d.s.0 + aim.1 * d.rp * 1.2, d.s.1 - aim.0 * d.rp * 1.2),
            ];
            let mid = |back: f64| (d.s.0 - aim.0 * d.rp * back, d.s.1 - aim.1 * d.rp * back);
            if let Some((u, _)) = bold::until(p, k, &["slap"], tc, PULL_S) {
                let m = mid(0.2 + 1.6 * bold::ease_out(u));
                px::ink_polyline(img, &[ends[0], m, ends[1]], 3.0, STRING, 1.0);
                px::ink_disc(img, m, 2.6, bold::GOLD, 1.0);
                px::glint(img, m, 4.0 + 7.0 * u, bold::WHITE, 0.5 + 0.5 * u);
            } else if let Some((age, _)) = bold::since(p, k, &["slap"], tc, SNAP_S) {
                let left = 1.0 - age / SNAP_S;
                let swing = 0.9 * left * dmath::cos(age * 95.0);
                px::polyline(img, &[ends[0], mid(swing * 0.6), ends[1]], 2.0, STRING, 0.5 * left);
                px::ink_polyline(img, &[ends[0], mid(-swing), ends[1]], 2.6, bold::WHITE, 1.0);
            }
        }
        // the slapped enemy flies off with speed lines
        if let (Some((age, _)), Some(d), Some(e)) = (bold::since(p, k, &["hit:slap"], tc, FLY_S), me, foe) {
            let dir = (e.s.0 - d.s.0, e.s.1 - d.s.1);
            px::speed_lines(img, e.s, e.rp, dir, 30.0, 7, STRING, 1.0 - age / FLY_S);
        }
        // THE ELEPHANT: speed lines behind the charge
        if let (Some((t0, _)), Some(d)) = (self.stamp_at(p, f.t), me)
            && tc - 0.07 > t0
        {
            let was = p.run.p(p.sim_i(tc - 0.07), k);
            let s0 = bold::at(f.cam, was[0], was[1], R);
            let dir = (d.s.0 - s0.0, d.s.1 - s0.1);
            if dmath::hypot(dir.0, dir.1) > 2.0 {
                px::speed_lines(img, d.s, d.rp * 1.5, dir, 24.0, 6, DUST[2], 0.8);
            }
        }
    }

    /// The slap and a trample are heavy hits whatever their damage.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match bold::how(e) {
            "slap" | "trample" => Some(HitSize::Heavy),
            _ => Some(by_damage),
        }
    }
}
