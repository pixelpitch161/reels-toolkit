//! SNARE DRUM with the bold effects (`picture.effects = "bold"`; the owner's picks 2a to 2d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **PARADIDDLE** (2a): two thick hickory sticks ride the ball, the idle one held ready; each jab stops on the
//!   enemy for three frames; the stick of the coming accent lifts with a glint before it strikes, and the accent
//!   itself is thicker, haloed in the snare's colour and lands with wood chips.
//! - **FLAM** (2b): a see-through grace stick leads every stroke and lands a hair early with its own pale star
//!   beside the real one.
//! - **RIMSHOT** (2c): while it is armed a chrome hoop with lugs sits round the ball and a glint runs round it; the
//!   stroke that spends it sparks off the rim, cracks out in a jagged ring and leaves the enemy stunned.
//! - **BUZZ ROLL** (2d): the sticks blur into a saw-edged disc of light on the canvas, exactly as wide as the
//!   grind reaches; every tick throws sawdust, more each time, and the last tick is a full accent.

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::roster::WeaponCfg;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, Status, StatusKind};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Pic};
use bf_picture::ring::PictureCtx;
use bf_picture::scene::R_BALL as R;

use super::{BEAD, CHROME, HICKORY, STICK_Y, SnareDrumLook, how, state};
use bf_core::roster::FighterCfg;

/// The grace stick and its star.
const GHOST: V3 = [190.0, 232.0, 255.0];
/// Seconds the grace stick is ahead of the real one.
const LEAD: f64 = 0.05;
/// Seconds a rimshot's crack ring runs.
const CRACK_S: f64 = 0.4;
/// Seconds the chips of an accent or a grind tick fly.
const CHIP_S: f64 = 0.28;
/// The teeth of the buzz disc.
const TEETH: f64 = 18.0;

/// How a stick is drawn.
#[derive(Clone, Copy)]
enum Kind {
    Plain,
    /// An accent, haloed in this colour.
    Accent(V3),
    /// FLAM's grace stick.
    Ghost,
}

/// A deterministic 0 to 1 from two whole numbers.
fn rnd(a: u64, b: u64) -> f64 {
    let mut h = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h % 10_000) as f64 / 10_000.0
}

/// One stick from `a` to its tip `b`, each `(x, z, height)` in the ring.
fn stick(arr: &mut Img<f32>, cam: &Cam, a: V3, b: V3, kind: Kind) {
    let (sa, sb) = (bold::at(cam, a[0], a[1], a[2]), bold::at(cam, b[0], b[1], b[2]));
    match kind {
        Kind::Plain => {
            px::ink_line(arr, sa, sb, 3.0, HICKORY, 1.0);
            px::line(arr, sa, sb, 1.0, bold::pale(HICKORY), 1.0);
            px::ink_disc(arr, sb, 2.6, BEAD, 1.0);
        }
        Kind::Accent(col) => {
            px::line(arr, sa, sb, 9.0, col, 0.8);
            px::ink_line(arr, sa, sb, 4.6, bold::pale(HICKORY), 1.0);
            px::line(arr, sa, sb, 1.6, bold::WHITE, 1.0);
            px::disc(arr, sb, 6.2, col, 0.8);
            px::ink_disc(arr, sb, 4.0, bold::WHITE, 1.0);
        }
        Kind::Ghost => {
            px::line(arr, sa, sb, 5.0, bold::INK, 0.22);
            px::line(arr, sa, sb, 3.0, GHOST, 0.55);
            px::disc(arr, sb, 3.0, GHOST, 0.7);
        }
    }
}

/// The saw-edged disc of BUZZ ROLL as light on the canvas: a faint fill, six blurred sticks and a bright toothed
/// edge whose points reach exactly `r` from `(cx, cz)`. `u` is how far the roll is (it gets louder).
#[allow(clippy::too_many_arguments)]
fn saw_disc(ic: &mut [V3], c: &CanvasCtx<'_>, cx: f64, cz: f64, r: f64, spin: f64, u: f64, col: V3) {
    let depth = 0.1 + 0.08 * u;
    let (edge, fill, wood) = (bold::light(bold::pale(col)), bold::light(col), bold::light(HICKORY));
    for (j, light) in ic.iter_mut().enumerate() {
        let (dx, dz) = (c.sxp[j] - cx, c.szp[j] - cz);
        let d = dmath::hypot(dx, dz);
        if d > r + 0.06 {
            continue;
        }
        let ang = dmath::atan2(dz, dx) - spin;
        // a saw tooth: the edge climbs to the full reach and drops back at once
        let tooth = (ang * TEETH / dmath::TAU).rem_euclid(1.0);
        let rim = r * (1.0 - depth * (1.0 - tooth));
        let mut v: f64 = 0.0;
        let mut tint = fill;
        if d <= rim {
            v = 0.22 + 0.18 * u;
            // the blurred sticks: six spokes, each with a smear behind it
            let spoke = (ang * 6.0 / dmath::TAU).rem_euclid(1.0);
            if d > 0.3 * r && spoke < 0.3 {
                v = 0.75 - 1.6 * spoke;
                tint = wood;
            }
            if rim - d < 0.07 {
                v = 1.0;
                tint = edge;
            }
        }
        if v > 0.0 {
            let s = (v.clamp(0.0, 1.0) * 3.0 + BAYER[c.ys[j] % 4][c.xs[j] % 4] * 0.9).floor() / 3.0;
            for ch in 0..3 {
                light[ch] += s * tint[ch] * (0.8 + 0.35 * u);
            }
        }
    }
}

/// `n` chips of wood flying from the screen point `c`, `age` seconds after they were struck.
fn chips(img: &mut Pic, c: (f64, f64), age: f64, n: u64, speed: f64, seed: u64) {
    let u = age / CHIP_S;
    if !(0.0..1.0).contains(&u) {
        return;
    }
    for i in 0..n {
        let ang = rnd(seed, i) * dmath::TAU;
        let far = speed * (0.5 + 0.7 * rnd(seed, i + 40)) * bold::ease_out(u);
        let q = (
            c.0 + dmath::cos(ang) * far,
            c.1 + dmath::sin(ang) * far * 0.7 + 10.0 * u * u,
        );
        let turn = ang * 3.0 + age * 30.0;
        let tip = (q.0 + dmath::cos(turn) * 3.6, q.1 + dmath::sin(turn) * 3.6);
        let col = if i % 3 == 0 { bold::pale(HICKORY) } else { HICKORY };
        px::ink_line(img, q, tip, 2.0, col, (1.0 - u).min(0.4) / 0.4);
    }
}

/// The roster numbers the bold moves read, taken in `setup_as` from its `cfg`: for its own side that is
/// `p.cfg[k]`, and EUPHONIUM's copy of this look gets the copied fighter's numbers.
#[derive(Clone, Debug, Default)]
pub(super) struct Nums {
    /// RIMSHOT's beats armed.
    rim_beats: f64,
    /// The sticks' spread, in degrees.
    spread: f64,
}

impl Nums {
    /// The numbers of the roster entry `cfg`.
    pub(super) fn of(cfg: &FighterCfg) -> Nums {
        Nums {
            rim_beats: [&cfg.unlock1, &cfg.unlock2, &cfg.low]
                .iter()
                .find(|ab| ab.params.contains_key("dmg_k"))
                .and_then(|ab| ab.params.get("dur_beats"))
                .copied()
                .unwrap_or(8.0),
            spread: match &cfg.weapon {
                WeaponCfg::Sticks(w) => w.spread,
                _ => 35.0,
            },
        }
    }
}

impl SnareDrumLook {
    /// The stretch RIMSHOT is armed in that holds `t`: from its arming to the stroke that spends it (or to its
    /// time running out).
    fn armed_at(&self, p: &PictureCtx, t: f64) -> Option<(f64, f64)> {
        let beats = self.nums.rim_beats;
        let t0 = *self.armed.iter().rev().find(|&&t0| t0 <= t)?;
        let spent = self.rims.iter().map(|r| r.0).find(|&tr| tr >= t0);
        let t1 = spent.unwrap_or(f64::INFINITY).min(t0 + beats * p.show.grid.beat);
        (t < t1).then_some((t0, t1))
    }

    /// The BUZZ ROLL that holds `t`, and how far through it `t` is.
    fn buzz_at(&self, t: f64) -> Option<f64> {
        self.buzz
            .iter()
            .find(|&&(a, b)| a <= t && t < b)
            .map(|&(a, b)| (t - a) / (b - a).max(1e-6))
    }

    /// Whether `e`, a tick of a BUZZ ROLL, is the last one of its roll to land.
    fn last_tick(p: &PictureCtx, e: &Event) -> bool {
        let who = e.who();
        let Some(until) = p
            .rec
            .events
            .iter()
            .rev()
            .find(|b| b.kind == "buzz_roll" && b.who() == who && b.t <= e.t)
            .map(|b| b.f(3))
        else {
            return false;
        };
        !p.rec
            .events
            .iter()
            .any(|x| x.kind == "hit" && x.who() == who && how(x) == "buzz" && x.t > e.t && x.t <= until + 1e-6)
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        // BUZZ ROLL: the saw disc, as wide as the grind reaches
        for r in self.live(p, c.t).iter().filter(|r| r.side == 2) {
            let u = self.buzz_at(c.t).unwrap_or(0.5);
            saw_disc(ic, c, r.x0, r.z0, r.ext, c.t * (14.0 + 10.0 * u), u, p.col[k]);
        }
        // a landed rimshot cracks out in a jagged ring
        for &(t0, _) in &self.rims {
            let u = (c.t - t0) / CRACK_S;
            if !(0.0..1.0).contains(&u) || t0 > p.t_ko {
                continue;
            }
            let at = p.run.p(p.sim_i(t0), 1 - k);
            floor::band(
                ic,
                c,
                Family::Noise,
                at[0],
                at[1],
                0.35 + 1.9 * bold::ease_out(u),
                1.0 - u,
                c.t,
                bold::light(bold::pale(CHROME)),
                1.3,
            );
        }
    }

    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let k = self.k;
        let me = &f.fs[k];
        if !f.in_fight || f.t > p.t_ko || me.shattered {
            return;
        }
        let col = p.col[k];
        // S1: a stick that has just landed stays where it touched
        let th = bold::hold_time(p, k, &["stick", "flam", "rimshot"], f.t);
        let rows = self.live(p, th);
        if rows.iter().any(|r| r.side == 2) {
            return;
        }
        let flam = state::within(&self.flams, f.t);
        let spread = self.nums.spread.to_radians();
        let them = f.fs[1 - k].p;
        let (dx, dz) = (them[0] - me.p[0], them[1] - me.p[1]);
        let n = dmath::hypot(dx, dz).max(1e-9);
        let (ux, uz) = (dx / n, dz / n);
        let rb = R * me.scale.max(0.3);
        for side in 0..2i64 {
            // the side of the jab: where the grace stick is drawn beside it
            let (ox, oz) = if side == 1 { (uz, -ux) } else { (-uz, ux) };
            if flam {
                // the grace stick: where the real one will be a moment from now
                for g in self
                    .live(p, th + LEAD)
                    .iter()
                    .filter(|g| g.side == side && g.ext > 0.02)
                {
                    stick(
                        arr,
                        f.cam,
                        [g.x0 + ox * 0.13, g.z0 + oz * 0.13, STICK_Y],
                        [g.x1 + ox * 0.13, g.z1 + oz * 0.13, STICK_Y],
                        Kind::Ghost,
                    );
                }
            }
            if let Some(r) = rows.iter().find(|r| r.side == side && r.ext > 0.02) {
                let kind = if self.accent_at(th, side) {
                    Kind::Accent(col)
                } else {
                    Kind::Plain
                };
                stick(arr, f.cam, [r.x0, r.z0, STICK_Y], [r.x1, r.z1, STICK_Y], kind);
                continue;
            }
            // the idle stick, held ready at its side of the ball
            let off = if side == 1 { -spread } else { spread };
            let (c, s) = (dmath::cos(off), dmath::sin(off));
            let butt = [me.p[0] + (ux * c - uz * s) * rb, me.p[1] + (ux * s + uz * c) * rb];
            // S3: the stick of the coming accent lifts first
            let lift = match bold::until(p, k, &["stick"], f.t, p.show.grid.beat * 0.5) {
                Some((u, e)) if e.i(4) == side && e.i(5) != 0 => bold::ease_out(u),
                _ => 0.0,
            };
            let len = R * (0.55 - 0.2 * lift);
            let tip = [butt[0] + ux * len, butt[1] + uz * len, STICK_Y + 0.22 + 0.85 * lift];
            let kind = if lift > 0.0 { Kind::Accent(col) } else { Kind::Plain };
            stick(arr, f.cam, [butt[0], butt[1], STICK_Y], tip, kind);
            if lift > 0.35 {
                px::glint(
                    arr,
                    bold::at(f.cam, tip[0], tip[1], tip[2]),
                    3.0 + 5.0 * lift,
                    bold::WHITE,
                    1.0,
                );
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let k = self.k;
        if !f.in_fight || f.t > p.t_ko + CHIP_S {
            return;
        }
        // RIMSHOT armed: the hoop lights up, its lugs showing, and a glint runs round it
        if let (Some((t0, _)), Some(d)) = (self.armed_at(p, f.t), drawn.iter().find(|d| d.k == k))
            && !f.fs[k].shattered
        {
            let flare = bold::until(p, k, &["hit:rimshot"], f.t, 0.12).map_or(0.0, |(u, _)| u);
            let beat = p.show.grid.beat_pulse(f.t, 0.15, 0.04).clamp(0.0, 1.0);
            let rr = d.rp + 3.5;
            let hoop: V3 = std::array::from_fn(|ch| CHROME[ch] + (bold::WHITE[ch] - CHROME[ch]) * beat.max(flare));
            px::ink_ring(img, d.s, rr, 2.2 + 2.0 * flare, hoop, 1.0);
            for j in 0..8 {
                let a = f64::from(j) * dmath::TAU / 8.0 + 0.4;
                px::ink_disc(
                    img,
                    (d.s.0 + dmath::cos(a) * rr, d.s.1 + dmath::sin(a) * rr),
                    2.0,
                    CHROME,
                    1.0,
                );
            }
            let a = (f.t - t0) * 7.0;
            px::glint(
                img,
                (d.s.0 + dmath::cos(a) * rr, d.s.1 + dmath::sin(a) * rr),
                6.0 + 2.0 * beat,
                bold::WHITE,
                1.0,
            );
        }
        let i0 = p.ev_t.partition_point(|&te| te < f.t - CHIP_S);
        let i1 = p.ev_t.partition_point(|&te| te <= f.t + LEAD);
        for (n, e) in p.rec.events[i0..i1.max(i0)].iter().enumerate() {
            if e.kind != "hit" || e.who() != Some(k) {
                continue;
            }
            let age = f.t - e.t;
            let c = bold::at(f.cam, e.f(5), e.f(6), R);
            let seed = (i0 + n) as u64;
            match how(e) {
                // the grace note's own star, a hair early and beside the real one
                "flam" => {
                    let u = (age + LEAD) / (LEAD + 0.16);
                    if (0.0..1.0).contains(&u) {
                        let r = 4.0 + 5.0 * bold::ease_out(u * 3.0);
                        let q = (c.0 - 9.0, c.1 - 7.0);
                        px::ink_star(img, q, r, r * 0.36, 4, 0.6, GHOST, (1.0 - u).min(0.5) * 2.0);
                    }
                }
                // an accent throws wood chips
                "stick" if e.i(8) != 0 && age >= 0.0 => chips(img, c, age, 6, 28.0, seed),
                "rimshot" if age >= 0.0 => {
                    chips(img, c, age, 9, 36.0, seed);
                    // the slap on the rim: a spark on the hoop, on the enemy's side
                    if let Some(d) = drawn.iter().find(|d| d.k == k) {
                        let u = age / 0.16;
                        if u < 1.0 {
                            let (dx, dy) = (c.0 - d.s.0, c.1 - d.s.1);
                            let n = dmath::hypot(dx, dy).max(1e-9);
                            let q = (d.s.0 + dx / n * (d.rp + 3.5), d.s.1 + dy / n * (d.rp + 3.5));
                            let r = 6.0 + 9.0 * bold::ease_out(u);
                            px::ink_star(img, q, r, r * 0.3, 6, e.t, bold::WHITE, 1.0 - u * u);
                        }
                    }
                }
                // sawdust off every tick of the grind, more each time; the last tick is a full accent
                "buzz" if age >= 0.0 => {
                    let u = self.buzz_at(e.t).unwrap_or(0.5);
                    if Self::last_tick(p, e) {
                        chips(img, c, age, 10, 28.0, seed);
                    } else {
                        chips(img, c, age, 2 + (5.0 * u) as u64, 11.0 + 9.0 * u, seed);
                    }
                }
                _ => {}
            }
        }
    }

    /// A landed rimshot stuns the enemy.
    pub(super) fn bold_statuses(&self, t: f64) -> Vec<Status> {
        self.rims
            .iter()
            .filter(|&&(t0, stun)| t0 <= t && t < t0 + stun)
            .map(|&(t0, stun)| Status {
                who: 1 - self.k,
                kind: StatusKind::Stun,
                left: Some(1.0 - (t - t0) / stun.max(1e-6)),
            })
            .collect()
    }

    /// An accent is a solid hit, a rimshot a heavy one; the grace note has its own star; a tick of the grind is a
    /// tap, but the last one is a full accent.
    pub(super) fn bold_hit(p: &PictureCtx, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match how(e) {
            "flam" => None,
            "rimshot" => Some(HitSize::Heavy),
            "stick" if e.i(8) != 0 => Some(by_damage.max(HitSize::Solid)),
            "buzz" if Self::last_tick(p, e) => Some(by_damage.max(HitSize::Solid)),
            "buzz" => Some(HitSize::Tap),
            _ => Some(by_damage),
        }
    }
}
