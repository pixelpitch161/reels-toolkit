//! MARIMBA with the bold effects (`picture.effects = "bold"`; the owner's picks 4a to 4d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **MALLET RUN** (4a): a big outlined mallet tumbles through the air; every rope it banks off bows there and
//!   rings a rosewood bar on the spot (a shorter bar for each note up the scale), and the head steps from rose to
//!   orange to gold to white; the more bounces, the bigger the hit and the ring round it.
//! - **TREMOLO** (4b): every mallet flies as two heads circling each other and lands as a quick double: the ring's
//!   pop and a second pale star a moment later.
//! - **DEAD STROKE** (4c): the head is dark and matte while the move is armed; a landing mark closes on the enemy
//!   as it comes in; it lands with no burst, stays pressed on the enemy while a flat grey ring shrinks inward, and
//!   the enemy is marked pinned.
//! - **FIVE OCTAVES** (4d): a keyboard of bars curves round the marimba on the canvas, long bars to short, and
//!   lights up bar by bar as each launches its mallets in its own colour, rose through gold to white.

use std::collections::BTreeMap;

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_picture::bold::floor;
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, Status, StatusKind};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::ropes::RopeEv;
use bf_picture::scene::{A, R_BALL as R, to_world};

use super::{BIRCH, FLY_Y, MarimbaLook, ROSEWOOD, Row, how};
use bf_core::roster::FighterCfg;

/// A head's colour by its bounces: rose, orange, gold, white.
const STEPS: [V3; 4] = [
    [236.0, 106.0, 168.0],
    [255.0, 150.0, 58.0],
    [255.0, 214.0, 96.0],
    [255.0, 250.0, 236.0],
];
/// DEAD STROKE's head: dark and matte.
const MATTE: V3 = [56.0, 50.0, 62.0];
/// DEAD STROKE's flat ring.
const GREY: V3 = [158.0, 156.0, 166.0];
/// Seconds a bar rings on the rope after a bounce.
const BAR_S: f64 = 0.55;
/// Seconds the flat ring takes to shrink onto a pinned enemy.
const PRESS_S: f64 = 0.3;
/// Seconds TREMOLO's second star comes after the first.
const DOUBLE_S: f64 = 0.07;
/// Seconds FIVE OCTAVES' keyboard shows before the first mallet, and after the last.
const KEYS_IN_S: f64 = 0.25;
const KEYS_OUT_S: f64 = 0.3;
/// The bars of FIVE OCTAVES' keyboard.
const KEYS: f64 = 30.0;

/// How a mallet is drawn.
#[derive(Clone, Copy)]
enum Style {
    /// MALLET RUN's: big and tumbling (the angle of its shaft).
    Run(f64),
    /// The same with DEAD STROKE's matte head.
    Dead(f64),
    /// TREMOLO's: two heads circling each other (the angle of the pair).
    Pair(f64),
    /// One of FIVE OCTAVES' 61.
    Small,
}

/// One painter item of the bold look.
enum BItem {
    /// A mallet in flight: `x`, `z`, its unit heading, its head's colour.
    Mallet(f64, f64, (f64, f64), V3, Style),
    /// A bar ringing where a mallet banked off a rope: `x`, `z`, whether that rope runs along z, its half-length,
    /// its age in seconds and the colour it was struck in.
    Bar(f64, f64, bool, f64, f64, V3),
    /// FIVE OCTAVES' keyboard on the canvas round `(x, z)`: the bearing its first bar is on, how far through the
    /// fan it is (below 0 before it starts, above 1 after), and how far out its bars have grown (0 to 1).
    Keys(f64, f64, f64, f64, f64),
}

/// A head's colour after `b` bounces.
fn step(b: i64) -> V3 {
    STEPS[b.clamp(0, 3) as usize]
}

/// The colour of the `u`-th part of FIVE OCTAVES' sweep (0 the first bar, 1 the last): rose through orange and
/// gold to white.
fn sweep(u: f64) -> V3 {
    let x = u.clamp(0.0, 1.0) * 3.0;
    let i = (x.floor() as usize).min(2);
    let f = x - i as f64;
    std::array::from_fn(|c| STEPS[i][c] + (STEPS[i + 1][c] - STEPS[i][c]) * f)
}

/// A round head with the dark outline and no highlight: matte.
fn matte_head(fr: &mut Frame, cam: &Cam, w: V3, r: f64, col: V3) {
    let pc = cam.to_cam(&[w])[0];
    if pc[2] <= cam.near {
        return;
    }
    let s = cam.proj(&[pc])[0];
    let rp = (r * cam.f / pc[2]).max(2.5);
    fr.disc(s, rp + 1.2, bold::INK, 14, 0.0, Some(pc[2]));
    fr.disc(s, rp, col, 14, 0.0, Some(pc[2] - 0.01));
}

/// One mallet: the shaft from the head at `(x, z)` out to `len` along `(dx, dz)`, then the head.
#[allow(clippy::too_many_arguments)]
fn mallet(fr: &mut Frame, cam: &Cam, x: f64, z: f64, dir: (f64, f64), len: f64, r: f64, head: V3, matte: bool) {
    let at = to_world(x, z, FLY_Y);
    let tail = to_world(x + dir.0 * len, z + dir.1 * len, FLY_Y + 0.05);
    bold::thick_line(fr, cam, &[tail, at], BIRCH, 3);
    if matte {
        matte_head(fr, cam, at, r, head);
    } else {
        bold::blob(fr, cam, at, r, head);
    }
}

/// The roster numbers the bold moves read, taken in `setup_as` from its `cfg`: for its own side that is
/// `p.cfg[k]`, and EUPHONIUM's copy of this look gets the copied fighter's numbers.
#[derive(Clone, Debug, Default)]
pub(super) struct Nums {
    /// FIVE OCTAVES' beats.
    fan_beats: f64,
}

impl Nums {
    /// The numbers of the roster entry `cfg`.
    pub(super) fn of(cfg: &FighterCfg) -> Nums {
        Nums {
            fan_beats: [&cfg.unlock1, &cfg.unlock2, &cfg.low]
                .iter()
                .find(|ab| ab.params.contains_key("n") && ab.params.contains_key("beats"))
                .and_then(|ab| ab.params.get("beats"))
                .copied()
                .unwrap_or(2.0),
        }
    }
}

impl MarimbaLook {
    /// Its mallets showing at `t` with their ids, from the recorded `MALLETS`.
    fn live_ids(&self, p: &PictureCtx, t: f64) -> Vec<(i64, Row)> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) || !p.run.has("MALLETS") {
            return Vec::new();
        }
        let i = p.sim_i(t.min(p.t_ko));
        let Some(Value::List(rows)) = p.run.nested("MALLETS", i) else {
            return Vec::new();
        };
        rows.iter()
            .filter_map(|v| {
                let id = match v {
                    Value::List(r) => match r.get(1) {
                        Some(Value::Int(n)) => *n,
                        _ => -1,
                    },
                    _ => -1,
                };
                Some((id, Row::of(v)?))
            })
            .filter(|(_, r)| r.owner == self.k)
            .collect()
    }

    /// DEAD STROKE is armed at `t` and not yet spent.
    fn armed(&self, t: f64) -> bool {
        Self::in_window(&self.dead, t)
            && !self
                .pins
                .iter()
                .any(|&(tp, _)| self.dead.iter().any(|&(a, b)| a <= tp && tp <= t && t < b))
    }

    /// FIVE OCTAVES: when it started, how long its fan runs, and the id of its first mallet.
    fn fan(&self, p: &PictureCtx) -> Option<(f64, f64, i64)> {
        let t0 = self.five?;
        let beats = self.nums.fan_beats;
        let id0 = self
            .throws
            .iter()
            .find(|th| th.octave && th.t >= t0)
            .map_or(0, |th| th.id);
        Some((t0, (beats * p.show.grid.beat).max(1e-3), id0))
    }

    /// How many mallets FIVE OCTAVES throws.
    fn fan_n(&self) -> f64 {
        (self.throws.iter().filter(|th| th.octave).count() as f64).max(1.0)
    }

    /// Whether the mallet of this hit pinned the enemy (DEAD STROKE's).
    fn pins_with(p: &PictureCtx, e: &Event) -> bool {
        let Some(k) = e.who() else { return false };
        let i0 = p.ev_t.partition_point(|&te| te < e.t - 1e-6);
        let i1 = p.ev_t.partition_point(|&te| te <= e.t + 1e-6);
        p.rec.events[i0..i1.max(i0)]
            .iter()
            .any(|x| x.kind == "pinned" && x.i(3) == k as i64)
    }

    /// Every rope a mallet banks off bows there (and the ring's own spark runs from the spot): harder each note
    /// up the scale, and only a little for FIVE OCTAVES' small ones.
    pub(super) fn bold_rope_events(&self, p: &PictureCtx) -> Vec<RopeEv> {
        let octave: BTreeMap<i64, bool> = self.throws.iter().map(|th| (th.id, th.octave)).collect();
        self.bounces
            .iter()
            .filter(|b| b.t <= p.t_ko)
            .map(|b| {
                let side = match (b.x.abs() >= b.z.abs(), b.x >= 0.0, b.z >= 0.0) {
                    (true, true, _) => 1,
                    (true, false, _) => 3,
                    (false, _, true) => 0,
                    (false, _, false) => 2,
                };
                let small = octave.get(&b.id).copied().unwrap_or(false);
                RopeEv {
                    t: b.t,
                    side,
                    x: b.x,
                    z: b.z,
                    spd: if small { 0.7 } else { 2.0 + 0.5 * b.b as f64 },
                }
            })
            .collect()
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let o = 1 - k;
        // a bounce rings out from the rope in the head's new colour
        for b in self.bounces.iter().filter(|b| b.t <= c.t && c.t < b.t + 0.28) {
            let u = (c.t - b.t) / 0.28;
            floor::ring(
                ic,
                c,
                b.x,
                b.z,
                0.12 + 0.75 * bold::ease_out(u),
                0.07,
                bold::light(step(b.b)),
                1.0 - u,
            );
        }
        // S3: DEAD STROKE's mallet is about to land: a landing mark closes on the enemy
        if let Some((u, e)) = bold::until(p, o, &["pinned"], c.t, 0.3)
            && e.i(3) == k as i64
        {
            let at = c.fs[o].p;
            floor::target(ic, c, at[0], at[1], R * 1.15, u, bold::light(GREY));
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight {
            return Vec::new();
        }
        let mut out: Vec<(f64, Payload)> = Vec::new();
        let depth_at = |x: f64, z: f64| f.cam.depth(&[to_world(x, z, FLY_Y)])[0];
        // FIVE OCTAVES: the keyboard curves round the marimba (under everything else)
        let (k, o) = (self.k, 1 - self.k);
        if let Some((t0, dur, _)) = self.fan(p)
            && f.t >= t0 - KEYS_IN_S
            && f.t < t0 + dur + KEYS_OUT_S
            && !f.fs[k].shattered
        {
            let i0 = p.sim_i(t0);
            let (a0, b0) = (p.run.p(i0, k), p.run.p(i0, o));
            let aim = dmath::atan2(b0[1] - a0[1], b0[0] - a0[0]);
            let grown = ((f.t - (t0 - KEYS_IN_S)) / KEYS_IN_S)
                .min((t0 + dur + KEYS_OUT_S - f.t) / KEYS_OUT_S)
                .clamp(0.0, 1.0);
            let me = f.fs[k].p;
            out.push((
                depth_at(me[0], me[1]) + 4.0,
                Box::new(BItem::Keys(me[0], me[1], aim, (f.t - t0) / dur, bold::ease_out(grown))),
            ));
        }
        let trem = Self::in_window(&self.tremolo, f.t);
        let armed = self.armed(f.t);
        let fan = self.fan(p);
        let n = self.fan_n();
        for (id, m) in self.live_ids(p, f.t) {
            let sp = dmath::hypot(m.vx, m.vz).max(1e-9);
            let dir = (m.vx / sp, m.vz / sp);
            let turn = f.t * 15.0 + id as f64 * 1.7;
            let (col, style) = if m.octave {
                let j = fan.map_or(0.0, |(_, _, id0)| (id - id0) as f64);
                (sweep(j / n), Style::Small)
            } else if armed {
                (MATTE, Style::Dead(turn))
            } else if trem {
                (step(m.b), Style::Pair(f.t * 24.0 + id as f64))
            } else {
                (step(m.b), Style::Run(turn))
            };
            out.push((
                depth_at(m.x, m.z) - 0.02,
                Box::new(BItem::Mallet(m.x, m.z, dir, col, style)),
            ));
        }
        // the bars that are still ringing where MALLET RUN's mallets banked off the ropes
        let octave: BTreeMap<i64, bool> = self.throws.iter().map(|th| (th.id, th.octave)).collect();
        for b in self.bounces.iter().filter(|b| b.t <= f.t && f.t < b.t + BAR_S) {
            if octave.get(&b.id).copied().unwrap_or(false) {
                continue;
            }
            let half = 0.5 - 0.085 * (b.b - 1).clamp(0, 4) as f64;
            out.push((
                depth_at(b.x, b.z),
                Box::new(BItem::Bar(b.x, b.z, b.x.abs() >= b.z.abs(), half, f.t - b.t, step(b.b))),
            ));
        }
        out
    }

    pub(super) fn bold_draw_item(&self, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        match payload.downcast_ref::<BItem>() {
            Some(&BItem::Mallet(x, z, dir, col, style)) => match style {
                Style::Run(turn) | Style::Dead(turn) => {
                    let spin = (dmath::cos(turn), dmath::sin(turn));
                    mallet(fr, f.cam, x, z, spin, 0.55, 0.17, col, matches!(style, Style::Dead(_)));
                }
                Style::Pair(turn) => {
                    // two heads circling each other, their shafts trailing
                    let (c, s) = (dmath::cos(turn), dmath::sin(turn));
                    let (ox, oz) = ((-dir.1 * c + dir.0 * s) * 0.2, (dir.0 * c + dir.1 * s) * 0.2);
                    let back = (-dir.0, -dir.1);
                    mallet(fr, f.cam, x - ox, z - oz, back, 0.42, 0.14, col, false);
                    mallet(fr, f.cam, x + ox, z + oz, back, 0.42, 0.14, bold::pale(col), false);
                }
                Style::Small => mallet(fr, f.cam, x, z, (-dir.0, -dir.1), 0.24, 0.1, col, false),
            },
            Some(&BItem::Bar(x, z, along_z, half, age, col)) => {
                // a rosewood bar hanging at the rope: it pops out, shivers as it rings and shrinks away
                let grow = bold::ease_out(age / 0.05) * (1.0 - bold::ease_in((age - (BAR_S - 0.15)) / 0.15));
                let shiver = 0.06 * dmath::sin(age * 55.0) * dmath::exp(-age * 9.0);
                let (half, tall) = (half * grow, 0.2 * grow);
                let at = |along: f64, up: f64| {
                    let (bx, bz) = if along_z { (x, z + along) } else { (x + along, z) };
                    to_world(bx, bz, FLY_Y + 0.1 + up + shiver)
                };
                let body = if age < 0.09 { bold::pale(col) } else { ROSEWOOD };
                bold::outlined_poly(
                    fr,
                    f.cam,
                    &[at(-half, -tall), at(half, -tall), at(half, tall), at(-half, tall)],
                    body,
                );
                fr.line(
                    f.cam,
                    &[at(-half * 0.8, tall * 0.45), at(half * 0.8, tall * 0.45)],
                    col,
                    1,
                );
            }
            Some(&BItem::Keys(x, z, aim, through, grown)) => {
                // long bars for the low notes, short ones for the high; each lights up as its mallets leave
                let r0 = R * 1.4;
                for i in 0..KEYS as usize {
                    let u = i as f64 / KEYS;
                    let (a0, a1) = (
                        aim + (i as f64 + 0.14) / KEYS * dmath::TAU,
                        aim + (i as f64 + 0.86) / KEYS * dmath::TAU,
                    );
                    let r1 = r0 + (0.8 - 0.42 * u) * grown;
                    let corner = |r: f64, a: f64| (x + r * dmath::cos(a), z + r * dmath::sin(a));
                    let quad = [corner(r0, a0), corner(r1, a0), corner(r1, a1), corner(r0, a1)];
                    if quad.iter().any(|q| q.0.abs() > A || q.1.abs() > A) {
                        continue;
                    }
                    let since = through - (i as f64 + 0.5) / KEYS;
                    let col = if since < 0.0 {
                        ROSEWOOD
                    } else if since < 0.1 {
                        bold::WHITE
                    } else {
                        sweep(u)
                    };
                    let pts: Vec<V3> = quad.iter().map(|q| to_world(q.0, q.1, 0.02)).collect();
                    bold::outlined_poly(fr, f.cam, &pts, col);
                }
            }
            None => {}
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let k = self.k;
        let o = 1 - k;
        if !f.in_fight || f.t > p.t_ko + 0.4 {
            return;
        }
        let i0 = p.ev_t.partition_point(|&te| te < f.t - 0.4);
        let i1 = p.ev_t.partition_point(|&te| te <= f.t);
        for e in p.rec.events[i0..i1.max(i0)]
            .iter()
            .filter(|e| e.kind == "hit" && e.who() == Some(k))
        {
            let age = f.t - e.t;
            let c = bold::at(f.cam, e.f(5), e.f(6), R);
            match how(e) {
                // the more bounces, the bigger the ring round the hit, in the head's colour
                "mallet" if !Self::pins_with(p, e) => {
                    let b = e.i(8).clamp(0, 4) as f64;
                    let u = age / (0.2 + 0.04 * b);
                    if u < 1.0 {
                        let r = (9.0 + 5.0 * b) * (0.7 + 1.1 * bold::ease_out(u));
                        px::ink_ring(img, c, r, 1.5 + 0.5 * b, step(e.i(8)), 1.0 - u * u);
                    }
                }
                // TREMOLO: the second of the pair lands a moment later, beside the first
                "tremolo" => {
                    let u = (age - DOUBLE_S) / 0.2;
                    if (0.0..1.0).contains(&u) {
                        let r = 5.0 + 6.0 * bold::ease_out(u * 2.0);
                        let q = (c.0 + 10.0, c.1 - 8.0);
                        px::ink_star(
                            img,
                            q,
                            r,
                            r * 0.38,
                            4,
                            0.5,
                            bold::pale(step(e.i(8))),
                            (1.0 - u).min(0.5) * 2.0,
                        );
                        px::ink_ring(img, q, r * 1.5, 1.5, bold::pale(p.col[k]), 1.0 - u);
                    }
                }
                _ => {}
            }
        }
        // DEAD STROKE has landed: the dark mallet stays pressed on the enemy and a flat grey ring shrinks inward
        let Some(d) = drawn.iter().find(|d| d.k == o).filter(|_| !f.fs[o].shattered) else {
            return;
        };
        for &(t0, secs) in &self.pins {
            let age = f.t - t0;
            if age < 0.0 || age >= secs.max(PRESS_S) {
                continue;
            }
            // which side of the enemy it came in on
            let them = p.run.p(p.sim_i(t0), o);
            let from = self
                .ends
                .iter()
                .find(|e| e.hit && (e.t - t0).abs() < 1e-6)
                .map_or((them[0] - 1.0, them[1]), |e| (e.x, e.z));
            let (a, b) = (bold::at(f.cam, them[0], them[1], R), bold::at(f.cam, from.0, from.1, R));
            let n = dmath::hypot(b.0 - a.0, b.1 - a.1);
            let (ux, uy) = if n > 0.5 {
                ((b.0 - a.0) / n, (b.1 - a.1) / n)
            } else {
                (-0.8, -0.6)
            };
            let head = (d.s.0 + ux * (d.rp + 5.0), d.s.1 + uy * (d.rp + 5.0));
            let tail = (head.0 + ux * 19.0, head.1 + uy * 19.0);
            px::ink_line(img, head, tail, 3.0, BIRCH, 1.0);
            px::disc(img, head, 9.0, GREY, 1.0);
            px::ink_disc(img, head, 6.5, MATTE, 1.0);
            let u = age / PRESS_S;
            if u < 1.0 {
                let r = d.rp * (2.4 - 1.3 * bold::ease_out(u));
                px::ink_ring(img, d.s, r, 2.5, GREY, 1.0);
            }
        }
    }

    /// A landed DEAD STROKE pins the enemy.
    pub(super) fn bold_statuses(&self, t: f64) -> Vec<Status> {
        self.pins
            .iter()
            .filter(|&&(t0, secs)| t0 <= t && t < t0 + secs)
            .map(|&(t0, secs)| Status {
                who: 1 - self.k,
                kind: StatusKind::Pin,
                left: Some(1.0 - (t - t0) / secs.max(1e-6)),
            })
            .collect()
    }

    /// A mallet's hit grows with its bounces; TREMOLO's second hit has its own star; DEAD STROKE lands with no
    /// burst at all.
    pub(super) fn bold_hit(p: &PictureCtx, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match how(e) {
            "tremolo" => None,
            "mallet" | "octave" if Self::pins_with(p, e) => None,
            "mallet" => Some(match e.i(8) {
                i64::MIN..=1 => by_damage,
                2 => by_damage.max(HitSize::Solid),
                _ => HitSize::Heavy,
            }),
            _ => Some(by_damage),
        }
    }
}
