//! MELLOPHONE with the bold effects (`picture.effects = "bold"`; the owner's picks 3a to 3d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **SLAPBACK** (3a): a glint at the bell, then a stack of three thick outlined arcs; a wave that lands on a rope
//!   bows it (the ring's own rope spark runs from there) and a thick ring tightens on the spot while a dashed line
//!   on the canvas shows where the echo will go; the echo comes back gold, wider and with more arcs.
//! - **HORNS UP** (3b): the bell tilts up under a glint; three outlined sound spears fly straight at the enemy,
//!   two taps and a bigger gold one that lands solid and leaves the enemy stunned.
//! - **PRESS BOX** (3c): bulbs chase round the canvas' edge and along the top ropes while it is on; an echo waiting
//!   on a rope shows as two rings on two spots, and its two waves leave from those spots.
//! - **STADIUM ECHO** (3d): one giant white-gold stack of arcs that grows with every rope it comes off; every rope
//!   it lands on bows and rings out in brass bands, and its last pass is a heavy hit with a flash round the enemy.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::roster::WeaponCfg;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, Status, StatusKind};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::ropes::RopeEv;
use bf_picture::scene::{self, A, R_BALL as R};

use super::{FLY_Y, HOT, MellophoneLook, WaveRow, arc, bayer, how, num};

/// A first wave: the mellophone's blue, lifted so it reads on the canvas.
const PERI: V3 = [188.0, 206.0, 255.0];
/// What the arcs behind an echo's first fade toward.
const DEEP_GOLD: V3 = [226.0, 140.0, 44.0];
/// Seconds the bell glints before a wave leaves it.
const BLOW_S: f64 = 0.18;
/// Seconds the bell is tilting up before HORNS UP.
const TILT_S: f64 = 0.28;
/// Seconds a split echo takes to come in from its own spot on the rope.
const SPLIT_S: f64 = 0.16;
/// How far either side of the landing the two spots of a split echo are.
const SPLIT_D: f64 = 0.34;
/// Seconds the flash of STADIUM ECHO's last pass shows.
const FLASH_S: f64 = 0.45;

/// One painter item of the bold look.
enum BItem {
    /// A wave in flight: its row, where it is drawn (`x`, `z`), and whether it is the stab that stuns.
    Wave(WaveRow, f64, f64, bool),
    /// An echo waiting on a rope: `x`, `z`, the share of its wait that is left, STADIUM ECHO's?
    Echo(f64, f64, f64, bool),
}

/// `a` mixed toward `b` by `k`.
fn mix(a: V3, b: V3, k: f64) -> V3 {
    std::array::from_fn(|c| a[c] + (b[c] - a[c]) * k.clamp(0.0, 1.0))
}

/// The unit direction along the rope a wave landed on at `(x, z)`.
fn along_rope(x: f64, z: f64) -> (f64, f64) {
    if x.abs() >= z.abs() { (0.0, 1.0) } else { (1.0, 0.0) }
}

/// A ring at flying height round `(x, z)`.
fn ring_pts(x: f64, z: f64, r: f64) -> Vec<V3> {
    (0..=14)
        .map(|j| {
            let a = f64::from(j) * dmath::TAU / 14.0;
            scene::to_world(x + r * dmath::cos(a), z + r * dmath::sin(a), FLY_Y)
        })
        .collect()
}

/// The band of light under a wave.
fn under(ic: &mut [V3], c: &CanvasCtx<'_>, w: &WaveRow, half_w: f64, tint: V3, k: f64) {
    let wide = WaveRow { half_w, ..*w };
    let pts = arc(&wide, 6);
    let reach = half_w + 0.45;
    for (j, light) in ic.iter_mut().enumerate() {
        let (x, z) = (c.sxp[j], c.szp[j]);
        if (x - w.x).abs() > reach || (z - w.z).abs() > reach {
            continue;
        }
        let d = pts
            .iter()
            .map(|q| dmath::hypot(x - q.0, z - q.1))
            .fold(f64::INFINITY, f64::min);
        let band = (1.0 - d / 0.26).clamp(0.0, 1.0);
        let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
        for ch in 0..3 {
            light[ch] += band * k * tint[ch];
        }
    }
}

/// How wide a wave is drawn (a little wider than it hits, and wider each generation), its lead arc's width in
/// pixels, how many arcs it stacks and its colour.
fn shape(w: &WaveRow) -> (f64, u32, usize, V3) {
    let g = w.generation;
    if w.kind == 2 {
        (
            w.half_w * (1.0 + 0.1 * g.min(5.0)),
            7 + g.min(4.0) as u32,
            4 + (g.min(4.0) / 2.0) as usize,
            mix(HOT, bold::WHITE, 0.25 + g / 5.0),
        )
    } else if g < 1.0 {
        (w.half_w.max(0.42), 5, 3, PERI)
    } else {
        (
            w.half_w.max(0.42) * (1.0 + 0.2 * g.min(3.0)),
            4 + g.min(3.0) as u32,
            3 + g.min(2.0) as usize,
            bold::GOLD,
        )
    }
}

impl MellophoneLook {
    /// The `slap` event that blew wave `id`, looking back from `t`.
    fn birth(p: &PictureCtx, k: usize, id: i64, t: f64) -> Option<&Event> {
        let i1 = p.ev_t.partition_point(|&te| te <= t);
        p.rec.events[..i1]
            .iter()
            .rev()
            .take_while(|e| e.t > t - 5.0)
            .find(|e| e.kind == "slap" && e.who() == Some(k) && e.i(3) == id)
    }

    /// Whether the stab born with `b` is the last of its HORNS UP, the one that stuns.
    fn last_stab(p: &PictureCtx, k: usize, b: &Event) -> bool {
        let Some(h) = p
            .rec
            .events
            .iter()
            .rev()
            .find(|e| e.kind == "horns_up" && e.who() == Some(k) && e.t <= b.t)
        else {
            return false;
        };
        let n = p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "slap" && e.who() == Some(k) && e.s(8) == "stab")
            .filter(|e| e.t >= h.t && e.t <= b.t && e.i(3) <= b.i(3))
            .count();
        n as i64 >= h.i(3)
    }

    /// For a wave that left a rope as one of PRESS BOX's two: which way along the rope its own spot is.
    fn split_side(p: &PictureCtx, k: usize, b: &Event) -> Option<(f64, f64)> {
        let i0 = p.ev_t.partition_point(|&te| te < b.t - 1e-6);
        let i1 = p.ev_t.partition_point(|&te| te <= b.t + 1e-6);
        let twin = p.rec.events[i0..i1.max(i0)].iter().find(|e| {
            e.kind == "slap"
                && e.who() == Some(k)
                && e.i(3) != b.i(3)
                && e.i(9) == b.i(9)
                && (e.f(4) - b.f(4)).abs() < 1e-9
                && (e.f(5) - b.f(5)).abs() < 1e-9
        })?;
        let (tx, tz) = along_rope(b.f(4), b.f(5));
        let side = (b.f(6) - twin.f(6)) * tx + (b.f(7) - twin.f(7)) * tz;
        let s = if side >= 0.0 { 1.0 } else { -1.0 };
        Some((tx * s, tz * s))
    }

    /// Whether PRESS BOX is on at `t`, and how strongly its lights show (they fade in and out).
    fn press_at(&self, t: f64) -> Option<f64> {
        self.press
            .iter()
            .find(|&&(a, b)| a <= t && t < b)
            .map(|&(a, b)| ((t - a) / 0.2).min((b - t) / 0.4).clamp(0.0, 1.0))
    }

    /// The seconds an echo waits on its rope: SLAPBACK's and STADIUM ECHO's.
    fn waits(&self, p: &PictureCtx) -> (f64, f64) {
        let cfg = &p.cfg[self.k];
        let slap = match &cfg.weapon {
            WeaponCfg::Slapback(w) => w.delay,
            _ => 0.5,
        };
        let stadium = [&cfg.unlock1, &cfg.unlock2, &cfg.low]
            .iter()
            .find(|ab| ab.params.contains_key("half_w"))
            .and_then(|ab| ab.params.get("delay"))
            .copied()
            .unwrap_or(0.25);
        let beat = p.show.grid.beat;
        ((slap * beat).max(1e-3), (stadium * beat).max(1e-3))
    }

    /// The last hit STADIUM ECHO landed, if it has landed one.
    fn last_pass(p: &PictureCtx, k: usize) -> Option<&Event> {
        p.rec
            .events
            .iter()
            .rev()
            .find(|e| e.kind == "hit" && e.who() == Some(k) && how(e) == "stadium" && e.t <= p.t_ko + 1e-6)
    }

    /// How far the bell is tilted up for HORNS UP at `t`, 0 to 1.
    fn tilt(&self, p: &PictureCtx, t: f64) -> f64 {
        let hold = 0.75 * p.show.grid.beat;
        self.horns
            .iter()
            .map(|&t0| {
                if t < t0 {
                    bold::ease_out(1.0 - (t0 - t) / TILT_S)
                } else if t < t0 + hold {
                    1.0
                } else {
                    1.0 - bold::ease_in((t - t0 - hold) / 0.2)
                }
            })
            .fold(0.0, f64::max)
    }

    pub(super) fn bold_ball(&self, p: &PictureCtx, kk: usize, t: f64, glass: V3, roll: f64) -> (V3, f64) {
        if kk != self.k {
            return (glass, roll);
        }
        // the bell tilts up
        (glass, roll - 0.45 * self.tilt(p, t))
    }

    /// Every wave that lands on a rope bows it there (and the ring's own spark runs from the spot): harder for
    /// one that will echo, hardest for STADIUM ECHO's.
    pub(super) fn bold_rope_events(&self, p: &PictureCtx) -> Vec<RopeEv> {
        let k = self.k;
        p.rec
            .events
            .iter()
            .filter(|e| e.kind == "slap_rope" && e.who() == Some(k) && e.t <= p.t_ko)
            .map(|e| {
                let (x, z) = (e.f(4), e.f(5));
                let side = match (x.abs() >= z.abs(), x >= 0.0, z >= 0.0) {
                    (true, true, _) => 1,
                    (true, false, _) => 3,
                    (false, _, true) => 0,
                    (false, _, false) => 2,
                };
                let stadium = Self::birth(p, k, e.i(3), e.t).is_some_and(|b| b.s(8) == "stadium");
                let spd = if stadium {
                    4.2
                } else if e.i(6) != 0 {
                    3.0
                } else {
                    1.6
                };
                RopeEv {
                    t: e.t,
                    side,
                    x,
                    z,
                    spd,
                }
            })
            .collect()
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let them = c.fs[1 - k].p;
        // a band of light under every wave
        for w in self.waves(p, c.t) {
            let (half_w, _, _, col) = shape(&w);
            let (half_w, strength) = match w.kind {
                1 => (0.16, 0.6),
                2 => (half_w, 1.1),
                _ => (half_w, 0.75),
            };
            under(ic, c, &w, half_w, bold::light(col), strength);
        }
        // an echo waiting on its rope: a dashed line on the canvas to where it will go
        for row in self.rows(p, "SLAPQ", c.t) {
            let Value::List(r) = &row else { continue };
            if num(r, 0) != Some(k as f64) {
                continue;
            }
            let (Some(x), Some(z)) = (num(r, 1), num(r, 2)) else {
                continue;
            };
            floor::dashes(ic, c, [x, z], them, 0.07, 0.3, c.t * 6.0, bold::light(bold::GOLD), 0.95);
        }
        // a wave landing on a rope rings out from the spot: STADIUM ECHO's in wide brass bands
        let i0 = p.ev_t.partition_point(|&te| te < c.t - 0.35);
        let i1 = p.ev_t.partition_point(|&te| te <= c.t);
        for e in p.rec.events[i0..i1.max(i0)]
            .iter()
            .filter(|e| e.kind == "slap_rope" && e.who() == Some(k))
        {
            let stadium = Self::birth(p, k, e.i(3), e.t).is_some_and(|b| b.s(8) == "stadium");
            let (life, r1, strength) = if stadium { (0.35, 1.6, 1.2) } else { (0.2, 0.55, 0.7) };
            let u = (c.t - e.t) / life;
            if u < 1.0 {
                let r = 0.15 + r1 * bold::ease_out(u);
                floor::band(
                    ic,
                    c,
                    Family::Brass,
                    e.f(4),
                    e.f(5),
                    r,
                    1.0 - u,
                    c.t,
                    bold::light(HOT),
                    strength,
                );
            }
        }
        // STADIUM ECHO's last pass: brass bands go out from the enemy
        if let Some(e) = Self::last_pass(p, k) {
            let u = (c.t - e.t) / FLASH_S;
            if (0.0..1.0).contains(&u) {
                let r = 0.4 + 2.2 * bold::ease_out(u);
                floor::band(
                    ic,
                    c,
                    Family::Brass,
                    e.f(5),
                    e.f(6),
                    r,
                    1.0 - u,
                    c.t,
                    bold::light(HOT),
                    1.3,
                );
            }
        }
        // PRESS BOX: bulbs chase round the canvas' edge
        if let Some(fade) = self.press_at(c.t) {
            let tint = bold::light(HOT);
            let step = (c.t * 9.0).floor();
            for (j, light) in ic.iter_mut().enumerate() {
                let (x, z) = (c.sxp[j], c.szp[j]);
                let edge = A - x.abs().max(z.abs());
                if !(0.0..0.4).contains(&edge) {
                    continue;
                }
                // how far round the ring this pixel is
                let s = match (x.abs() >= z.abs(), x >= 0.0, z >= 0.0) {
                    (true, true, _) => z + A,
                    (false, _, true) => 2.0 * A + (A - x),
                    (true, false, _) => 4.0 * A + (A - z),
                    (false, _, false) => 6.0 * A + (x + A),
                };
                let cell = (s / 0.6).floor();
                let d = dmath::hypot((s / 0.6 - cell - 0.5) * 0.6, edge - 0.2);
                if d > 0.17 {
                    continue;
                }
                let lit = (cell - step).rem_euclid(4.0) < 1.0;
                let v = if lit { 1.2 } else { 0.3 } * fade;
                let v = (v * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight {
            return Vec::new();
        }
        let k = self.k;
        let mut out: Vec<(f64, Payload)> = Vec::new();
        let depth_at = |x: f64, z: f64| f.cam.depth(&[scene::to_world(x, z, FLY_Y)])[0];
        for row in self.rows(p, "SLAP", f.t) {
            let Some(w) = WaveRow::of(&row).filter(|w| w.owner == k) else {
                continue;
            };
            let id = match &row {
                Value::List(r) => num(r, 1).unwrap_or(-1.0) as i64,
                _ => -1,
            };
            let b = Self::birth(p, k, id, f.t);
            let (mut x, mut z) = (w.x, w.z);
            let mut stun = false;
            if let Some(b) = b {
                if w.kind == 1 {
                    stun = Self::last_stab(p, k, b);
                } else if w.kind == 0
                    && f.t - b.t < SPLIT_S
                    && let Some((sx, sz)) = Self::split_side(p, k, b)
                {
                    // one of PRESS BOX's two: it comes in from its own spot on the rope
                    let off = SPLIT_D * (1.0 - (f.t - b.t) / SPLIT_S);
                    x += sx * off;
                    z += sz * off;
                }
            }
            out.push((depth_at(x, z), Box::new(BItem::Wave(w, x, z, stun))));
        }
        let (wait_slap, wait_stadium) = self.waits(p);
        let press = self.press_at(f.t).is_some();
        for row in self.rows(p, "SLAPQ", f.t) {
            let Value::List(r) = &row else { continue };
            if num(r, 0) != Some(k as f64) {
                continue;
            }
            let (Some(x), Some(z), Some(left)) = (num(r, 1), num(r, 2), num(r, 3)) else {
                continue;
            };
            let stadium = num(r, 5) == Some(2.0);
            let share = (left / if stadium { wait_stadium } else { wait_slap }).clamp(0.0, 1.0);
            if press && !stadium {
                // PRESS BOX: the echo will leave as two, from two spots
                let (tx, tz) = along_rope(x, z);
                for s in [-1.0, 1.0] {
                    let (qx, qz) = (x + tx * s * SPLIT_D, z + tz * s * SPLIT_D);
                    out.push((depth_at(qx, qz), Box::new(BItem::Echo(qx, qz, share, false))));
                }
            } else {
                out.push((depth_at(x, z), Box::new(BItem::Echo(x, z, share, stadium))));
            }
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let col = p.col[self.k];
        match payload.downcast_ref::<BItem>() {
            Some(&BItem::Wave(w, x, z, stun)) if w.kind == 1 => draw_spear(fr, f.cam, &w, x, z, stun),
            Some(&BItem::Wave(w, x, z, _)) => {
                // a stack of thick arcs, the widest in front
                let (half_w, width, n, lead) = shape(&w);
                let sp = dmath::hypot(w.vx, w.vz).max(1e-9);
                for j in (0..n).rev() {
                    let lag = 0.2 * j as f64;
                    let ripple = WaveRow {
                        x: x - w.vx / sp * lag,
                        z: z - w.vz / sp * lag,
                        half_w: half_w * (1.0 - 0.15 * j as f64),
                        ..w
                    };
                    let pts: Vec<V3> = arc(&ripple, 8)
                        .iter()
                        .map(|q| scene::to_world(q.0, q.1, FLY_Y))
                        .collect();
                    // the arcs behind fade toward the mellophone's blue, an echo's toward a deeper gold
                    let back = if w.kind == 0 && w.generation < 1.0 {
                        col
                    } else {
                        DEEP_GOLD
                    };
                    let c = mix(lead, back, 0.25 * j as f64);
                    bold::thick_line(fr, f.cam, &pts, c, width.saturating_sub(j as u32).max(3));
                }
            }
            Some(&BItem::Echo(x, z, share, stadium)) => {
                // a thick ring on the rope that tightens until the echo leaves
                let (r, c, width) = if stadium {
                    (0.2 + 0.7 * share, HOT, 4)
                } else {
                    (0.14 + 0.5 * share, bold::GOLD, 3)
                };
                bold::thick_line(fr, f.cam, &ring_pts(x, z, r), c, width);
                bold::blob(fr, f.cam, scene::to_world(x, z, FLY_Y), 0.07, bold::WHITE);
            }
            None => {}
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let k = self.k;
        if !f.in_fight || f.t > p.t_ko + FLASH_S {
            return;
        }
        let me = drawn.iter().find(|d| d.k == k).filter(|_| !f.fs[k].shattered);
        if let Some(d) = me {
            // S3: the bell glints before a wave leaves it
            if let Some((u, e)) = bold::until(p, k, &["slap"], f.t, BLOW_S)
                && e.i(9) == 0
                && e.s(8) != "stab"
            {
                let (dx, dy) = (
                    bold::at(f.cam, e.f(4), e.f(5), R).0 - d.s.0,
                    bold::at(f.cam, e.f(4), e.f(5), R).1 - d.s.1,
                );
                let n = dmath::hypot(dx, dy).max(1e-9);
                let q = (d.s.0 + dx / n * d.rp, d.s.1 + dy / n * d.rp);
                px::glint(img, q, 3.0 + 6.0 * u, bold::WHITE, 1.0);
            }
            // HORNS UP: a glint over the bell as it tilts up
            let tilt = self.tilt(p, f.t);
            if tilt > 0.0 && bold::until(p, k, &["horns_up"], f.t, TILT_S).is_some() {
                let q = (d.s.0 + d.rp * 0.35, d.s.1 - d.rp - 3.0 - 4.0 * tilt);
                px::glint(img, q, 4.0 + 8.0 * tilt, bold::GOLD, 1.0);
            }
        }
        // PRESS BOX: bulbs run along the top ropes
        if let Some(fade) = self.press_at(f.t) {
            const N: usize = 13;
            let step = (f.t * 9.0).floor() as usize;
            for side in 0..4 {
                let pts = f.cam.wp(&p.rope_points(side, scene::ROPE_Y[2], f.t, N));
                for (j, q) in pts.iter().enumerate().take(N - 1) {
                    if (j + side * (N - 1) + 4 * N - step % 4).is_multiple_of(4) {
                        px::ink_disc(img, *q, 2.8, HOT, fade);
                    } else {
                        px::disc(img, *q, 1.6, bold::deep(HOT), 0.8 * fade);
                    }
                }
            }
        }
        // STADIUM ECHO's last pass: a flash round the enemy
        if let Some(e) = Self::last_pass(p, k) {
            let u = (f.t - e.t) / FLASH_S;
            if (0.0..1.0).contains(&u) {
                let c = bold::at(f.cam, e.f(5), e.f(6), R);
                if u < 0.4 {
                    let r = 16.0 + 26.0 * bold::ease_out(u / 0.4);
                    px::ink_star(img, c, r, r * 0.42, 8, 0.3, bold::WHITE, 1.0 - u / 0.4 * 0.6);
                }
                for j in 0..2 {
                    let ur = (u - 0.12 * f64::from(j)).max(0.0);
                    px::ink_ring(img, c, 14.0 + 40.0 * bold::ease_out(ur), 2.5, HOT, 1.0 - ur);
                }
            }
        }
    }

    /// HORNS UP's last stab stuns.
    pub(super) fn bold_statuses(&self, p: &PictureCtx, t: f64) -> Vec<Status> {
        let i0 = p.ev_t.partition_point(|&te| te < t - 4.0);
        let i1 = p.ev_t.partition_point(|&te| te <= t);
        p.rec.events[i0..i1.max(i0)]
            .iter()
            .filter(|e| e.kind == "horns_stun" && e.i(3) == self.k as i64 && t < e.t + e.f(4))
            .filter_map(|e| {
                Some(Status {
                    who: e.who().filter(|&w| w < 2)?,
                    kind: StatusKind::Stun,
                    left: Some(1.0 - (t - e.t) / e.f(4).max(1e-6)),
                })
            })
            .collect()
    }

    /// An echo is a solid hit whatever its damage; two stabs tap and the one that stuns is solid; STADIUM ECHO
    /// is solid every pass and heavy on its last.
    pub(super) fn bold_hit(p: &PictureCtx, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        let k = e.who().filter(|&k| k < 2)?;
        match how(e) {
            "slapback" if e.i(8) > 0 => Some(by_damage.max(HitSize::Solid)),
            "stab" => {
                let stuns = p
                    .rec
                    .events
                    .iter()
                    .any(|s| s.kind == "horns_stun" && s.i(3) == k as i64 && (s.t - e.t).abs() < 1e-6);
                Some(if stuns { HitSize::Solid } else { HitSize::Tap })
            }
            "stadium" if Self::last_pass(p, k).is_some_and(|l| (l.t - e.t).abs() < 1e-9) => Some(HitSize::Heavy),
            "stadium" => Some(by_damage.max(HitSize::Solid)),
            _ => Some(by_damage),
        }
    }
}

/// A HORNS UP stab as a sound spear at `(x, z)`: a thick shaft and an outlined head, bigger and gold for the one
/// that stuns.
fn draw_spear(fr: &mut Frame, cam: &Cam, w: &WaveRow, x: f64, z: f64, stun: bool) {
    let sp = dmath::hypot(w.vx, w.vz).max(1e-9);
    let (ux, uz) = (w.vx / sp, w.vz / sp);
    let (nx, nz) = (-uz, ux);
    let (s, col, width) = if stun {
        (1.5, bold::GOLD, 5)
    } else {
        (1.0, bold::pale(PERI), 3)
    };
    let at = |along: f64, side: f64| scene::to_world(x + ux * along + nx * side, z + uz * along + nz * side, FLY_Y);
    bold::thick_line(fr, cam, &[at(-0.75 * s, 0.0), at(0.0, 0.0)], col, width);
    bold::outlined_poly(
        fr,
        cam,
        &[at(0.34 * s, 0.0), at(-0.06 * s, 0.19 * s), at(-0.06 * s, -0.19 * s)],
        col,
    );
}
