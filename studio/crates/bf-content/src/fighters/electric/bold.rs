//! ELECTRIC with the bold effects (`picture.effects = "bold"`; the owner's picks 24a to 24d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **LIGHTNING RIFF** (24a): the headstock or the corner post gathers sparks before every note; the bolt is a
//!   thick inked fork (one prong strikes, one hooks round the enemy) and the enemy flashes white and black like
//!   an x-ray; after a post strike sparks crawl away along that post's ropes.
//! - **DISTORTION** (24b): a crunchy square-stepped glow sits round the ball (flickering tiles on the canvas, a
//!   stepped ring round the glass, a jagged ring at the longer reach); the bolts run in square steps and fork
//!   three ways, past the enemy.
//! - **TAPPING** (24c): a row of pips over the ball, one for every 8th of the bar; each tap lights the next pip
//!   and fires a quick thin bolt.
//! - **GUITAR SMASH** (24d): the ball rises spinning over a landing mark, slams down and breaks into four shards
//!   with a jagged shockwave, bolts along the floor and big maple splinters; the shards fly back and snap
//!   together inside a closing ring.

use bf_core::dmath::{self, PI, TAU};
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::ss;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Pic};
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{CORNERS, R_BALL as R, ROPE_Y};

use super::{BODY, Bolt, ElectricLook, MAPLE, POST_CORNER, POST_SIDES, bolt_path, draw_bolt, jag_ring, on_screen};

/// Seconds a bold bolt shows (classic: 0.13).
const BOLT_S: f64 = 0.2;
/// Seconds the source gathers sparks before a note.
const CHARGE_S: f64 = 0.22;
/// Seconds the enemy is an x-ray after a bolt (a tap: half).
const XRAY_S: f64 = 0.13;
/// Seconds sparks crawl away along the ropes after a post strike.
const CRAWL_S: f64 = 0.5;
/// Seconds the ball rises before the smash.
const RISE_S: f64 = 0.34;
/// Seconds of bolts along the floor after the smash, and of the pop as it snaps back together.
const FLOOR_BOLT_S: f64 = 0.22;
const SNAP_S: f64 = 0.2;
/// The side of one tile of DISTORTION's glow on the canvas.
const TILE: f64 = 0.24;

/// The bolts' neon: the fighter's colour lifted, as `draw_bolt` lifts it.
fn neon(col: V3) -> V3 {
    col.map(|c| (c * 0.75 + 70.0).clamp(0.0, 255.0))
}

/// Screen points snapped to a grid of `grid` pixels and joined by level and upright runs only: a square-stepped
/// line.
fn stepped(pts: &[(f64, f64)], grid: f64) -> Vec<(f64, f64)> {
    let snap = |v: f64| (v / grid).round() * grid;
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(pts.len() * 2);
    for &(x, y) in pts {
        let (x, y) = (snap(x), snap(y));
        if let Some(&(lx, ly)) = out.last() {
            if lx == x && ly == y {
                continue;
            }
            if lx != x && ly != y {
                out.push((x, ly));
            }
        }
        out.push((x, y));
    }
    out
}

/// One line of a bolt on the picture: the dark outline, the neon body `w` wide and the white-hot core.
fn ink_bolt(img: &mut Pic, pts: &[(f64, f64)], w: f64, col: V3, alpha: f64) {
    px::ink_polyline(img, pts, w, col, alpha);
    px::polyline(img, pts, (w - 3.0).max(1.0), bold::WHITE, alpha);
}

/// The enemy as an x-ray: its ball white with dark bones, or (every other frame) dark with white bones.
fn xray(img: &mut Pic, d: &DrawnBall, flip: bool, col: V3) {
    let (bg, fg) = if flip {
        (bold::INK, bold::WHITE)
    } else {
        (bold::WHITE, bold::INK)
    };
    let ((x, y), r) = (d.s, d.rp);
    px::ink_disc(img, d.s, r, bg, 0.94);
    px::ring(img, d.s, r, 1.5, neon(col), 1.0);
    px::disc(img, (x, y - 0.5 * r), 0.2 * r, fg, 1.0);
    px::line(img, (x, y - 0.3 * r), (x, y + 0.62 * r), 1.6, fg, 1.0);
    for j in 0..3 {
        let yy = y - 0.1 * r + f64::from(j) * 0.2 * r;
        let half = (0.46 - 0.08 * f64::from(j)) * r;
        px::line(img, (x - half, yy), (x + half, yy), 1.6, fg, 1.0);
    }
}

impl ElectricLook {
    /// Its bolts near `t` on the attack clock, each with its age (negative: still to come): from `before`
    /// seconds ahead to `after` seconds old.
    fn near_bolts(&self, p: &PictureCtx, t: f64, before: f64, after: f64) -> Vec<(Bolt, f64)> {
        if !self.show(p, t) || self.bolt_t.is_empty() {
            return Vec::new();
        }
        let ta = self.ta(p, t);
        let (j0, j1) = (self.left(ta - after), self.right(ta + before));
        self.bolts
            .get(j0..j1)
            .unwrap_or(&[])
            .iter()
            .map(|b| (*b, ta - b.t0))
            .filter(|&(_, age)| age < after)
            .collect()
    }

    /// Whether DISTORTION, and whether TAPPING (with its start and end), is on at `t`.
    fn modes(&self, p: &PictureCtx, t: f64) -> (bool, Option<(f64, f64)>) {
        let ta = self.ta(p, t);
        let fork = self.fork.iter().any(|&(t0, t1)| t0 <= ta && ta < t1);
        let tap = self.tap.iter().copied().find(|&(t0, t1)| t0 <= ta && ta < t1 + 0.25);
        (fork, tap)
    }

    /// A bolt's lines in the ring frame (x, z, height), the trunk first: a miss or a tap is one line; a riff
    /// note that lands forks in two (one prong strikes, one hooks round the enemy); under DISTORTION it forks
    /// three ways and the outer prongs reach past the enemy. Re-seeded every other frame, so it flickers.
    fn bolt_lines(&self, fs: &[BallState; 2], b: &Bolt, frame: u64) -> Vec<Vec<V3>> {
        let (p0, p1) = self.bolt_ends(fs, b);
        let seed = b.i as u64 * 131 + (frame / 2) * 7;
        let levels = if b.fork { 2 } else { 3 };
        if b.tap || !b.hit {
            return vec![bolt_path(p0, p1, seed, 0.15, levels)];
        }
        let en = fs[1 - self.k].p;
        let d = [en[0] - p0[0], en[1] - p0[1]];
        let nd = dmath::hypot(d[0], d[1]).max(1e-9);
        let (u, n) = ([d[0] / nd, d[1] / nd], [-d[1] / nd, d[0] / nd]);
        let side = if px::rnd(seed, 1) < 0.5 { 1.0 } else { -1.0 };
        let split: V3 = std::array::from_fn(|c| p0[c] + (p1[c] - p0[c]) * 0.45);
        let hook = |sgn: f64, wide: f64, past: f64| -> V3 {
            [
                en[0] + n[0] * sgn * wide + u[0] * past,
                en[1] + n[1] * sgn * wide + u[1] * past,
                R * 0.8,
            ]
        };
        let mut lines = vec![bolt_path(p0, split, seed, 0.34, levels - 1)];
        lines.push(bolt_path(split, p1, seed + 1, 0.34, levels - 1));
        if b.fork {
            for sgn in [-1.0, 1.0] {
                let end = hook(sgn, R * 1.35, R * 1.5);
                lines.push(bolt_path(split, end, seed + 3 + (sgn > 0.0) as u64, 0.16, levels));
            }
        } else {
            lines.push(bolt_path(split, hook(side, R * 1.3, R * 0.9), seed + 2, 0.22, levels));
        }
        lines
    }

    /// The ball rises before the smash: up fast, a hang, then down hard.
    pub(super) fn bold_state(&self, p: &PictureCtx, st: &mut BallState, t: f64) {
        if let Some(u) = self.rising(p, t) {
            let h = if u < 0.8 {
                bold::ease_out(u / 0.55)
            } else {
                1.0 - bold::ease_in((u - 0.8) / 0.2)
            };
            st.lift += 1.5 * R * h;
        }
    }

    /// ...spinning.
    pub(super) fn bold_roll(&self, p: &PictureCtx, t: f64, roll: f64) -> f64 {
        match self.rising(p, t) {
            Some(u) => roll + 2.0 * TAU * bold::ease_in(u),
            None => roll,
        }
    }

    /// How far the rise before a smash is at `t` (0 to 1), or `None` outside one.
    fn rising(&self, p: &PictureCtx, t: f64) -> Option<f64> {
        let tc = self.ta(p, t);
        self.smash
            .iter()
            .find(|sm| sm.t0 - RISE_S <= tc && tc < sm.t0)
            .map(|sm| (tc - (sm.t0 - RISE_S)) / RISE_S)
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let tint = bold::light(p.col[k]);
        for (b, age) in self.near_bolts(p, c.t, CHARGE_S, BOLT_S) {
            let (p0, p1) = self.bolt_ends(c.fs, &b);
            if age >= 0.0 {
                // a lit lane under every bolt
                let gain = if b.hit { 0.8 } else { 0.4 } * (1.0 - age / BOLT_S);
                floor::lane(ic, c, [p0[0], p0[1]], [p1[0], p1[1]], 0.1, tint, gain);
            } else if !b.tap {
                // the charge: light gathers under the headstock or the post
                let u = 1.0 + age / CHARGE_S;
                floor::disc(ic, c, p0[0], p0[1], 0.2 + 0.35 * u, tint, 0.7 * u);
            }
        }
        let st = &c.fs[k];
        if self.show(p, c.t) && st.vis && c.t < p.t_ko && self.modes(p, c.t).0 {
            // DISTORTION: a square glow of flickering tiles under the ball, and a jagged ring at its reach
            let q = st.p;
            let tick = (c.t * 15.0) as u64;
            for (j, light) in ic.iter_mut().enumerate() {
                let tx = ((c.sxp[j] - q[0]) / TILE).round();
                let tz = ((c.szp[j] - q[1]) / TILE).round();
                let far = tx.abs().max(tz.abs());
                if far <= 4.0 {
                    let cell = ((tx + 8.0) * 31.0 + tz + 8.0) as u64;
                    let lit = if px::rnd(cell, tick) > 0.4 { 1.0 } else { 0.25 };
                    let v = (1.0 - far / 6.0) * lit * 1.5;
                    for ch in 0..3 {
                        light[ch] += v * tint[ch];
                    }
                }
            }
            let raw = &p.cfg[k].raw;
            let reach = raw["weapon"]["reach"].as_f64().unwrap_or(1.1)
                * raw["abilities"]["unlock1"]["reach_k"].as_f64().unwrap_or(1.4);
            floor::band(ic, c, Family::Noise, q[0], q[1], 2.0 * R + reach, 0.5, c.t, tint, 0.6);
        }
        // the smash to come: the ball's shadow shrinks on a landing mark that closes in
        if let Some(u) = self.rising(p, c.t) {
            let q = st.p;
            floor::target(ic, c, q[0], q[1], R * 1.6, u, tint);
        }
    }

    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind == "smash" {
            floor::wave(ic, c, Family::Noise, w, c.t, bold::light(p.col[w.owner]), 1.4);
        }
    }

    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let col = p.col[self.k];
        // posts about to strike: the ropes buzz first (as classic)
        for (b, age) in self.near_bolts(p, f.t, 0.11, 0.05) {
            if b.src >= 0 {
                self.rope_sparks(p, f, arr, &b, age);
            }
        }
        // the bolts' glow, under their inked lines
        for (b, age) in self.near_bolts(p, f.t, 0.0, BOLT_S) {
            if age < 0.0 {
                continue;
            }
            let k_ = dmath::pow(1.0 - age / BOLT_S, 0.7);
            for (n, line) in self.bolt_lines(f.fs, &b, f.f as u64).iter().enumerate() {
                let (halo, glow) = if b.tap || n >= 2 { (7, 3) } else { (11, 5) };
                draw_bolt(arr, f.cam, line, k_, col, halo, glow, 1);
            }
        }
    }

    /// The sparks that crawl away from a post along its two ropes after it struck.
    fn rope_crawl(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, b: &Bolt, age: f64) {
        let post = b.src as usize;
        let u = age / CRAWL_S;
        let col = neon(p.col[self.k]);
        let corner = CORNERS[POST_CORNER[post]];
        let (s0, s1) = POST_SIDES[post];
        for (m, s) in [s0, s1].into_iter().enumerate() {
            let (a, _) = p.side_ends(s);
            let at_start = (0..2).all(|i| (a[i] - corner[i]).abs() <= 1e-6);
            for (h, y) in [ROPE_Y[2], ROPE_Y[0]].into_iter().enumerate() {
                let pts = p.rope_points(s, y, t, 15);
                let last = pts.len() - 1;
                for j in 0..3u64 {
                    let seed = b.i as u64 * 7 + (m * 2 + h) as u64;
                    let far = bold::ease_out(u) * (0.3 + 0.45 * px::rnd(seed, j));
                    let i = far * last as f64;
                    let (i0, fr) = (i.floor() as usize, i - i.floor());
                    let pick = |n: usize| if at_start { pts[n] } else { pts[last - n] };
                    let (w0, w1) = (pick(i0.min(last)), pick((i0 + 1).min(last)));
                    let w: V3 = std::array::from_fn(|ch| w0[ch] + (w1[ch] - w0[ch]) * fr);
                    let sc = cam.wp(&[w])[0];
                    // a hopping spark: up off the rope every other frame
                    let hop = if ((t * 30.0) as u64 + j).is_multiple_of(2) {
                        2.0
                    } else {
                        0.0
                    };
                    let q = (sc.0, sc.1 - hop);
                    px::disc(img, q, 2.6, bold::INK, 1.0 - u);
                    px::glint(
                        img,
                        q,
                        2.0 + 3.0 * (1.0 - u),
                        if j == 1 { bold::WHITE } else { col },
                        1.0 - u,
                    );
                }
            }
        }
    }

    /// The four shards of the smashed ball round `c` (the screen point of its middle), `rp` its radius in
    /// pixels: thrown out by `out` (0 to 1) and tumbling, back together at 0.
    fn shards(img: &mut Pic, c: (f64, f64), rp: f64, out: f64, col: V3) {
        let glass = col.map(|v| v * 0.35 + 165.0);
        for (i, tumble) in [1.3, -0.9, 1.7, -1.4].into_iter().enumerate() {
            let a = PI / 4.0 + PI / 2.0 * i as f64;
            let far = rp * (1.9 + 0.3 * (i % 2) as f64) * out;
            let apex = (
                c.0 + dmath::cos(a) * far,
                c.1 + dmath::sin(a) * far * 0.62 + rp * 0.55 * out,
            );
            let rot = a + tumble * out;
            let rim: Vec<(f64, f64)> = (0..=8)
                .map(|n| {
                    let b = rot - PI / 4.0 + PI / 2.0 * f64::from(n) / 8.0;
                    (apex.0 + dmath::cos(b) * rp, apex.1 + dmath::sin(b) * rp)
                })
                .collect();
            let mut wedge = vec![apex];
            wedge.extend(rim.iter().copied());
            px::ink_poly(img, &wedge, glass, 1.0);
            // the glass's shine inside, a chip of the black body at the break, and the neon rim
            let inner = |b: f64, r: f64| (apex.0 + dmath::cos(b) * r, apex.1 + dmath::sin(b) * r);
            let shine: Vec<(f64, f64)> = (0..=4)
                .map(|n| inner(rot - 0.45 + 0.6 * f64::from(n) / 4.0, rp * 0.68))
                .collect();
            px::polyline(img, &shine, 2.0, bold::WHITE, 1.0);
            px::poly(
                img,
                &[inner(rot, 1.0), inner(rot - 0.6, rp * 0.34), inner(rot + 0.5, rp * 0.3)],
                BODY,
                0.92,
            );
            px::polyline(img, &rim, 2.0, neon(col), 1.0);
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let (k, cam, t, frame) = (self.k, f.cam, f.t, f.f as u64);
        let col = p.col[k];
        let me = drawn.iter().find(|d| d.k == k);
        let foe = drawn.iter().find(|d| d.k == 1 - k);
        let (fork, tap) = if self.show(p, t) && t < p.t_ko {
            self.modes(p, t)
        } else {
            (false, None)
        };
        // DISTORTION: a square-stepped ring round the glass, and crunchy pixels popping off it
        if let (true, Some(d)) = (fork, me) {
            let r = d.rp * if frame % 4 < 2 { 1.28 } else { 1.4 };
            let ring: Vec<(f64, f64)> = (0..=24)
                .map(|i| {
                    let a = f64::from(i) * TAU / 24.0;
                    (d.s.0 + r * dmath::cos(a), d.s.1 + r * dmath::sin(a))
                })
                .collect();
            px::ink_polyline(img, &stepped(&ring, 6.0), 2.0, neon(col), 1.0);
            for i in 0..4u64 {
                let a = px::rnd(frame / 2, i) * TAU;
                let far = r + 4.0 + 8.0 * px::rnd(frame / 2, i + 9);
                let q = (
                    ((d.s.0 + far * dmath::cos(a)) / 3.0).round() * 3.0,
                    ((d.s.1 + far * dmath::sin(a)) / 3.0).round() * 3.0,
                );
                let sq = [(q.0, q.1), (q.0 + 3.0, q.1), (q.0 + 3.0, q.1 + 3.0), (q.0, q.1 + 3.0)];
                px::ink_poly(img, &sq, if i % 2 == 0 { bold::WHITE } else { neon(col) }, 1.0);
            }
        }
        // the bolts: the charge before, the inked fork, the x-ray, the sparks crawling off along the ropes
        for (b, age) in self.near_bolts(p, t, CHARGE_S, CRAWL_S) {
            let (p0, _) = self.bolt_ends(f.fs, &b);
            let src = bold::at(cam, p0[0], p0[1], p0[2]);
            if age < 0.0 {
                if b.tap {
                    continue;
                }
                // sparks gather on the headstock or the post top
                let u = 1.0 + age / CHARGE_S;
                for i in 0..6u64 {
                    let a = (i as f64 + px::rnd(b.i as u64, i)) * TAU / 6.0 + u * 2.0;
                    let far = 4.0 + 15.0 * (1.0 - bold::ease_in(u));
                    let (cx, sy) = (dmath::cos(a), dmath::sin(a));
                    let q0 = (src.0 + cx * (far + 5.0), src.1 + sy * (far + 5.0));
                    let q1 = (src.0 + cx * far, src.1 + sy * far);
                    px::ink_line(img, q0, q1, 1.5, neon(col), 0.4 + 0.6 * u);
                }
                if b.src >= 0 {
                    px::ink_ring(img, src, 5.0 + 3.0 * u, 1.5, neon(col), u);
                }
                px::glint(img, src, 3.0 + 6.0 * u, bold::WHITE, u);
                continue;
            }
            if age < BOLT_S {
                let u = age / BOLT_S;
                let thin = if u < 0.5 { 1.0 } else { 1.5 - u };
                let alpha = (2.0 * (1.0 - u)).min(1.0);
                for (n, line) in self.bolt_lines(f.fs, &b, frame).iter().enumerate() {
                    let w = if b.tap {
                        2.5
                    } else if n >= 2 {
                        3.0
                    } else {
                        4.5
                    };
                    let sp = on_screen(cam, line);
                    let sp = if b.fork { stepped(&sp, 5.0) } else { sp };
                    ink_bolt(img, &sp, w * thin, col, alpha);
                }
                px::glint(img, src, 7.0 * (1.0 - u) + 2.0, bold::WHITE, 1.0 - u);
                if !b.hit && !b.tap {
                    // a miss fizzles out in chips
                    let end = bold::at(cam, b.x1, b.y1, R * 0.9);
                    px::chips(img, end, age, BOLT_S, 4, 10.0, b.i as u64, neon(col));
                }
            }
            if b.hit
                && age < if b.tap { XRAY_S / 2.0 } else { XRAY_S }
                && let Some(d) = foe
            {
                xray(img, d, frame % 2 == 1, col);
            }
            if b.src >= 0 {
                self.rope_crawl(p, img, cam, t, &b, age);
            }
        }
        // TAPPING: a pip for every 8th over the ball; each tap lights the next one
        if let (Some((t0, t1)), Some(d)) = (tap, me) {
            let ta = self.ta(p, t);
            let n = ((t1 - t0) / (p.show.grid.beat / 2.0)).round().clamp(1.0, 16.0) as usize;
            let taps: Vec<f64> = self
                .bolts
                .iter()
                .filter(|b| b.tap && b.t0 >= t0 - 1e-6 && b.t0 <= ta.min(t1 + 1e-6))
                .map(|b| b.t0)
                .collect();
            let shown = bold::ease_out((ta - t0) / 0.12) * (1.0 - ((ta - t1) / 0.25).clamp(0.0, 1.0));
            let y = d.s.1 - d.rp - 11.0;
            for i in 0..n {
                let x = d.s.0 + (i as f64 - (n as f64 - 1.0) / 2.0) * 8.0;
                let fresh = taps.get(i).map_or(0.0, |&tt| (1.0 - (ta - tt) / 0.12).max(0.0));
                let half = 2.5 + 2.5 * fresh;
                let (yy, fill) = match taps.get(i) {
                    Some(_) if fresh > 0.0 => (y + 4.0 * fresh, bold::WHITE),
                    Some(_) => (y, neon(col)),
                    None => (y, bold::deep(col)),
                };
                let sq = [
                    (x - half, yy - half),
                    (x + half, yy - half),
                    (x + half, yy + half),
                    (x - half, yy + half),
                ];
                px::ink_poly(img, &sq, fill, shown);
                if fresh > 0.0 {
                    // the tapping finger: a spark from the pip down to the glass
                    px::ink_line(img, (x, yy + half), (x, d.s.1 - d.rp * 0.8), 1.5, bold::WHITE, fresh);
                }
            }
        }
        self.bold_smash(p, img, f);
    }

    /// GUITAR SMASH on the finished picture: the jagged ring, the bolts along the floor, the splinters, the four
    /// shards and the snap.
    fn bold_smash(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>) {
        let (k, cam, t, frame) = (self.k, f.cam, f.t, f.f as u64);
        let col = p.col[k];
        if f.in_fight {
            for w in p.run.waves(f.i) {
                if w.kind == "smash" && w.owner == k {
                    let fade = (2.0 * (1.0 - w.r / w.rmax)).clamp(0.0, 1.0);
                    let ring = on_screen(cam, &jag_ring(w.cx, w.cz, w.r, t));
                    px::ink_polyline(img, &ring, 2.0, neon(col), fade);
                }
            }
        }
        let tc = p.fx_clock(t);
        for sm in &self.smash {
            let age = tc - sm.t0;
            let c = sm.c;
            let mid = bold::at(cam, c[0], c[1], R);
            let rp = bold::px_per_unit(cam, c[0], c[1], R) * R;
            if (0.0..FLOOR_BOLT_S).contains(&age) {
                // bolts along the floor from where it hit
                let reach = 1.0 + 1.8 * bold::ease_out(age / 0.1);
                for i in 0..6u64 {
                    let a = (i as f64 + px::rnd(31, i)) * TAU / 6.0;
                    let end = [c[0] + reach * dmath::cos(a), c[1] + reach * dmath::sin(a), 0.12];
                    let line = bolt_path([c[0], c[1], 0.12], end, i * 17 + (frame / 2) * 5, 0.32, 2);
                    ink_bolt(
                        img,
                        &on_screen(cam, &line),
                        3.5,
                        col,
                        (2.0 * (1.0 - age / FLOOR_BOLT_S)).min(1.0),
                    );
                }
            }
            // the splinters: big, inked, maple
            for (j, s) in sm.sp.iter().enumerate() {
                if !(0.0 <= age && tc < s.t1 + 0.05) {
                    continue;
                }
                let tau = tc.min(s.t1) - sm.t0;
                let dv = [dmath::cos(s.ang), dmath::sin(s.ang)];
                let q = [c[0] + dv[0] * s.speed * tau, c[1] + dv[1] * s.speed * tau];
                let turn = s.ang + tau * 14.0 * if j % 2 == 0 { 1.0 } else { -1.0 };
                let half = [dmath::cos(turn) * 0.24, dmath::sin(turn) * 0.24];
                let a = bold::at(cam, q[0] - half[0], q[1] - half[1], R * 0.7);
                let b = bold::at(cam, q[0] + half[0], q[1] + half[1], R * 0.7);
                px::ink_line(img, a, b, 3.0, MAPLE, 1.0);
                px::line(img, a, b, 1.0, bold::pale(MAPLE), 1.0);
            }
            if (0.0..sm.dur).contains(&age) && (f.in_fight || t < p.t_hs + 0.6) {
                // in four shards: thrown out, lying, then pulled back together inside a closing ring
                let u = age / sm.dur;
                let back = ss(0.6, 0.97, u);
                let out = bold::ease_out(u / 0.22) * (1.0 - back);
                Self::shards(img, mid, rp, out, col);
                if back > 0.0 {
                    px::ink_ring(img, mid, rp * (1.15 + 2.2 * (1.0 - back)), 2.0, neon(col), back);
                }
            }
            let snap = (age - sm.dur) / SNAP_S;
            if (0.0..1.0).contains(&snap) && t < p.t_ko {
                // snapped back: a white pop off the glass
                let q = f.fs[k].p;
                let s = bold::at(cam, q[0], q[1], R);
                px::ring(
                    img,
                    s,
                    rp * (1.1 + 0.9 * bold::ease_out(snap)),
                    2.5,
                    bold::WHITE,
                    1.0 - snap,
                );
                for i in 0..4 {
                    let a = PI / 4.0 + f64::from(i) * PI / 2.0;
                    let far = rp * (1.3 + 1.0 * bold::ease_out(snap));
                    let g = (s.0 + dmath::cos(a) * far, s.1 + dmath::sin(a) * far);
                    px::glint(img, g, 5.0 * (1.0 - snap) + 1.0, bold::WHITE, 1.0 - snap);
                }
            }
        }
    }
}
