//! Weapon kind `sticks`, SNARE DRUM's PARADIDDLE, and its three abilities `flam` (FLAM), `rimshot` (RIMSHOT) and
//! `buzz_roll` (BUZZ ROLL). A Studio kind: the Python kit never had it, so this module is the source of truth (spec
//! `docs/superpowers/specs/2026-11-16-snare-drum-fighter-design.md`).
//!
//! Two sticks ride the ball, one each side: a stick's butt sits on the ball's edge `spread` degrees off the line to
//! the enemy. Every `step16` sixteenths (haste divides it) a snare that is not stunned and not buzzing plays the next
//! stroke of the paradiddle sticking R L R R, L R L L: that side's stick jabs at the enemy (its aim locked when the
//! stroke starts), reaching out over `out16` sixteenths to its full reach and pulling back over as long. Its reach is
//! `reach` ball radii from the butt (`base` at the start, `grow` more per landed stroke, up to `max_reach`). While it
//! reaches out, a stick whose tip segment touches the enemy hits once: `dmg` (x `accent` on the first stroke of each
//! group of four) and a knock of `knock` along the jab.
//!
//! - FLAM: for `dur_beats` beats every landed stroke lands again as a hit of kind `flam` worth `mult` of it.
//! - RIMSHOT: for up to `dur_beats` beats the snare is armed; the next stroke to land is a rimshot (a hit of kind
//!   `rimshot`): damage x `dmg_k`, knock x `knock_k`, and the enemy is stunned for `stun_beats` beats. The arming is
//!   spent.
//! - BUZZ ROLL: for `beats` beats the sticks blur into a disc of `radius` ball radii round the ball (the strokes
//!   pause); every `tick16` sixteenths an enemy touching it takes `dmg` (a hit of kind `buzz`) and a knock of `knock`.
//!   It runs for whichever ball fires it (a copied one too).
//!
//! Events: `stick` (owner; stroke number, side 0 = left / 1 = right, accent 0/1), `flam` (owner; until), `rimshot`
//! (owner; 0 when armed, or 1 and the stunned seconds when it lands), `buzz_roll` (owner; until). Recorded as
//! `STICKS`: each jab in flight `(owner, side, ext, butt x, butt y, tip x, tip y)` and each live buzz disc
//! `(owner, 2, radius, x, y, 0, 0)`.
//!
//! None of this draws from the generator, so a fight without a snare is unchanged.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, radius, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, AbilityKind, SticksCfg, WeaponCfg};

/// The paradiddle's sticking, 0 = left, 1 = right: R L R R, L R L L.
pub const STICKING: [usize; 8] = [1, 0, 1, 1, 0, 1, 0, 0];

/// A stick's tip radius (it hits within this plus the enemy's radius).
pub const TIP_R: f64 = 0.05;

/// One jab in flight.
#[derive(Clone, Debug, PartialEq)]
pub struct Jab {
    /// 0 = left stick, 1 = right.
    pub side: usize,
    /// When it started.
    pub t0: f64,
    /// Seconds it takes to reach out (as long again to pull back).
    pub out: f64,
    /// The unit direction it jabs along (locked at its start).
    pub dir: V2,
    /// Its butt's angle off the jab direction's bearing at the start, radians (the butt rides the ball).
    pub butt: f64,
    /// Its full reach from the butt.
    pub reach: f64,
    /// The first stroke of a group of four.
    pub accent: bool,
    /// It has hit (a jab hits once).
    pub hit: bool,
}

/// A BUZZ ROLL in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Buzz {
    /// When it ends.
    pub until: f64,
    /// The disc's radius.
    pub r: f64,
    /// Seconds between two ticks.
    pub every: f64,
    /// The next tick.
    pub next: f64,
    /// Damage a tick.
    pub dmg: f64,
    /// Knockback a tick.
    pub knock: f64,
}

/// The kind's per-fight state (`Fight::sticks`), indexed by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The strokes played so far.
    pub n: [i64; 2],
    /// The reach, in ball radii.
    pub reach: [f64; 2],
    /// The jabs in flight, per side.
    pub jabs: [[Option<Jab>; 2]; 2],
    /// FLAM runs until.
    pub flam_until: [f64; 2],
    /// FLAM's `mult`.
    pub flam_mult: [f64; 2],
    /// RIMSHOT is armed until.
    pub rim_until: [f64; 2],
    /// RIMSHOT's `(dmg_k, knock_k, stun seconds)`.
    pub rim: [[f64; 3]; 2],
    /// BUZZ ROLL, while it runs.
    pub buzz: [Option<Buzz>; 2],
}

/// Fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Sticks(_))
}

/// Sticks or a buzz disc can be in the ring: a fighter is a snare or has BUZZ ROLL.
pub fn live(f: &Fight) -> bool {
    (0..2).any(|k| is(f, k) || crate::abilities::has(f, k, &AbilityKind::BuzzRoll))
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a SticksCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Sticks(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not sticks", other.kind()),
    }
}

/// Setup: the reach starts at `base`.
pub fn setup(f: &mut Fight, k: usize) {
    f.sticks.reach[k] = weapon(f, k).base;
}

fn rot(d: V2, a: f64) -> V2 {
    let (c, s) = (dmath::cos(a), dmath::sin(a));
    [d[0] * c - d[1] * s, d[0] * s + d[1] * c]
}

fn unit(d: V2) -> V2 {
    let n = geom::norm(d);
    if n > 1e-9 { geom::div(d, n) } else { [1.0, 0.0] }
}

/// Where jab `j` of ball `k` is now: its butt, its tip and how far out it is (0..1).
pub fn place(f: &Fight, k: usize, j: &Jab) -> (V2, V2, f64) {
    let s = (f.t - j.t0) / j.out.max(1e-9);
    let ext = if s < 1.0 { s.max(0.0) } else { (2.0 - s).max(0.0) };
    let butt = geom::add(f.balls[k].p, geom::scale(rot(j.dir, j.butt), radius(f, k)));
    let tip = geom::add(butt, geom::scale(j.dir, j.reach * ext));
    (butt, tip, ext)
}

/// The grid tick: on its stroke grid a snare that is not stunned and not buzzing plays the next stroke.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    if stunned(f, k) || f.sticks.buzz[k].is_some() {
        return;
    }
    let w = weapon(f, k).clone();
    let main16 = py::round_half_even(w.step16) as i64;
    let ev16 = (py::round_half_even(main16 as f64 / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(k16, ev16) != 0 {
        return;
    }
    let i = f.sticks.n[k];
    f.sticks.n[k] += 1;
    let side = STICKING[py::imod_py(i, 8) as usize];
    let accent = py::imod_py(i, 4) == 0;
    let r = radius(f, k);
    let (me, them) = (f.balls[k].p, f.balls[1 - k].p);
    let d0 = unit(geom::sub(them, me));
    let off = w.spread.to_radians() * if side == 1 { -1.0 } else { 1.0 };
    let butt = geom::add(me, geom::scale(rot(d0, off), r));
    let dir = unit(geom::sub(them, butt));
    // the butt's angle measured from the jab's own direction, so it rides the ball as the ball moves
    let butt_a = dmath::atan2(rot(d0, off)[1], rot(d0, off)[0]) - dmath::atan2(dir[1], dir[0]);
    let out = w.out16 * f.rules.beat / 4.0 / f.ab_mult(k);
    f.sticks.jabs[k][side] = Some(Jab {
        side,
        t0: f.t,
        out,
        dir,
        butt: butt_a,
        reach: f.sticks.reach[k] * r,
        accent,
        hit: false,
    });
    let data = vec![Value::Int(i), Value::Int(side as i64), Value::Int(i64::from(accent))];
    f.ev("stick", k as i64, data);
}

/// `flam`: FLAM runs for `dur_beats` beats.
pub fn start_flam(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + param(ab, "dur_beats") * f.rules.beat;
    f.sticks.flam_until[k] = until;
    f.sticks.flam_mult[k] = param(ab, "mult");
    f.ev("flam", k as i64, vec![Value::Float(until)]);
}

/// `rimshot`: armed for `dur_beats` beats.
pub fn start_rimshot(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    f.sticks.rim_until[k] = f.t + param(ab, "dur_beats") * f.rules.beat;
    f.sticks.rim[k] = [
        param(ab, "dmg_k"),
        param(ab, "knock_k"),
        param(ab, "stun_beats") * f.rules.beat,
    ];
    f.ev("rimshot", k as i64, vec![Value::Int(0), Value::Float(0.0)]);
}

/// `buzz_roll`: the disc for `beats` beats; its first tick at once.
pub fn start_buzz_roll(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + param(ab, "beats") * f.rules.beat;
    f.sticks.buzz[k] = Some(Buzz {
        until,
        r: param(ab, "radius") * f.rules.r,
        every: (param(ab, "tick16") * f.rules.beat / 4.0).max(1e-3),
        next: f.t,
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
    });
    f.sticks.jabs[k] = [None, None];
    f.ev("buzz_roll", k as i64, vec![Value::Float(until)]);
}

/// The later step: every jab reaches out, hits or pulls back; every buzz disc ticks.
pub fn step(f: &mut Fight) {
    for k in 0..2 {
        for side in 0..2 {
            let Some(j) = f.sticks.jabs[k][side].clone() else {
                continue;
            };
            if f.t >= j.t0 + 2.0 * j.out {
                f.sticks.jabs[k][side] = None;
                continue;
            }
            let (butt, tip, _) = place(f, k, &j);
            let going_out = f.t < j.t0 + j.out;
            let o = 1 - k;
            if going_out && !j.hit && f.winner.is_none() {
                let op = f.balls[o].p;
                if seg_dist(op, butt, tip) < radius(f, o) + TIP_R {
                    if let Some(jj) = f.sticks.jabs[k][side].as_mut() {
                        jj.hit = true;
                    }
                    land(f, k, o, &j, tip);
                }
            }
        }
        buzz_step(f, k);
    }
}

fn seg_dist(p: V2, a: V2, b: V2) -> f64 {
    let ab = geom::sub(b, a);
    let l2 = ab[0] * ab[0] + ab[1] * ab[1];
    let t = if l2 > 1e-12 {
        (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    geom::hypot(geom::sub(p, geom::add(a, geom::scale(ab, t))))
}

/// A stroke of ball `k` reaches the enemy `o` at `at`: its hit (a rimshot when armed), FLAM's second hit, the reach's
/// growth.
fn land(f: &mut Fight, k: usize, o: usize, j: &Jab, at: V2) {
    let w = weapon(f, k).clone();
    let mut dmg = w.dmg * if j.accent { w.accent } else { 1.0 };
    let mut knock = w.knock;
    let rim = f.sticks.rim_until[k] > f.t;
    let how = if rim {
        let [dk, kk, _] = f.sticks.rim[k];
        dmg *= dk;
        knock *= kk;
        "rimshot"
    } else {
        "stick"
    };
    if !f.damage(k, o, dmg, how, at, i64::from(j.accent)) {
        return;
    }
    f.knock(o, j.dir, knock);
    if rim {
        f.sticks.rim_until[k] = 0.0;
        let stun = f.sticks.rim[k][2];
        let b = &mut f.balls[o];
        b.stun = geom::py_max(b.stun, stun);
        f.ev("rimshot", k as i64, vec![Value::Int(1), Value::Float(stun)]);
    }
    if f.sticks.flam_until[k] > f.t {
        let mult = f.sticks.flam_mult[k];
        f.damage(k, o, dmg * mult, "flam", at, i64::from(j.accent));
    }
    f.sticks.reach[k] = py::py_min(w.max_reach, f.sticks.reach[k] + w.grow);
}

fn buzz_step(f: &mut Fight, k: usize) {
    let Some(mut bz) = f.sticks.buzz[k].clone() else {
        return;
    };
    let o = 1 - k;
    while f.t + 1e-9 >= bz.next && bz.next < bz.until {
        bz.next += bz.every;
        if f.winner.is_some() {
            continue;
        }
        let (me, op) = (f.balls[k].p, f.balls[o].p);
        let d = geom::sub(op, me);
        if geom::hypot(d) < bz.r + radius(f, o) {
            let at = geom::add(me, geom::scale(unit(d), bz.r));
            if f.damage(k, o, bz.dmg, "buzz", at, 0) {
                f.knock(o, unit(d), bz.knock);
            }
        }
    }
    f.sticks.buzz[k] = if f.t >= bz.until { None } else { Some(bz) };
}

/// The record keys this kind adds: `STICKS` when sticks or a buzz disc can be in the ring.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if live(f) { &["STICKS"] } else { &[] }
}

/// This kind's rows of one frame: every jab in flight and every live buzz disc.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut rows = Vec::new();
    for k in 0..2 {
        for j in f.sticks.jabs[k].iter().flatten() {
            let (butt, tip, ext) = place(f, k, j);
            rows.push(Value::List(vec![
                Value::Int(k as i64),
                Value::Int(j.side as i64),
                Value::Float(ext),
                Value::Float(butt[0]),
                Value::Float(butt[1]),
                Value::Float(tip[0]),
                Value::Float(tip[1]),
            ]));
        }
        if let Some(bz) = &f.sticks.buzz[k] {
            let p = f.balls[k].p;
            rows.push(Value::List(vec![
                Value::Int(k as i64),
                Value::Int(2),
                Value::Float(bz.r),
                Value::Float(p[0]),
                Value::Float(p[1]),
                Value::Float(0.0),
                Value::Float(0.0),
            ]));
        }
    }
    out.push_nested("STICKS", Value::List(rows));
}
