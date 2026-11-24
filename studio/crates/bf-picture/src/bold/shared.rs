//! The bold effects every fighter gets from the ring itself, with nothing to do in its own look:
//!
//! - S4 "bigger hits": a hit pops in one of three sizes ([`HitSize`]): a tap is a small star, a solid hit adds a
//!   ring and chips, a heavy hit adds a short camera shake and squashes the enemy.
//! - S6 "speed-up look": while a fighter is hasted, afterimages trail its ball, a little note circles it and a
//!   ring round it drains with the time left.
//! - S7 "low-HP spotlight": as a low-HP move starts the ring dims round its fighter and the move's name shows
//!   big.
//! - S9 "shields show the block": a blocked hit flares the shield white where it was struck, and the shield's
//!   ring thins as its time runs down.
//! - S10 "ropes react more": a spark runs along the rope from where a ball hit it (the deeper bow is in
//!   `ring/ropes.rs`).
//! - S11 "level-up pop": a ring and a small up-arrow on the ball when its weapon grows.
//! - S12 "status you can see": the marks of [`StatusKind`] on a ball, from the looks' [`Look::bold_status`].
//! - N4, the side banners: each fighter's special move is named on a ribbon from its own side of the screen.
//!
//! [`Shared`] is everything of this worked out once from the recording; the `bold_*` methods of [`PictureCtx`]
//! draw it, each called from one place in `ring/frame.rs` and only when the fight file asked for the bold
//! effects.

use bf_core::dmath;

use super::px::{self, Surface, bad};
use super::{GOLD, INK, WHITE, deep, ease_out, pale};
use crate::camera::{Cam, V3};
use crate::config::{FPS, W};
use crate::font;
use crate::img::{Img, Rgb8};
use crate::look::{DrawnBall, Look, Pic};
use crate::ring::state::BallState;
use crate::ring::{PictureCtx, bayer, owner};
use crate::scene::{self, R_BALL as R};
use crate::ui::{DrawLog, Kind};

/// How big a hit pops (S4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HitSize {
    /// A light touch: a small star.
    Tap,
    /// A proper hit: a star, a ring and a few chips.
    Solid,
    /// A big one: a bigger star, two rings, more chips, a short shake and the enemy squashed.
    Heavy,
}

impl HitSize {
    /// The size of a hit by its damage: under 1.6 a tap, from 6 a heavy hit.
    pub fn of_damage(dmg: f64) -> HitSize {
        if dmg >= 6.0 {
            HitSize::Heavy
        } else if dmg >= 1.6 {
            HitSize::Solid
        } else {
            HitSize::Tap
        }
    }
}

/// What can be wrong with a fighter that the picture should say (S12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    /// Stunned: stars circle its head.
    Stun,
    /// Pinned or held in place: a pin ring round it.
    Pin,
    /// Slowed: a blue tint and down arrows.
    Slow,
    /// Silenced (it cannot attack, or cannot use its moves): grey with X marks.
    Silence,
    /// Out of tune (soured, weakened): a sour green with a tuner gauge.
    Sour,
}

/// One status on one ball right now, as a look reports it ([`Look::bold_status`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Status {
    /// The ball it is on, 0 or 1.
    pub who: usize,
    /// What it is.
    pub kind: StatusKind,
    /// The share of its time that is left, 1 to 0, for the ring that drains round the ball; `None` when it has
    /// no known end.
    pub left: Option<f64>,
}

/// A hit that pops.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pop {
    pub t: f64,
    pub att: usize,
    pub vic: usize,
    /// Where it landed, ring frame.
    pub x: f64,
    pub z: f64,
    pub size: HitSize,
    /// The event's index, for the chips' directions.
    pub seed: u64,
}

/// A special move's name on its fighter's side banner.
#[derive(Clone, Debug, PartialEq)]
pub struct Banner {
    pub t0: f64,
    pub t1: f64,
    pub who: usize,
    pub name: String,
    /// It is the fighter's low-HP move: the banner is bigger and the ring dims round the fighter (S7).
    pub low: bool,
}

/// The shared bold effects of one picture, worked out once.
#[derive(Clone, Debug, Default)]
pub struct Shared {
    /// The hits that pop, in time order.
    pub pops: Vec<Pop>,
    /// The banners, in time order.
    pub banners: Vec<Banner>,
    /// `(time, fighter)` of every level-up.
    pub levels: Vec<(f64, usize)>,
    /// Each fighter's stretches of haste, `(from, to)`.
    pub haste: [Vec<(f64, f64)>; 2],
    /// Each fighter's stretches under a shield, `(from, to)`.
    pub shield: [Vec<(f64, f64)>; 2],
    /// `(time, blocker, x, z)` of every blocked hit.
    pub blocks: Vec<(f64, usize, f64, f64)>,
}

/// The seconds a pop of each size shows.
fn pop_life(size: HitSize) -> f64 {
    match size {
        HitSize::Tap => 0.13,
        HitSize::Solid => 0.26,
        HitSize::Heavy => 0.34,
    }
}

/// The seconds a heavy hit squashes its victim.
const SQUASH_S: f64 = 0.16;
/// The seconds the ring stays dim round a low-HP move's fighter.
const SPOT_S: f64 = 0.9;
/// The seconds a level-up pop shows.
const LEVEL_S: f64 = 0.6;
/// The seconds a block's flare shows.
const BLOCK_S: f64 = 0.22;
/// The seconds a spark runs along a rope.
const ROPE_SPARK_S: f64 = 0.32;

/// The stretches where `key`'s value for fighter `k` is above 0, in seconds.
fn stretches(p: &PictureCtx, key: &str, k: usize) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    if !p.run.has(key) {
        return out;
    }
    let t_of = |i: usize| p.show.grid.t_fight + i as f64 / f64::from(FPS);
    let mut from: Option<usize> = None;
    for i in 0..p.run.nsim {
        let on = p.run.f(key, i, k).is_some_and(|v| v > 0.0);
        match (on, from) {
            (true, None) => from = Some(i),
            (false, Some(i0)) => {
                out.push((t_of(i0), t_of(i)));
                from = None;
            }
            _ => {}
        }
    }
    if let Some(i0) = from {
        out.push((t_of(i0), t_of(p.run.nsim)));
    }
    out
}

impl Shared {
    /// Everything from the recording: call once the looks are set up (`p.looks` is not yet filled then, so the
    /// looks are given).
    pub fn new(p: &PictureCtx, looks: &[Box<dyn Look>]) -> Shared {
        let mut s = Shared::default();
        // the hits that pop: not the quiet ones, and never two of one fighter's within 0.08 s unless bigger
        let mut last: [Option<(f64, HitSize)>; 2] = [None, None];
        for (i, e) in p.rec.events.iter().enumerate() {
            if e.t > p.t_ko + 1e-6 {
                break;
            }
            match e.kind.as_str() {
                "hit" => {
                    let Some(att) = owner(e) else { continue };
                    let how = e.s(7);
                    if looks[att].quiet_hits().contains(&how) {
                        continue;
                    }
                    let Some(size) = looks[att].bold_hit_size(p, e, HitSize::of_damage(e.f(4))) else {
                        continue;
                    };
                    if let Some((t0, s0)) = last[att]
                        && e.t - t0 < 0.08
                        && size <= s0
                    {
                        continue;
                    }
                    last[att] = Some((e.t, size));
                    let vic = usize::try_from(e.i(3)).unwrap_or(1 - att).min(1);
                    s.pops.push(Pop {
                        t: e.t,
                        att,
                        vic,
                        x: e.f(5),
                        z: e.f(6),
                        size,
                        seed: i as u64,
                    });
                }
                "level" => {
                    if let Some(k) = owner(e) {
                        s.levels.push((e.t, k));
                    }
                }
                "block" => {
                    if let Some(k) = owner(e) {
                        s.blocks.push((e.t, k, e.f(4), e.f(5)));
                    }
                }
                _ => {}
            }
        }
        for (who, caps) in p.show.caps_ability.iter().enumerate() {
            for c in caps {
                let name = c.lines.first().cloned().unwrap_or_default();
                let low = name == p.abil.names[who][2];
                s.banners.push(Banner {
                    t0: c.t0,
                    t1: c.t1,
                    who,
                    name,
                    low,
                });
            }
        }
        s.banners.sort_by(|a, b| a.t0.total_cmp(&b.t0));
        for k in 0..2 {
            s.haste[k] = stretches(p, "HA", k);
            s.shield[k] = stretches(p, "SH", k);
        }
        s
    }

    /// The pops alive at `t`.
    fn pops_at(&self, t: f64) -> &[Pop] {
        let i0 = self.pops.partition_point(|h| h.t < t - 0.4);
        let i1 = self.pops.partition_point(|h| h.t <= t);
        &self.pops[i0..i1.max(i0)]
    }

    /// The stretch of `list` that holds `t`, as the share of it that is left (1 to 0).
    pub fn left_in(list: &[(f64, f64)], t: f64) -> Option<f64> {
        list.iter().find(|&&(a, b)| a <= t && t < b).map(|&(a, b)| {
            if b > a {
                ((b - t) / (b - a)).clamp(0.0, 1.0)
            } else {
                0.0
            }
        })
    }

    /// The camera jolts of the heavy hits, `(time, pixels)`.
    pub fn jolts(&self) -> Vec<(f64, f64)> {
        self.pops
            .iter()
            .filter(|h| h.size == HitSize::Heavy)
            .map(|h| (h.t, 1.3))
            .collect()
    }

    /// S4: the squash of ball `k` at `t` as `(sx, sy)`, while a heavy hit on it is fresh.
    pub fn squash(&self, k: usize, t: f64) -> Option<(f64, f64)> {
        let i1 = self.pops.partition_point(|h| h.t <= t);
        self.pops[..i1]
            .iter()
            .rev()
            .take_while(|h| t - h.t < SQUASH_S)
            .find(|h| h.vic == k && h.size == HitSize::Heavy)
            .map(|h| {
                let u = (t - h.t) / SQUASH_S;
                // down and wide at once, then a small spring back
                let a = 0.2 * (1.0 - u) * dmath::cos(u * dmath::PI * 1.5);
                (1.0 + a, 1.0 - a)
            })
    }
}

/// A deterministic 0 to 1 from a seed and a counter.
fn rnd(seed: u64, n: u64) -> f64 {
    let mut h = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ n.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h % 100_000) as f64 / 100_000.0
}

/// A small eighth note at `c`: the "little symbol" that circles a hasted ball.
fn note<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), col: V3, a: f64) {
    px::line(s, (c.0 + 1.5, c.1 + 2.0), (c.0 + 1.5, c.1 - 5.0), 3.0, INK, a);
    px::disc(s, (c.0, c.1 + 2.0), 3.2, INK, a);
    px::line(s, (c.0 + 1.5, c.1 - 5.0), (c.0 + 4.5, c.1 - 3.0), 3.0, INK, a);
    px::line(s, (c.0 + 1.5, c.1 + 2.0), (c.0 + 1.5, c.1 - 5.0), 1.0, col, a);
    px::disc(s, (c.0, c.1 + 2.0), 2.2, col, a);
    px::line(s, (c.0 + 1.5, c.1 - 5.0), (c.0 + 4.5, c.1 - 3.0), 1.0, col, a);
}

/// The angle on the screen from `a` to `b`.
fn angle(a: (f64, f64), b: (f64, f64)) -> f64 {
    dmath::atan2(b.1 - a.1, b.0 - a.0)
}

impl PictureCtx {
    /// The share of fighter `k`'s shield that is left at `t` (1 as it goes up, 0 as it drops); `None` without a
    /// shield, and always `None` without the bold effects.
    pub fn bold_shield_left(&self, k: usize, t: f64) -> Option<f64> {
        Shared::left_in(&self.bold_fx.shield[k], t)
    }

    /// The share of fighter `k`'s haste that is left at `t`; `None` without one.
    pub fn bold_haste_left(&self, k: usize, t: f64) -> Option<f64> {
        Shared::left_in(&self.bold_fx.haste[k], t)
    }

    /// The time fighter `k`'s shield last blocked a hit at or before `t`, and where (ring frame).
    pub fn bold_last_block(&self, k: usize, t: f64) -> Option<(f64, f64, f64)> {
        self.bold_fx
            .blocks
            .iter()
            .rev()
            .find(|b| b.1 == k && b.0 <= t)
            .map(|b| (b.0, b.2, b.3))
    }

    /// The drawn ball of fighter `k`, if it was drawn this frame.
    fn bold_ball(drawn: &[DrawnBall], k: usize) -> Option<&DrawnBall> {
        drawn.iter().find(|d| d.k == k)
    }

    /// S7, first of the crisp layers: the ring dims round the fighter whose low-HP move just started. Drawn
    /// before every other bold mark so that those stay bright.
    pub(crate) fn bold_spotlight(&self, img: &mut Pic, t: f64, drawn: &[DrawnBall]) {
        for b in self.bold_fx.banners.iter().filter(|b| b.low) {
            let u = t - b.t0;
            if !(0.0..SPOT_S).contains(&u) {
                continue;
            }
            let Some(d) = Self::bold_ball(drawn, b.who) else {
                continue;
            };
            // eased in over a fifth of a second: a dim, never a flash
            let k = ease_out(u / 0.2) * (1.0 - ease_out((u - (SPOT_S - 0.25)) / 0.25));
            // the pool of light opens a little as it lands
            let r = d.rp * (2.2 + 0.5 * ease_out(u / 0.2));
            px::dim_outside(img, d.s, r, d.rp * 1.4, 1.0 - 0.38 * k, &bayer);
            px::ring(img, d.s, r, 1.5, pale(self.col[b.who]), 0.55 * k);
        }
    }

    /// The crisp bold marks after the grade: speed-up (S6), shields (S9), statuses (S12), level-ups (S11) and
    /// the hit pops (S4), in that order, so a hit always reads on top.
    pub(crate) fn bold_marks(&self, img: &mut Pic, cam: &Cam, t: f64, fs: &[BallState; 2], drawn: &[DrawnBall]) {
        let g = &self.show.grid;
        let in_fight = g.t_fight <= t && t < self.t_hs;
        let sh = &self.bold_fx;
        if in_fight {
            for d in drawn {
                let (k, st) = (d.k, &fs[d.k]);
                if st.shattered || !st.vis {
                    continue;
                }
                let col = self.col[k];
                // S6: afterimages, the circling note and the draining ring
                if st.haste > 0.0 {
                    for j in 1..=3u32 {
                        let past = self.fighter(k, (t - 0.05 * f64::from(j)).max(g.t_fight));
                        let s1 = cam.wp(&[scene::to_world(past.p[0], past.p[1], R + past.lift)])[0];
                        if dmath::hypot(s1.0 - d.s.0, s1.1 - d.s.1) > d.rp * 0.45 {
                            px::ring(img, s1, d.rp, 1.5, pale(col), 0.5 / f64::from(j));
                        }
                    }
                    let ang = t * 7.0 + k as f64 * 2.0;
                    let c = (
                        d.s.0 + dmath::cos(ang) * (d.rp + 9.0),
                        d.s.1 + dmath::sin(ang) * (d.rp + 9.0) * 0.45 - d.rp * 0.2,
                    );
                    note(img, c, pale(col), 1.0);
                    if let Some(left) = Shared::left_in(&sh.haste[k], t) {
                        let a0 = -dmath::PI / 2.0;
                        px::ink_arc(
                            img,
                            d.s,
                            d.rp + 4.0,
                            1.6,
                            a0,
                            a0 + dmath::TAU * left.max(0.02),
                            pale(col),
                            0.95,
                        );
                    }
                }
                // S9: the shield's ring thins as its time runs down, and flares where a hit was blocked
                if st.shield > 0.0 {
                    let left = Shared::left_in(&sh.shield[k], t).unwrap_or(1.0);
                    let ice = [200.0, 232.0, 255.0];
                    px::ring(img, d.s, d.rp + 7.0, 1.0 + 2.5 * left, ice, 0.55 + 0.3 * left);
                }
                for &(tb, who, x, z) in sh.blocks.iter().filter(|b| b.1 == k) {
                    let u = (t - tb) / BLOCK_S;
                    if !(0.0..1.0).contains(&u) || who != k {
                        continue;
                    }
                    let hit = cam.wp(&[scene::to_world(x, z, R)])[0];
                    let ang = angle(d.s, hit);
                    px::arc(img, d.s, d.rp + 7.0, 4.0, ang - 0.7, ang + 0.7, WHITE, 1.0 - u);
                    let edge = (
                        d.s.0 + dmath::cos(ang) * (d.rp + 7.0),
                        d.s.1 + dmath::sin(ang) * (d.rp + 7.0),
                    );
                    px::glint(img, edge, 4.0 + 6.0 * ease_out(u), WHITE, 1.0 - u);
                }
            }
            // S12: the statuses the looks report, and the engine's own slow
            let mut statuses: Vec<Status> = Vec::new();
            for l in &self.looks {
                statuses.extend(l.bold_status(self, t));
            }
            for (k, st) in fs.iter().enumerate() {
                if st.slow > 0.0 && !statuses.iter().any(|s| s.who == k && s.kind == StatusKind::Slow) {
                    statuses.push(Status {
                        who: k,
                        kind: StatusKind::Slow,
                        left: None,
                    });
                }
            }
            for s in &statuses {
                let Some(d) = Self::bold_ball(drawn, s.who) else {
                    continue;
                };
                if fs[s.who].shattered {
                    continue;
                }
                self.bold_status_mark(img, d, s, t);
            }
        }
        // S11: the level-up pop
        let tc = self.fx_clock(t);
        for &(tl, k) in &sh.levels {
            let u = (tc - tl) / LEVEL_S;
            if !(0.0..1.0).contains(&u) {
                continue;
            }
            let Some(d) = Self::bold_ball(drawn, k) else {
                continue;
            };
            let fade = 1.0 - ease_out((u - 0.6) / 0.4);
            px::ink_ring(
                img,
                d.s,
                d.rp + 2.0 + 10.0 * ease_out(u / 0.5),
                1.5,
                GOLD,
                fade * (1.0 - u * 0.5),
            );
            let top = (d.s.0, d.s.1 - d.rp - 6.0 - 8.0 * ease_out(u / 0.4));
            px::ink_arrow(img, (top.0, top.1 + 9.0), top, 3.0, 6.0, GOLD, fade);
        }
        // S4: the hit pops
        for h in sh.pops_at(tc) {
            let life = pop_life(h.size);
            let u = (tc - h.t) / life;
            if !(0.0..1.0).contains(&u) {
                continue;
            }
            let c = cam.wp(&[scene::to_world(h.x, h.z, R)])[0];
            let col = pale(self.col[h.att]);
            let (r0, r1, chips, rings) = match h.size {
                HitSize::Tap => (2.5, 5.5, 0u64, 0u32),
                HitSize::Solid => (4.0, 9.0, 5, 1),
                HitSize::Heavy => (6.0, 14.0, 9, 2),
            };
            for j in 0..rings {
                let ur = (u - 0.12 * f64::from(j)).max(0.0);
                let rr = r1 * (0.8 + 1.7 * ease_out(ur));
                px::ink_ring(img, c, rr, 1.5, col, (1.0 - ur) * 0.9);
            }
            for n in 0..chips {
                let ang = rnd(h.seed, n) * dmath::TAU;
                let sp = (26.0 + 30.0 * rnd(h.seed, n + 50)) * if h.size == HitSize::Heavy { 1.35 } else { 1.0 };
                let dt = u * life;
                let q = (
                    c.0 + dmath::cos(ang) * sp * dt * 3.0,
                    c.1 + dmath::sin(ang) * sp * dt * 3.0 + 160.0 * dt * dt,
                );
                px::disc(img, q, 2.2, INK, 1.0 - u);
                px::disc(img, q, 1.3, if n % 2 == 0 { col } else { WHITE }, 1.0 - u);
            }
            // the star itself is gone in the first half
            let us = u * if h.size == HitSize::Tap { 1.0 } else { 2.0 };
            if us < 1.0 {
                let r = r0 + (r1 - r0) * ease_out(us);
                let rot = rnd(h.seed, 99) * 1.5;
                px::ink_star(img, c, r, r * 0.38, 4, rot, WHITE, 1.0);
                if h.size != HitSize::Tap {
                    px::star(img, c, r * 0.55, r * 0.2, 4, rot + dmath::PI / 4.0, col, 1.0);
                }
            }
        }
    }

    /// S12: one status mark on a drawn ball.
    fn bold_status_mark(&self, img: &mut Pic, d: &DrawnBall, s: &Status, t: f64) {
        let (c, rp) = (d.s, d.rp);
        let top = (c.0, c.1 - rp - 5.0);
        match s.kind {
            StatusKind::Stun => {
                for n in 0..3 {
                    let ang = t * 5.5 + f64::from(n) * dmath::TAU / 3.0;
                    let q = (top.0 + dmath::cos(ang) * rp * 0.85, top.1 + dmath::sin(ang) * rp * 0.28);
                    // the far side of the orbit is smaller, so the stars read as circling
                    let r = if dmath::sin(ang) < 0.0 { 3.0 } else { 4.2 };
                    px::ink_star(img, q, r, r * 0.42, 5, ang, GOLD, 1.0);
                }
            }
            StatusKind::Pin => {
                let steel = [214.0, 224.0, 240.0];
                for n in 0..8 {
                    let a0 = f64::from(n) * dmath::TAU / 8.0 + t * 0.8;
                    px::ink_arc(img, c, rp + 3.5, 1.6, a0, a0 + 0.5, steel, 1.0);
                }
                // the pin: a round head over the ball and its needle
                px::ink_line(img, (top.0, top.1 - 2.0), (top.0, top.1 + 7.0), 1.6, steel, 1.0);
                px::ink_disc(img, (top.0, top.1 - 4.0), 3.0, [236.0, 70.0, 80.0], 1.0);
            }
            StatusKind::Slow => {
                let blue = [96.0, 160.0, 255.0];
                px::disc(img, c, rp, blue, 0.3);
                let bob = (t * 2.2).rem_euclid(1.0) * 5.0;
                for side in [-1.0, 1.0] {
                    let x = c.0 + side * (rp + 6.0);
                    px::ink_arrow(img, (x, c.1 - 6.0 + bob), (x, c.1 + 3.0 + bob), 1.8, 4.0, blue, 1.0);
                }
            }
            StatusKind::Silence => {
                let grey = [150.0, 150.0, 156.0];
                px::disc(img, c, rp, grey, 0.45);
                for side in [-1.0, 1.0] {
                    px::ink_cross(
                        img,
                        (top.0 + side * 6.0, top.1 - 1.0),
                        3.0,
                        1.8,
                        [240.0, 240.0, 244.0],
                        1.0,
                    );
                }
            }
            StatusKind::Sour => {
                let green = [156.0, 222.0, 64.0];
                px::disc(img, c, rp, green, 0.3);
                // a tuner gauge: a half dial with the needle swinging well off the middle
                let g = (top.0, top.1 + 1.0);
                px::ink_arc(img, g, 7.0, 1.6, dmath::PI, dmath::TAU, [240.0, 240.0, 244.0], 1.0);
                let ang = -dmath::PI / 2.0 + 0.9 * dmath::sin(t * 9.0).signum() * (0.6 + 0.3 * dmath::sin(t * 23.0));
                let tip = (g.0 + dmath::cos(ang) * 6.5, g.1 + dmath::sin(ang) * 6.5);
                px::ink_line(img, g, tip, 1.4, [255.0, 96.0, 80.0], 1.0);
            }
        }
        if let Some(left) = s.left {
            let a0 = -dmath::PI / 2.0;
            let col = match s.kind {
                StatusKind::Stun => GOLD,
                StatusKind::Pin => [214.0, 224.0, 240.0],
                StatusKind::Slow => [96.0, 160.0, 255.0],
                StatusKind::Silence => [220.0, 220.0, 226.0],
                StatusKind::Sour => [156.0, 222.0, 64.0],
            };
            px::ink_arc(
                img,
                c,
                rp + 1.5,
                1.4,
                a0,
                a0 + dmath::TAU * left.clamp(0.02, 1.0),
                col,
                0.95,
            );
        }
    }

    /// S10: a spark runs both ways along a rope from where a ball hit it. Drawn on the `float32` picture after
    /// the front ropes.
    pub(crate) fn bold_rope_sparks(&self, arr: &mut Img<f32>, cam: &Cam, t: f64) {
        const N: usize = 31;
        let i0 = self.rope_ev.partition_point(|r| r.t < t - ROPE_SPARK_S);
        let i1 = self.rope_ev.partition_point(|r| r.t <= t);
        for r in self.rope_ev.get(i0..i1).unwrap_or(&[]) {
            if r.side > 3 || r.spd < 1.0 {
                continue;
            }
            let u = (t - r.t) / ROPE_SPARK_S;
            let (a, b) = self.side_ends(r.side);
            let ab = [b[0] - a[0], b[1] - a[1]];
            let len2 = ab[0] * ab[0] + ab[1] * ab[1];
            if bad(len2) {
                continue;
            }
            let uc = (((r.x - a[0]) * ab[0] + (r.z - a[1]) * ab[1]) / len2).clamp(0.0, 1.0);
            for y in scene::ROPE_Y {
                let pts = cam.wp(&self.rope_points(r.side, y, t, N));
                for dir in [-1.0, 1.0] {
                    let at = uc + dir * 0.75 * u;
                    if !(0.0..=1.0).contains(&at) {
                        continue;
                    }
                    let q = pts[((at * (N - 1) as f64).round() as usize).min(N - 1)];
                    px::disc(arr, q, 2.4, [255.0, 244.0, 190.0], 1.0 - u);
                    px::disc(arr, q, 1.2, WHITE, 1.0 - u * 0.5);
                }
            }
        }
    }

    /// N4 and S7 on the finished bytes: each fighter's special move on a ribbon from its own side (the left
    /// fighter's higher, the right one's just under it, so two can show at once), bigger for a low-HP move.
    pub(crate) fn bold_banners(&self, img: &mut Rgb8, log: &mut DrawLog, t: f64) {
        for b in &self.bold_fx.banners {
            let u = t - b.t0;
            let hold = b.t1 - b.t0;
            if !(0.0..hold).contains(&u) {
                continue;
            }
            let left = b.who == 0;
            let scale: u32 = if b.low && b.name.chars().count() <= 14 { 3 } else { 2 };
            let mask = font::text_mask(&b.name, scale);
            let (tw, th) = (mask.w as f64, mask.h as f64);
            let hgt = th + 14.0;
            let y = if left { 100.0 } else { 100.0 + 40.0 };
            let wd = (tw + 38.0 + 30.0).max(150.0);
            let k = ease_out(u / 0.14) * (1.0 - super::ease_in((u - (hold - 0.2)) / 0.2));
            let off = (1.0 - k) * -wd;
            // the ribbon in "left" coordinates, mirrored for the right fighter
            let mx = |x: f64| if left { x + off } else { W as f64 - (x + off) };
            let col = self.col[b.who];
            let body = [
                (mx(-2.0), y),
                (mx(wd), y),
                (mx(wd - 16.0), y + hgt),
                (mx(-2.0), y + hgt),
            ];
            px::ink_poly(img, &body, deep(col), 1.0);
            px::poly(
                img,
                &[
                    (mx(-2.0), y),
                    (mx(wd - 1.0), y),
                    (mx(wd - 3.0), y + 3.0),
                    (mx(-2.0), y + 3.0),
                ],
                col,
                1.0,
            );
            px::line(
                img,
                (mx(0.0), y + hgt - 2.5),
                (mx(wd - 17.0), y + hgt - 2.5),
                1.0,
                pale(col),
                1.0,
            );
            px::ink_disc(img, (mx(21.0), y + hgt / 2.0), 7.5, col, 1.0);
            // the name stands still once the ribbon is nearly in: it never slides through the edge of the frame
            if k > 0.85 {
                let tx = if left { 36 } else { W as i32 - 36 - mask.w as i32 };
                let ty = (y + (hgt - th) / 2.0).round() as i32;
                font::stamp(img, &mask, tx, ty, [255, 255, 255], Some(font::OUTLINE), true, 1.0);
                log.note(tx, ty, tx + mask.w as i32, ty + mask.h as i32, Kind::Caption, &b.name);
            }
            // a white flash as it lands
            let fl = 1.0 - u / 0.1;
            if fl > 0.0 {
                px::poly(img, &body, [255.0, 255.0, 255.0], fl * 0.5);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_are_sized_by_their_damage() {
        assert_eq!(HitSize::of_damage(0.5), HitSize::Tap);
        assert_eq!(HitSize::of_damage(1.6), HitSize::Solid);
        assert_eq!(HitSize::of_damage(5.9), HitSize::Solid);
        assert_eq!(HitSize::of_damage(6.0), HitSize::Heavy);
        assert!(HitSize::Tap < HitSize::Heavy);
    }

    #[test]
    fn a_heavy_hit_squashes_only_its_victim_and_only_briefly() {
        let s = Shared {
            pops: vec![Pop {
                t: 10.0,
                att: 0,
                vic: 1,
                x: 0.0,
                z: 0.0,
                size: HitSize::Heavy,
                seed: 3,
            }],
            ..Shared::default()
        };
        assert!(s.squash(0, 10.02).is_none());
        let (sx, sy) = s.squash(1, 10.02).expect("the victim is squashed");
        assert!(sx > 1.0 && sy < 1.0);
        assert!(s.squash(1, 10.5).is_none());
        assert!(s.squash(1, 9.9).is_none());
        assert_eq!(s.jolts(), vec![(10.0, 1.3)]);
    }

    #[test]
    fn the_share_left_runs_from_1_to_0() {
        let list = [(2.0, 4.0)];
        assert_eq!(Shared::left_in(&list, 2.0), Some(1.0));
        assert_eq!(Shared::left_in(&list, 3.0), Some(0.5));
        assert_eq!(Shared::left_in(&list, 4.0), None);
    }
}
