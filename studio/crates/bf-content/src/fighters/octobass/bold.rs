//! OCTOBASS with the bold effects (`picture.effects = "bold"`; the owner's picks 7a to 7d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **LEVER PUNCH** (7a): a thick, outlined two-part arm with a brass elbow that stands up while the arm is
//!   short and unfolds flat as it reaches; a big red felt fist that squashes flat where it stops on the enemy
//!   (the fight itself holds it there). The arms of a volley punch low, middle and high, and the third is the
//!   heaviest: a thicker arm, a bigger fist with a brass cuff.
//! - **FOOT PEDAL** (7b): a landing mark closes in on a hinged brass plate that trembles more and more under the
//!   enemy; on the snap the plate flips up on its hinge over a dark pit and the enemy is thrown up spinning; it
//!   lands in a puff with stun stars.
//! - **SEVEN LEVERS** (7c): all seven levers show folded round the ball, then ripple out one after another as a
//!   fan of bowed arms, each stopping squashed on the enemy; the seventh is a thick arm with a huge fist and its
//!   hit is a heavy one; the fan ripples back in to the folded levers.
//! - **BELOW HEARING** (7d): the ring dims while one slow ring rolls out lifting the canvas in tiles, with dust
//!   rising off its edge; just before it lands both balls, the ring and the dust hold still and the colour
//!   drains; then the boom: a quake ring bursts from the enemy, the floor cracks under it for two beats and the
//!   enemy is marked stunned.

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::roster::FighterCfg;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, Status, StatusKind};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::BallState;
use bf_picture::scene::{A, R_BALL as R, to_world};

use super::{AIR_S, BRASS, HOT, OctobassLook, VARNISH, how, state};

/// The arms' wood, light enough to read on the canvas.
const WOOD_HI: V3 = [136.0, 88.0, 46.0];
/// The felt of a fist, bright.
const FELT_HI: V3 = [212.0, 56.0, 44.0];
/// The dark under a flipped pedal plate.
const PIT: V3 = [26.0, 18.0, 30.0];
/// Seconds everything holds still before BELOW HEARING lands.
const HOLD_S: f64 = 4.0 / 30.0;
/// Seconds a snapped plate stands up and falls back.
const FLIP_S: f64 = 0.5;
/// Seconds the puff under a landing enemy shows.
const PUFF_S: f64 = 0.3;
/// Seconds the boom's ring runs.
const BOOM_S: f64 = 0.5;
/// How high a pedal throws the enemy, in ball radii.
const THROW: f64 = 1.8;

/// One lever arm as the bold look draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Arm {
    butt: [f64; 2],
    tip: [f64; 2],
    /// Held on the enemy.
    held: bool,
    /// One of SEVEN LEVERS (else a LEVER PUNCH).
    seven: bool,
    /// Its place in its volley (0 to 2) or in the seven (0 to 6).
    idx: usize,
    /// One of the seven that is not out: folded at the ball.
    idle: bool,
}

/// One painter item of the bold look.
enum BItem {
    Arm(Arm),
    /// The pedal plate: x, z, half its size, its tilt up in radians, its tremble.
    Plate(f64, f64, f64, f64, f64),
}

/// What the bold look works out once from the recording.
#[derive(Clone, Debug, Default)]
pub(super) struct Bold {
    /// Every arm fired: when, whether one of the seven, its place in its volley or in the seven.
    fires: Vec<(f64, bool, usize)>,
    /// SEVEN LEVERS: from its start until its last arm is back.
    sevens: Vec<(f64, f64)>,
    /// When each seventh lever fired.
    sevenths: Vec<f64>,
    /// The full reach of a LEVER PUNCH and of one of the seven, in ring units.
    reach: (f64, f64),
    /// SEVEN LEVERS' fan, in radians.
    fan: f64,
    /// The pedal plate's radius, and the seconds its stun and BELOW HEARING's last.
    pedal_r: f64,
    stun_pedal: f64,
    stun_below: f64,
}

/// A number from 0 to 1 that is always the same for the same `(a, b)`.
fn hash(a: i64, b: i64) -> f64 {
    let v = dmath::sin(a as f64 * 12.9898 + b as f64 * 78.233) * 43_758.545_3;
    v - v.floor()
}

fn unit(dx: f64, dz: f64) -> (f64, f64) {
    let n = dmath::hypot(dx, dz).max(1e-9);
    (dx / n, dz / n)
}

/// The distance from `(x, z)` to the segment `a`..`b`.
fn seg_dist(x: f64, z: f64, a: [f64; 2], b: [f64; 2]) -> f64 {
    let (vx, vz) = (b[0] - a[0], b[1] - a[1]);
    let l2 = vx * vx + vz * vz;
    let u = if l2 > 1e-12 {
        (((x - a[0]) * vx + (z - a[1]) * vz) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    dmath::hypot(x - a[0] - u * vx, z - a[1] - u * vz)
}

/// Whether the screen point is on one of the drawn balls.
fn on_ball(drawn: &[DrawnBall], s: (f64, f64)) -> bool {
    drawn.iter().any(|d| dmath::hypot(s.0 - d.s.0, s.1 - d.s.1) < d.rp)
}

/// The boom's cracks: seven jagged dark lines from `(cx, cz)`, `grow` (0 to 1) of their length, darkening the
/// canvas by `k` at most.
fn cracks(ic: &mut [V3], c: &CanvasCtx<'_>, cx: f64, cz: f64, seed: i64, grow: f64, k: f64) {
    if k <= 0.0 || grow <= 0.0 {
        return;
    }
    let mut segs: Vec<([f64; 2], [f64; 2], f64)> = Vec::new();
    for j in 0..7i64 {
        let mut a = j as f64 * dmath::TAU / 7.0 + hash(seed, j) * 0.6;
        let len = (1.0 + 0.9 * hash(seed, j + 20)) * grow;
        let mut p0 = [cx + dmath::cos(a) * 0.3, cz + dmath::sin(a) * 0.3];
        for s in 0..4i64 {
            a += (hash(seed + s, j + 40) - 0.5) * 1.1;
            let p1 = [p0[0] + dmath::cos(a) * len / 4.0, p0[1] + dmath::sin(a) * len / 4.0];
            segs.push((p0, p1, 0.07 - 0.012 * s as f64));
            p0 = p1;
        }
    }
    for (j, light) in ic.iter_mut().enumerate() {
        let (x, z) = (c.sxp[j], c.szp[j]);
        if dmath::hypot(x - cx, z - cz) > 2.4 {
            continue;
        }
        let v = segs
            .iter()
            .map(|&(a, b, w)| 1.0 - seg_dist(x, z, a, b) / w)
            .fold(0.0, f64::max);
        if v > 0.0 {
            let dark = 1.0 - k * (v * 2.5).min(1.0);
            for ch in light.iter_mut() {
                *ch *= dark;
            }
        }
    }
}

impl OctobassLook {
    /// The arms, the seven's windows and the numbers of side `k`, from its events and its roster entry.
    pub(super) fn bold_setup(p: &PictureCtx, k: usize, cfg: &FighterCfg) -> Bold {
        let beat = p.show.grid.beat;
        let raw = &cfg.raw;
        let ability = |slot: &str, key: &str, or: f64| raw["abilities"][slot][key].as_f64().unwrap_or(or);
        let mut b = Bold {
            reach: (
                raw["weapon"]["reach"].as_f64().unwrap_or(2.6) * R,
                ability("unlock2", "reach", 3.0) * R,
            ),
            fan: ability("unlock2", "fan", 60.0).to_radians(),
            pedal_r: ability("unlock1", "radius", 0.9) * R,
            stun_pedal: ability("unlock1", "stun_beats", 1.0) * beat,
            stun_below: ability("low", "stun_beats", 1.0) * beat,
            ..Bold::default()
        };
        let mut last: Option<(f64, usize)> = None;
        let mut n7 = 0;
        for e in p.rec.events.iter().filter(|e| e.who() == Some(k)) {
            match e.kind.as_str() {
                "seven_levers" => {
                    n7 = 0;
                    b.sevens.push((e.t, e.f(3)));
                }
                "lever" if e.i(4) == 1 => {
                    if n7 == 6 {
                        b.sevenths.push(e.t);
                    }
                    b.fires.push((e.t, true, n7.min(6)));
                    n7 += 1;
                }
                "lever" => {
                    // the arms of one volley follow each other within a beat
                    let idx = match last {
                        Some((t0, i)) if e.t - t0 < 0.7 * beat => (i + 1).min(2),
                        _ => 0,
                    };
                    last = Some((e.t, idx));
                    b.fires.push((e.t, false, idx));
                }
                _ => {}
            }
        }
        b
    }

    /// The arms in flight at `t`, each with its place in its volley: the recorded rows are in the order the arms
    /// fired, so they are the last arms fired.
    fn bold_arms(&self, p: &PictureCtx, t: f64) -> Vec<Arm> {
        let rows = self.live(p, t);
        let i1 = self.bold.fires.partition_point(|f| f.0 <= t + 0.02);
        let recent = &self.bold.fires[i1.saturating_sub(24)..i1];
        let mut out = Vec::new();
        for seven in [false, true] {
            let mine: Vec<_> = rows.iter().filter(|r| r.kind == i64::from(seven)).collect();
            let fired: Vec<usize> = recent.iter().filter(|f| f.1 == seven).map(|f| f.2).collect();
            let skip = fired.len().saturating_sub(mine.len());
            for (j, r) in mine.iter().enumerate() {
                if r.a <= 0.02 {
                    continue;
                }
                let idx = fired.get(skip + j).copied().unwrap_or(0);
                // the seven are drawn fanned twice as wide round the ball as the fight has them, so they read
                // as a fan and not as one line (the fists are where the fight has them)
                let me = p.run.p(p.sim_i(t.min(p.t_ko)), self.k);
                let butt = if seven {
                    let ang = dmath::atan2(r.z0 - me[1], r.x0 - me[0]) + (idx as f64 - 3.0) / 3.0 * self.bold.fan / 2.0;
                    [me[0] + dmath::cos(ang) * R, me[1] + dmath::sin(ang) * R]
                } else {
                    [r.x0, r.z0]
                };
                out.push(Arm {
                    butt,
                    tip: [r.b, r.c],
                    held: r.d > 0.5,
                    seven,
                    idx,
                    idle: false,
                });
            }
        }
        out
    }

    /// The BELOW HEARING that lands at or after `t` with its hold on: `(start of the hold, the landing)`.
    fn bold_hold(&self, t: f64) -> Option<(f64, f64)> {
        self.booms
            .iter()
            .map(|&tb| (tb - HOLD_S, tb))
            .find(|&(a, b)| a <= t && t < b)
    }

    /// The enemy thrown by a pedal flies higher; before BELOW HEARING lands both balls hold still.
    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64) {
        if let Some((t_hold, _)) = self.bold_hold(t) {
            st.p = p.run.p(p.sim_i(t_hold), kk);
        }
        if kk == self.k {
            return;
        }
        for &t0 in &self.pops {
            let u = (t - t0) / AIR_S;
            if (0.0..1.0).contains(&u) {
                st.lift = st.lift.max(R * THROW * dmath::sin(dmath::PI * u));
            }
        }
    }

    /// The thrown enemy spins once in the air.
    pub(super) fn bold_roll(&self, kk: usize, t: f64, roll: f64) -> f64 {
        if kk == self.k {
            return roll;
        }
        self.pops
            .iter()
            .map(|&t0| (t - t0) / AIR_S)
            .find(|u| (0.0..1.0).contains(u))
            .map_or(roll, |u| roll + dmath::TAU * u)
    }

    /// The pedal and BELOW HEARING stun; the pedal's stars stay until the enemy has landed.
    pub(super) fn bold_statuses(&self, t: f64) -> Vec<Status> {
        let who = 1 - self.k;
        let stun = |times: &[f64], dur: f64| {
            times
                .iter()
                .rev()
                .find(|&&t0| t0 <= t && t < t0 + dur)
                .map(|&t0| Status {
                    who,
                    kind: StatusKind::Stun,
                    left: Some(1.0 - (t - t0) / dur.max(1e-6)),
                })
        };
        stun(&self.pops, self.bold.stun_pedal.max(AIR_S + 0.2))
            .or_else(|| stun(&self.booms, self.bold.stun_below))
            .into_iter()
            .collect()
    }

    /// The seventh lever's shove is a heavy hit whatever its damage.
    pub(super) fn bold_hit(&self, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        if how(e) == "seven" && self.bold.sevenths.iter().any(|&t7| t7 <= e.t && e.t < t7 + 0.3) {
            return Some(HitSize::Heavy);
        }
        Some(by_damage)
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let (k, t) = (self.k, c.t);
        let o = 1 - k;
        for r in self.live(p, t) {
            match r.kind {
                // FOOT PEDAL: the landing mark closes in on the plate
                2 => floor::target(ic, c, r.x0, r.z0, r.b, 1.0 - r.a, bold::light(BRASS)),
                // BELOW HEARING: one slow ring that lifts the canvas in tiles, holding still before it lands
                3 => {
                    let me = c.fs[k].p;
                    let rad = self.bold_ring_r(r.a, r.b, t);
                    floor::band(
                        ic,
                        c,
                        Family::Quake,
                        me[0],
                        me[1],
                        rad,
                        1.0,
                        t,
                        bold::light(VARNISH),
                        1.6,
                    );
                    floor::ring(ic, c, me[0], me[1], rad, 0.05, bold::light(HOT), 0.9);
                }
                _ => {}
            }
        }
        // the thrown enemy lands in a puff
        for &t0 in &self.pops {
            let u = (t - t0 - AIR_S) / PUFF_S;
            if (0.0..1.0).contains(&u) {
                let e = c.fs[o].p;
                floor::band(
                    ic,
                    c,
                    Family::Air,
                    e[0],
                    e[1],
                    R * (1.0 + 1.4 * bold::ease_out(u)),
                    1.0 - u,
                    t,
                    bold::light(VARNISH),
                    1.2,
                );
            }
        }
        // the boom: a quake ring bursts from where it landed and the floor cracks for two beats
        let life = 2.0 * p.show.grid.beat;
        for &tb in self.booms.iter().filter(|&&tb| tb <= t && t < tb + life.max(BOOM_S)) {
            let age = t - tb;
            let at = p.run.p(p.sim_i(tb), o);
            if age < BOOM_S {
                let u = age / BOOM_S;
                floor::band(
                    ic,
                    c,
                    Family::Quake,
                    at[0],
                    at[1],
                    0.4 + 5.5 * bold::ease_out(u),
                    1.0 - u,
                    t,
                    bold::light(HOT),
                    2.0,
                );
            }
            if age < life {
                let fade = ((1.0 - age / life) * 2.0).min(1.0);
                cracks(
                    ic,
                    c,
                    at[0],
                    at[1],
                    (tb * 1000.0) as i64,
                    bold::ease_out(age / 0.1),
                    0.8 * fade,
                );
            }
        }
    }

    /// The slow ring's radius at `t`: the recorded one, but held still before it lands.
    fn bold_ring_r(&self, age: f64, r: f64, t: f64) -> f64 {
        match self.bold_hold(t) {
            Some((t_hold, _)) if age > 1e-6 => r * ((age - (t - t_hold)) / age).clamp(0.0, 1.0),
            _ => r,
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight {
            return Vec::new();
        }
        let (k, t) = (self.k, f.t);
        let mut out: Vec<(f64, Payload)> = Vec::new();
        let depth_at = |x: f64, z: f64, y: f64| f.cam.depth(&[to_world(x, z, y)])[0];
        let mut arms = if f.fs[k].shattered {
            Vec::new()
        } else {
            self.bold_arms(p, t)
        };
        // SEVEN LEVERS: the levers that are not out show folded round the ball
        if state::within(&self.bold.sevens, t) && !f.fs[k].shattered {
            let (me, foe) = (f.fs[k].p, f.fs[1 - k].p);
            let d0 = dmath::atan2(foe[1] - me[1], foe[0] - me[0]);
            for i in 0..7usize {
                if arms.iter().any(|a| a.seven && a.idx == i) {
                    continue;
                }
                let ang = d0 + (i as f64 - 3.0) / 3.0 * self.bold.fan;
                let butt = [me[0] + dmath::cos(ang) * R, me[1] + dmath::sin(ang) * R];
                arms.push(Arm {
                    butt,
                    tip: [butt[0] + dmath::cos(ang) * 0.3, butt[1] + dmath::sin(ang) * 0.3],
                    held: false,
                    seven: true,
                    idx: i,
                    idle: true,
                });
            }
        }
        for a in arms {
            let mid = [(a.butt[0] + a.tip[0]) / 2.0, (a.butt[1] + a.tip[1]) / 2.0];
            out.push((depth_at(mid[0], mid[1], R) - 0.05, Box::new(BItem::Arm(a))));
        }
        // FOOT PEDAL: the plate trembling under the enemy, then flipped up on its hinge
        let s = self.bold.pedal_r * 1.4;
        for r in self.live(p, t).iter().filter(|r| r.kind == 2) {
            let shake = 0.06 * (1.0 - r.a) * if f.f.is_multiple_of(2) { 1.0 } else { -1.0 };
            out.push((
                depth_at(r.x0, r.z0, 0.0) + 1.0,
                Box::new(BItem::Plate(r.x0, r.z0, s, 0.0, shake)),
            ));
        }
        for &(ts, x, z) in self.pedals.iter().filter(|q| q.0 <= t && t < q.0 + FLIP_S) {
            let age = t - ts;
            let tilt = if age < 0.3 {
                1.05 * bold::ease_out(age / 0.07)
            } else {
                1.05 * (1.0 - (age - 0.3) / (FLIP_S - 0.3))
            };
            out.push((depth_at(x, z, 0.0) + 1.0, Box::new(BItem::Plate(x, z, s, tilt, 0.0))));
        }
        out
    }

    pub(super) fn bold_draw_item(&self, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        match payload.downcast_ref::<BItem>() {
            Some(BItem::Arm(a)) => self.bold_arm(fr, f.cam, a),
            Some(&BItem::Plate(x, z, s, tilt, shake)) => {
                // a square plate set square to the ropes and kept inside them; it is hinged at the edge that
                // faces the camera's right (`across` runs away from the hinge) and flips up from the other,
                // so its face shows as a skewed flap over the pit
                let lim = A - s - 0.05;
                let (cx, cz) = (x.clamp(-lim, lim), z.clamp(-lim, lim));
                let at = |across: f64, along: f64, y: f64| to_world(cx - across + shake, cz + along, y);
                let y = 0.03;
                let (edge, up) = (-s + 2.0 * s * dmath::cos(tilt), y + 2.0 * s * dmath::sin(tilt));
                if tilt > 0.05 {
                    let pit = [at(-s, -s, 0.01), at(s, -s, 0.01), at(s, s, 0.01), at(-s, s, 0.01)];
                    bold::outlined_poly(fr, f.cam, &pit, PIT);
                }
                let plate = [at(-s, -s, y), at(-s, s, y), at(edge, s, up), at(edge, -s, up)];
                bold::outlined_poly(fr, f.cam, &plate, BRASS);
                // two treads across it
                for v in [0.4, 0.7] {
                    let (across, ty) = (-s + (edge + s) * v, y + (up - y) * v + 0.01);
                    fr.line(
                        f.cam,
                        &[at(across, -s * 0.8, ty), at(across, s * 0.8, ty)],
                        bold::deep(BRASS).map(|c| c * 1.5),
                        1,
                    );
                }
                // the hinge
                let hinge = [at(-s, -s, y + 0.03), at(-s, s, y + 0.03)];
                bold::thick_line(fr, f.cam, &hinge, WOOD_HI, 3);
                for q in hinge {
                    bold::blob(fr, f.cam, q, 0.05, BRASS);
                }
            }
            None => {}
        }
    }

    /// One lever arm: two thick parts with a brass elbow that stands up while the arm is short, a brass hinge at
    /// the ball, and the felt fist (squashed flat when it is held on the enemy). The arms of a volley punch low,
    /// middle and high; the seven bow apart like a fan; the last of either is the heaviest.
    fn bold_arm(&self, fr: &mut Frame, cam: &Cam, a: &Arm) {
        let heavy = a.idx == if a.seven { 6 } else { 2 };
        let y0 = if a.seven {
            R * 0.75
        } else {
            R * [0.45, 0.8, 1.15][a.idx.min(2)]
        };
        let full = if a.seven { self.bold.reach.1 } else { self.bold.reach.0 };
        let (dx, dz) = (a.tip[0] - a.butt[0], a.tip[1] - a.butt[1]);
        let d = dmath::hypot(dx, dz);
        let (ux, uz) = unit(dx, dz);
        let half = full / 2.0;
        let rise = if a.idle {
            0.42
        } else {
            0.55 * (half * half - d * d / 4.0).max(0.0).sqrt()
        };
        let bow = if a.seven {
            (a.idx as f64 - 3.0) / 3.0 * 0.5 * (0.3 + 0.7 * (d / full.max(1e-6)).min(1.0))
        } else {
            0.0
        };
        let elbow = to_world(
            (a.butt[0] + a.tip[0]) / 2.0 - uz * bow,
            (a.butt[1] + a.tip[1]) / 2.0 + ux * bow,
            y0 + rise + 0.08,
        );
        let (butt, tip) = (to_world(a.butt[0], a.butt[1], y0), to_world(a.tip[0], a.tip[1], y0));
        bold::thick_line(fr, cam, &[butt, elbow, tip], WOOD_HI, if heavy { 8 } else { 5 });
        bold::blob(fr, cam, butt, 0.06, BRASS);
        bold::blob(fr, cam, elbow, if heavy { 0.12 } else { 0.08 }, BRASS);
        let r = match (heavy, a.seven) {
            _ if a.idle => 0.1,
            (true, _) => 0.26,
            (false, false) => 0.19,
            (false, true) => 0.15,
        };
        if heavy && !a.idle {
            // a brass cuff behind the big fist
            bold::blob(
                fr,
                cam,
                to_world(a.tip[0] - ux * r * 0.9, a.tip[1] - uz * r * 0.9, y0),
                r * 0.6,
                BRASS,
            );
        }
        if a.held {
            // squashed flat against what it hit
            let flat: Vec<V3> = (0..12)
                .map(|i| {
                    let w = f64::from(i) * dmath::TAU / 12.0;
                    let (al, ac) = (dmath::cos(w) * r * 0.45, dmath::sin(w) * r * 1.5);
                    to_world(a.tip[0] + ux * al - uz * ac, a.tip[1] + uz * al + ux * ac, y0)
                })
                .collect();
            bold::outlined_poly(fr, cam, &flat, FELT_HI);
            fr.line(
                cam,
                &[
                    to_world(a.tip[0] + uz * r, a.tip[1] - ux * r, y0 + 0.01),
                    to_world(a.tip[0] - uz * r, a.tip[1] + ux * r, y0 + 0.01),
                ],
                bold::pale(FELT_HI),
                1,
            );
        } else {
            bold::blob(fr, cam, tip, r, FELT_HI);
        }
    }

    /// BELOW HEARING: the ring dims while its ring grows (the ring's own spotlight does the rest).
    pub(super) fn bold_over_ring(&self, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        if let Some(&(a, _)) = self.quiet.iter().find(|&&(a, b)| a <= f.t && f.t < b) {
            let k = (1.0 - 0.3 * ((f.t - a) / 0.3).clamp(0.0, 1.0)) as f32;
            for v in arr.data.iter_mut() {
                *v *= k;
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, t) = (self.k, f.t);
        let hold = self.bold_hold(t);
        // dust rises off the slow ring's edge (and hangs while everything holds still)
        if let Some(r) = self.live(p, t).iter().find(|r| r.kind == 3) {
            let me = f.fs[k].p;
            let rad = self.bold_ring_r(r.a, r.b, t);
            let clock = hold.map_or(t, |(t_hold, _)| t_hold);
            for j in 0..22i64 {
                let ang = j as f64 * dmath::TAU / 22.0 + hash(j, 3) * 0.3;
                let ph = clock * (0.9 + 0.6 * hash(j, 5)) + hash(j, 9);
                let u = ph - ph.floor();
                let rr = rad - 0.12 * hash(j, 7) - 0.1 * u;
                let at = bold::at(
                    f.cam,
                    me[0] + rr * dmath::cos(ang),
                    me[1] + rr * dmath::sin(ang),
                    0.04 + 0.45 * u,
                );
                if !on_ball(drawn, at) {
                    px::ink_disc(img, at, 1.8 - 0.8 * u, bold::pale(VARNISH), 1.0 - u * u);
                }
            }
        }
        // everything holds still: the colour drains for those frames
        if hold.is_some() {
            px::wash(img, 0.7);
        }
    }
}
