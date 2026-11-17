//! The Punchy look: the Studio's own layer over the kit's picture. Hit impact (a micro hit-stop, a shockwave, a
//! kick of the struck ball, debris), a subtle action camera with a dramatic zoom on the K.O. shatter, low-HP drama
//! (cracks, smoke, a heartbeat on the HP bar), an arena that reacts to the music and the lead, and a bigger K.O.
//!
//! Everything is read from the recording's events and arrays and the show's beat grid, built once when the
//! picture loads ([`Punch::new`]) and asked for its state at any time, so frames stay pure functions of their
//! number. A fight file with `picture.look = "classic"` (or none) never builds a [`Punch`]: its frames are the
//! kit's, bit for bit.

pub mod anime;
pub mod clock;
mod draw;

pub use clock::PunchClock;

use bf_core::dmath;
use bf_core::py;

use crate::camera::{Key, V3};
use crate::config::{FPS, ease, ss};
use crate::fx::particles::{Emit, Particles};
use crate::ring::PictureCtx;
use crate::scene::{self, R_BALL as R};

/// HP under which a fighter's ball starts to crack and smoke.
pub const LOW_HP: f64 = 25.0;
/// No camera move from here on: the end of the file loops into frame 0 on the kit's own camera.
const CAM_END: f64 = 58.0;

/// One hit as the Punchy look reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PunchHit {
    /// When it landed.
    pub t: f64,
    /// The attacker and the victim.
    pub att: usize,
    pub vic: usize,
    /// Its strength 0.25..1 from its damage.
    pub s: f64,
    /// The contact point in the ring frame.
    pub x: f64,
    pub z: f64,
    /// The unit direction from the attacker to the victim in the ring frame.
    pub dir: [f64; 2],
}

/// The Punchy layer of one fight.
pub struct Punch {
    /// The picture's clock (the micro hit-stop).
    pub clock: PunchClock,
    /// The hits that make an impact, in time order.
    pub hits: Vec<PunchHit>,
    /// When each fighter first fell under [`LOW_HP`] before the K.O.
    pub t_low: [Option<f64>; 2],
    /// The lead changes: `(time, the new leader)`.
    pub leads: Vec<(f64, usize)>,
    /// Debris, embers and smoke.
    pub debris: Particles,
    /// The anime layer: specials, ability spans, styles.
    pub anime: anime::Anime,
    t_fight: f64,
    t_ko: f64,
    t_hs: f64,
    t_solo: f64,
    #[allow(dead_code)]
    lose: usize,
    ko_pt: [f64; 2],
    col: [V3; 2],
    #[allow(dead_code)]
    dark: [V3; 2],
}

/// The strength of a hit of `dmg` damage.
pub fn strength(dmg: f64) -> f64 {
    (dmg / 18.0).clamp(0.25, 1.0)
}

impl Punch {
    /// The Punchy layer of the picture `p` (every other field of `p` already built).
    pub fn new(p: &PictureCtx) -> Punch {
        let g = &p.show.grid;
        let mut hits = Vec::new();
        for e in &p.rec.events {
            if e.kind != "hit" || e.t < g.t_fight || e.t >= p.t_ko + 1e-9 {
                continue;
            }
            let Some(att) = crate::ring::owner(e) else { continue };
            let vic = e.i(3);
            if !(0..2).contains(&vic) {
                continue;
            }
            let vic = vic as usize;
            let (dmg, how) = (e.f(4), e.s(7));
            let quiet = p.looks[att].quiet_hits().contains(&how);
            if !(dmg > 0.0) || how == "key" || quiet {
                continue;
            }
            let i = p.sim_i(e.t);
            let (pa, pv) = (p.run.p(i, att), p.run.p(i, vic));
            let d = [pv[0] - pa[0], pv[1] - pa[1]];
            let n = (d[0] * d[0] + d[1] * d[1]).sqrt();
            let dir = if n > 1e-6 { [d[0] / n, d[1] / n] } else { [1.0, 0.0] };
            hits.push(PunchHit {
                t: e.t,
                att,
                vic,
                s: strength(dmg),
                x: e.f(5),
                z: e.f(6),
                dir,
            });
        }
        let clock = PunchClock::new(&hits.iter().map(|h| (h.t, h.s)).collect::<Vec<_>>(), g.t_fight, p.t_ko);
        // low HP and lead changes, sampled frame by frame through the fight
        let mut t_low = [None, None];
        let mut leads = Vec::new();
        let mut leader: Option<usize> = None;
        let fr = 1.0 / f64::from(FPS);
        let mut t = g.t_fight;
        while t < p.t_ko {
            let hp = [p.hp_at(0, t), p.hp_at(1, t)];
            for k in 0..2 {
                if t_low[k].is_none() && hp[k] < LOW_HP && hp[k] > 0.0 {
                    t_low[k] = Some(t);
                }
            }
            let now = if hp[0] > hp[1] + 3.0 {
                Some(0)
            } else if hp[1] > hp[0] + 3.0 {
                Some(1)
            } else {
                leader
            };
            if now != leader {
                if leader.is_some() {
                    leads.push((t, now.unwrap_or(0)));
                }
                leader = now;
            }
            t += fr;
        }
        // debris on every hit, embers on a hit taken under LOW_HP, smoke while under it
        let mut debris = Particles::new(p.fx_seed ^ 0x5EED_9C4C, 900);
        for h in &hits {
            let pos = scene::to_world(h.x, h.z, R);
            let push = scene::to_world(h.dir[0], h.dir[1], 0.0);
            debris.emit(
                h.t,
                pos,
                "chips",
                &Emit {
                    n: Some((10.0 + 18.0 * h.s).round() as i64),
                    dir: [push[0], 0.9, push[2]],
                    col: Some(vec![p.col[h.vic], p.dark[h.vic], [235.0, 230.0, 220.0]]),
                    scale: 0.7 + 0.5 * h.s,
                    ..Emit::default()
                },
            );
            {
                debris.emit(
                    h.t,
                    pos,
                    "sparks",
                    &Emit {
                        n: Some((8.0 + 16.0 * h.s).round() as i64),
                        dir: [push[0], 0.5, push[2]],
                        col: Some(vec![p.col[h.att].map(|c| c * 0.5 + 128.0)]),
                        scale: 0.8,
                        ..Emit::default()
                    },
                );
            }
            if t_low[h.vic].is_some_and(|tl| h.t >= tl) {
                debris.emit(
                    h.t,
                    pos,
                    "embers",
                    &Emit {
                        n: Some(5),
                        ..Emit::default()
                    },
                );
            }
        }
        let anime = anime::Anime::new(p, &hits);
        for sp in &anime.specials {
            let pos = scene::to_world(sp.at[0], sp.at[1], R);
            let hot = p.col[sp.k].map(|c| c * 0.5 + 128.0);
            for (kind, n, col) in [
                ("sparks", 40, hot),
                ("embers", 30, [255.0, 190.0, 110.0]),
                ("chips", 30, p.col[1 - sp.k]),
                ("shards", 16, p.col[sp.k]),
                ("smoke", 10, [90.0, 86.0, 96.0]),
            ] {
                debris.emit(
                    sp.t_hit,
                    pos,
                    kind,
                    &Emit {
                        n: Some(n),
                        col: Some(vec![col, [255.0, 250.0, 235.0]]),
                        scale: 1.3,
                        ..Emit::default()
                    },
                );
            }
        }
        for k in 0..2 {
            let Some(tl) = t_low[k] else { continue };
            let mut t = tl;
            while t < p.t_ko {
                let q = p.run.p(p.sim_i(t), k);
                debris.emit(
                    t,
                    scene::to_world(q[0], q[1], R * 1.5),
                    "smoke",
                    &Emit {
                        n: Some(1),
                        col: Some(vec![[70.0, 66.0, 70.0], [96.0, 92.0, 96.0]]),
                        scale: 0.8,
                        ..Emit::default()
                    },
                );
                t += 0.14;
            }
        }
        Punch {
            clock,
            hits,
            t_low,
            leads,
            debris,
            anime,
            t_fight: g.t_fight,
            t_ko: p.t_ko,
            t_hs: p.t_hs,
            t_solo: p.t_solo,
            lose: p.lose,
            ko_pt: p.p_ko[p.lose],
            col: p.col,
            dark: p.dark,
        }
    }

    /// The hits that landed in `t - span ..= t`, with their age.
    fn recent(&self, t: f64, span: f64) -> impl Iterator<Item = (&PunchHit, f64)> {
        let lo = self.hits.partition_point(|h| h.t < t - span);
        let hi = self.hits.partition_point(|h| h.t <= t);
        self.hits[lo..hi].iter().map(move |h| (h, t - h.t))
    }

    /// The struck ball's kick at `t`: an offset in the ring frame that jolts along the hit and springs back.
    pub fn kick(&self, k: usize, t: f64) -> [f64; 2] {
        let mut o = [0.0, 0.0];
        for (h, u) in self.recent(t, 0.35) {
            if h.vic != k {
                continue;
            }
            let rise = (u / 0.035).min(1.0);
            let a = 0.17 * h.s * dmath::sin(rise * dmath::PI / 2.0) * dmath::exp(-u / 0.09);
            o[0] += h.dir[0] * a;
            o[1] += h.dir[1] * a;
        }
        o
    }

    /// The K.O. zoom's strength at `t`, 0..1.
    pub fn ko_zoom(&self, t: f64) -> f64 {
        if t < self.t_ko || t >= CAM_END {
            return 0.0;
        }
        let back = (self.t_hs + 2.4).min(self.t_solo).max(self.t_hs + 0.8);
        ease(((t - self.t_ko) / 0.35).min(1.0)) * (1.0 - ss(self.t_hs + 0.6, back, t))
    }

    /// The camera key at `t` with the Punchy moves: small push-ins on heavy hits, a slow push through the final
    /// exchange, the K.O. zoom onto the loser's shatter.
    pub fn camera_key(&self, mut key: Key, t: f64) -> Key {
        if t < self.t_fight || t >= CAM_END {
            return key;
        }
        let mut push = 0.0;
        let mut aim = [0.0, 0.0, 0.0];
        let mut aim_w = 0.0;
        for (h, u) in self.recent(t, 0.7) {
            if h.s < 0.5 {
                continue;
            }
            let env = ease((u / 0.06).min(1.0)) * (1.0 - ss(0.1, 0.7, u));
            let v = 0.055 * h.s * env;
            if v > push {
                push = v;
            }
            let w = scene::to_world(h.x, h.z, 0.3);
            for c in 0..3 {
                aim[c] += w[c] * env * 0.12;
            }
            aim_w += env * 0.12;
        }
        let ko = self.ko_zoom(t);
        key.dist *= 1.0 - self.zoom_of(push, t);
        let kp = scene::to_world(self.ko_pt[0], self.ko_pt[1], 0.4);
        let w_hit = aim_w.min(0.25) * (1.0 - ko);
        let w_ko = 0.75 * ko;
        for c in 0..3 {
            let hit_pt = if aim_w > 1e-9 { aim[c] / aim_w } else { key.target[c] };
            key.target[c] = key.target[c] * (1.0 - w_hit - w_ko) + hit_pt * w_hit + kp[c] * w_ko;
        }
        key
    }

    /// How far the camera has pushed in at `t` (a share of its distance, 0..0.5), given the hits' push.
    fn zoom_of(&self, push: f64, t: f64) -> f64 {
        let final_k = if t < self.t_ko {
            ss(self.t_ko - 6.0, self.t_ko, t)
        } else {
            0.0
        };
        (push + 0.07 * final_k + 0.42 * self.ko_zoom(t)).min(0.5)
    }

    /// The hits' push-in at `t`.
    fn push(&self, t: f64) -> f64 {
        self.recent(t, 0.7)
            .filter(|(h, _)| h.s >= 0.5)
            .map(|(h, u)| 0.055 * h.s * ease((u / 0.06).min(1.0)) * (1.0 - ss(0.1, 0.7, u)))
            .fold(0.0, f64::max)
    }

    /// The lens's crisp band at `t`: `sharp` widened as the camera pushes in, so what was sharp stays sharp.
    pub fn sharp(&self, t: f64, sharp: f64) -> f64 {
        if t < self.t_fight || t >= CAM_END {
            return sharp;
        }
        let z = self.zoom_of(self.push(t), t);
        (sharp + z * 0.5) / (1.0 - z)
    }

    /// The extra camera shake at `t` in screen pixels: heavy hits and the K.O.
    pub fn shake(&self, t: f64) -> (f64, f64) {
        let mut a: f64 = 0.0;
        for (h, u) in self.recent(t, 0.3) {
            if h.s >= 0.65 {
                a = a.max(2.6 * h.s * dmath::exp(-u / 0.06));
            }
        }
        if t >= self.t_ko && t < self.t_ko + 0.6 {
            a = a.max(4.0 * dmath::exp(-(t - self.t_ko) / 0.14));
        }
        if a < 0.05 {
            return (0.0, 0.0);
        }
        let w = 2.0 * dmath::PI * 31.0 * t;
        (a * dmath::sin(w), 0.7 * a * dmath::cos(1.3 * w))
    }

    /// How much the final exchange is under way at `t`, 0..1 (the crowd jumps harder).
    pub fn final_k(&self, t: f64) -> f64 {
        if t >= self.t_ko {
            0.0
        } else {
            ss(self.t_ko - 6.0, self.t_ko - 1.0, t)
        }
    }

    /// The LED boards' colour at `t` for a board at angle `am`: the kit's colour `base` (with its beat chase)
    /// pulsed harder on the bar, leaning toward the leader, with a sweep on a lead change. `hp` is both fighters'
    /// HP at `t`.
    pub fn led(&self, base: V3, am: f64, t: f64, hp: [f64; 2], bar: f64, beat: f64) -> V3 {
        if t < self.t_fight || t >= self.t_ko {
            return base;
        }
        let ph_bar = py::fmod_py(t, bar);
        let bar_k = dmath::exp(-ph_bar / 0.25);
        let beat_k = dmath::exp(-py::fmod_py(t, beat) / 0.12);
        let gain = 0.82 + 0.38 * bar_k.max(0.6 * beat_k);
        let (lead, gap) = if hp[0] >= hp[1] {
            (0, hp[0] - hp[1])
        } else {
            (1, hp[1] - hp[0])
        };
        let lean = (gap / 60.0).clamp(0.0, 0.45);
        let lc = self.col[lead];
        let mut c: V3 = std::array::from_fn(|i| (base[i] * (1.0 - lean) + lc[i] * 0.8 * lean) * gain);
        if let Some(&(tc, who)) = self.leads.iter().rev().find(|(tc, _)| *tc <= t) {
            let u = (t - tc) / 0.7;
            if u < 1.0 {
                let front = 2.0 * dmath::PI * u;
                let d = py::fmod_py(am - front + dmath::PI, 2.0 * dmath::PI) - dmath::PI;
                let band = (1.0 - d.abs() / 0.7).max(0.0) * (1.0 - u);
                let nc = self.col[who];
                for i in 0..3 {
                    c[i] = c[i] * (1.0 - band) + (nc[i] * 0.7 + 90.0) * band;
                }
            }
        }
        c.map(|v| v.clamp(0.0, 255.0))
    }

    /// The bar pulse of the floor's light pools at `t` (0..1).
    pub fn bar_pulse(&self, t: f64, bar: f64) -> f64 {
        if t < self.t_fight || t >= self.t_ko {
            return 0.0;
        }
        dmath::exp(-py::fmod_py(t, bar) / 0.3)
    }

    /// The fighter `k`'s low-HP state at `t`: how many hits it has taken since it fell under [`LOW_HP`] (plus
    /// one), or `None` when it is not low (or after the K.O.).
    pub fn low(&self, k: usize, t: f64) -> Option<usize> {
        let tl = self.t_low[k]?;
        if t < tl || t >= self.t_ko {
            return None;
        }
        Some(1 + self.hits.iter().filter(|h| h.vic == k && h.t >= tl && h.t <= t).count())
    }
}

pub use draw::*;
