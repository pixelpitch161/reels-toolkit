//! Weapon kind `plunger`, CORNET's PLUNGER MUTE, and its abilities DERBY HAT (`derby`) and STOP-TIME (`stoptime`).
//! CORNET is a fighter made from scratch in the Studio (2026-11-16): there is no Python kit reference, this module is
//! the source of truth (`docs/superpowers/specs/2026-11-16-cornet-fighter-design.md`).
//!
//! **PLUNGER MUTE.** Every `every` beats (from 16th `off16`), when no cup is out and the fighter is not stunned, a cup
//! on a cord flies at where the enemy will be in `lead` seconds, at `speed`. Within the cord's reach (`blade_len`) a
//! cup that meets the enemy sticks: `dmg_stick` (kind `plunger`), the reach grows by `grow` (to `max_reach`) and the
//! hold by `hold_grow` beats (to `hold_max`). Every beat while stuck it "wahs": `dmg` (kind `wah`) and a pull of `pull`
//! toward the fighter. After the hold's beats, or when the cord is stretched past `snap` times the reach, it pops off
//! and reels back. A stick the enemy cannot take (a shield, or it is out of reach of hits) reels straight back.
//! Events `plunger` (thrown: aim x, y), `plunger_stick`, `wah` (its count), `plunger_pop` (0 hold done, 1 stretched,
//! 2 blocked), `plunger_miss`.
//!
//! **DERBY HAT** (`derby`): a hat lobbed at where the enemy is heading lands `tele` beats later; an enemy within
//! `radius` (plus a ball radius) takes `dmg` (kind `derby`) and is muffled for the ability's duration
//! (`Ball::muffle`: it moves but starts no attack). Events `derby` (target x, y, landing time), `derby_land` (x, y, 1
//! if it landed on the enemy), `muffle` (the victim, seconds).
//!
//! **STOP-TIME** (`stoptime`): `n` beats from its trigger. On each beat but the last the enemy freezes for `freeze` of
//! a beat (`Ball::stun`) and the fighter dashes at it (a knock of its own ball toward the enemy, `boost`); a dash that
//! meets the enemy within its beat hits once (`dmg`, `knock`, kind `stoptime`). On the last beat a burst ring of kind
//! `break` (`speed`, `radius`, `dmg_break`, `knock_break`). Events `stop` (beat index), `break`.
//!
//! The record key `PLG` is (frames, 2, 7) floats per ball: the cup's x, y, state (0 none, 1 out, 2 stuck, 3 back), the
//! cord's reach, and the ball's newest hat in the air: x, y, height (NaN without one). None of this code draws from
//! the fight's generator.

use bf_core::event::Value;

use crate::abilities::{ab_dur, param};
use crate::fight::{Fight, Wave, radius, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, PlungerCfg, WeaponCfg};

/// NaN, for an empty slot of the record.
const NAN: f64 = f64::NAN;

/// Where a cup is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CupPhase {
    /// Flying out.
    Out,
    /// Stuck on the enemy.
    Stuck,
    /// Reeling back.
    Back,
}

impl CupPhase {
    /// The record's code.
    pub fn code(self) -> f64 {
        match self {
            CupPhase::Out => 1.0,
            CupPhase::Stuck => 2.0,
            CupPhase::Back => 3.0,
        }
    }
}

/// A cup on its cord.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cup {
    /// Its phase.
    pub phase: CupPhase,
    /// Where it is.
    pub p: V2,
    /// Its heading out (a unit vector).
    pub dir: V2,
    /// The 16th it stuck on (stuck only).
    pub k16: i64,
    /// Wahs so far.
    pub wahs: i64,
}

/// A derby hat in the air or landed.
#[derive(Clone, Debug, PartialEq)]
pub struct Hat {
    /// The fighter that threw it.
    pub owner: usize,
    /// Where it was thrown from.
    pub from: V2,
    /// Where it lands.
    pub to: V2,
    /// When it was thrown.
    pub t0: f64,
    /// When it lands.
    pub t1: f64,
    /// It lands on an enemy within this (plus a ball radius).
    pub radius: f64,
    /// Its damage.
    pub dmg: f64,
    /// Seconds it muffles.
    pub dur: f64,
    /// Whether it has landed.
    pub done: bool,
}

/// A STOP-TIME under way.
#[derive(Clone, Debug, PartialEq)]
pub struct Stop {
    /// The fighter playing it.
    pub owner: usize,
    /// Its first beat.
    pub t0: f64,
    /// The next beat to play, from 0.
    pub next: i64,
    /// Whether this beat's dash has hit.
    pub hit: bool,
    /// The ability's numbers.
    pub n: i64,
    /// Seconds of freeze a beat.
    pub freeze: f64,
    /// The dash's speed boost.
    pub boost: f64,
    /// A dash hit's damage and knock.
    pub dmg: f64,
    /// A dash hit's knock.
    pub knock: f64,
    /// The break ring's speed.
    pub speed: f64,
    /// The break ring's radius.
    pub radius: f64,
    /// The break ring's damage.
    pub dmg_break: f64,
    /// The break ring's knock.
    pub knock_break: f64,
    /// The ability's name.
    pub name: String,
}

/// Weapon kind `plunger`'s per-fight state (`Fight::plunger`), with DERBY HAT's hats and STOP-TIME's runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// Each fighter's cup.
    pub cup: [Option<Cup>; 2],
    /// Each fighter's hold now, beats.
    pub hold: [f64; 2],
    /// The hats thrown so far (landed ones stay a while for the record).
    pub hats: Vec<Hat>,
    /// The STOP-TIMEs under way.
    pub stops: Vec<Stop>,
}

/// Fighter `k`'s weapon is a plunger.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Plunger(_))
}

/// The plunger's settings.
///
/// # Panics
/// When fighter `k`'s weapon is not a plunger.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a PlungerCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Plunger(w) => w,
        other => panic!("fighter {k} ({}) has no plunger: {other:?}", cfg.name),
    }
}

/// A unit vector along `d`, or `fallback` when `d` is (nearly) zero.
fn unit(d: V2, fallback: V2) -> V2 {
    let n = geom::hypot(d);
    if n < 1e-9 { fallback } else { geom::div(d, n) }
}

/// The plunger's setup: the cord at its base reach, no cup out, the hold at its start.
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    f.balls[k].blade_len = w.base;
    f.plunger.cup[k] = None;
    f.plunger.hold[k] = w.hold;
}

/// The plunger's 16th grid: a throw on the grid when no cup is out, and a wah each beat a cup is stuck.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    let o = 1 - k;
    if let Some(cup) = f.plunger.cup[k] {
        if cup.phase == CupPhase::Stuck && k16 > cup.k16 && (k16 - cup.k16) % 4 == 0 {
            let wahs = cup.wahs + 1;
            let at = cup.p;
            f.ev("wah", k as i64, vec![Value::Int(wahs)]);
            if f.damage(k, o, w.dmg, "wah", at, wahs) {
                let toward = geom::sub(f.balls[k].p, f.balls[o].p);
                f.knock(o, toward, w.pull);
            }
            let done = wahs as f64 >= f.plunger.hold[k] - 1e-9;
            f.plunger.cup[k] = Some(Cup {
                wahs,
                phase: if done { CupPhase::Back } else { CupPhase::Stuck },
                ..cup
            });
            if done {
                f.ev("plunger_pop", k as i64, vec![Value::Int(0)]);
            }
        }
        return;
    }
    if stunned(f, k) {
        return;
    }
    let main16 = bf_core::py::round_half_even(4.0 * w.every) as i64;
    let off = w.off16 as i64;
    if k16 < off || bf_core::py::imod_py(k16 - off, main16) != 0 {
        return;
    }
    let (b, e) = (&f.balls[k], &f.balls[o]);
    let aim = geom::add(e.p, geom::scale(e.v, w.lead));
    let dir = unit(geom::sub(aim, b.p), [1.0, 0.0]);
    let p = geom::add(b.p, geom::scale(dir, radius(f, k)));
    f.plunger.cup[k] = Some(Cup {
        phase: CupPhase::Out,
        p,
        dir,
        k16: 0,
        wahs: 0,
    });
    f.ev("plunger", k as i64, vec![Value::Float(aim[0]), Value::Float(aim[1])]);
}

/// The cup moves on by one step: out until it meets the enemy or runs out of cord, stuck on the enemy's front,
/// or reeling back to the fighter.
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let Some(mut cup) = f.plunger.cup[k] else {
        return;
    };
    let w = weapon(f, k);
    let o = 1 - k;
    let (bp, op) = (f.balls[k].p, f.balls[o].p);
    let reach = f.balls[k].blade_len;
    match cup.phase {
        CupPhase::Out => {
            cup.p = geom::add(cup.p, geom::scale(cup.dir, w.speed * dt));
            if geom::hypot(geom::sub(cup.p, op)) < radius(f, o) + w.cup_r {
                // the stick: k16 is the current 16th, for the wahs' count
                let k16 = (((f.t - f.rules.t_fight) / (f.rules.beat / 4.0)) + 1e-6).floor() as i64;
                cup.k16 = k16;
                if f.damage(k, o, w.dmg_stick, "plunger", cup.p, 0) {
                    cup.phase = CupPhase::Stuck;
                    let b = &mut f.balls[k];
                    b.blade_len = geom::py_min(b.blade_len + w.grow, w.max_reach);
                    f.plunger.hold[k] = geom::py_min(f.plunger.hold[k] + w.hold_grow, w.hold_max);
                    f.ev("plunger_stick", k as i64, vec![Value::Float(cup.p[0]), Value::Float(cup.p[1])]);
                } else {
                    cup.phase = CupPhase::Back;
                    f.ev("plunger_pop", k as i64, vec![Value::Int(2)]);
                }
            } else if geom::hypot(geom::sub(cup.p, bp)) > reach + radius(f, k) {
                cup.phase = CupPhase::Back;
                f.ev("plunger_miss", k as i64, Vec::new());
            }
        }
        CupPhase::Stuck => {
            let face = unit(geom::sub(bp, op), geom::scale(cup.dir, -1.0));
            cup.p = geom::add(op, geom::scale(face, radius(f, o)));
            if geom::hypot(geom::sub(op, bp)) > w.snap * (reach + radius(f, k) + radius(f, o)) {
                cup.phase = CupPhase::Back;
                f.ev("plunger_pop", k as i64, vec![Value::Int(1)]);
            }
        }
        CupPhase::Back => {
            let d = geom::sub(bp, cup.p);
            let n = geom::hypot(d);
            let s = 1.5 * w.speed * dt;
            if n <= radius(f, k) + s {
                f.plunger.cup[k] = None;
                return;
            }
            cup.p = geom::add(cup.p, geom::scale(d, s / n));
        }
    }
    f.plunger.cup[k] = Some(cup);
}

/// DERBY HAT's trigger: a hat thrown at where the enemy is heading (60 % of where it would be in `tele` beats, kept
/// inside the ropes), landing `tele` beats later.
pub fn start_derby(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let o = &f.balls[1 - k];
    let lead = param(ab, "tele") * f.rules.beat;
    let a = f.rules.a - f.rules.r;
    let tgt = [
        geom::np_clip(o.p[0] + o.v[0] * lead * 0.6, -a, a),
        geom::np_clip(o.p[1] + o.v[1] * lead * 0.6, -a, a),
    ];
    let hat = Hat {
        owner: k,
        from: f.balls[k].p,
        to: tgt,
        t0: f.t,
        t1: f.t + lead,
        radius: param(ab, "radius"),
        dmg: param(ab, "dmg"),
        dur: ab_dur(f, ab),
        done: false,
    };
    f.ev(
        "derby",
        k as i64,
        vec![Value::Float(tgt[0]), Value::Float(tgt[1]), Value::Float(hat.t1)],
    );
    f.plunger.hats.push(hat);
}

/// STOP-TIME's trigger: `n` beats from now.
pub fn start_stoptime(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let stop = Stop {
        owner: k,
        t0: f.t,
        next: 0,
        hit: false,
        n: param(ab, "n") as i64,
        freeze: param(ab, "freeze"),
        boost: param(ab, "boost"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        speed: param(ab, "speed"),
        radius: param(ab, "radius"),
        dmg_break: param(ab, "dmg_break"),
        knock_break: param(ab, "knock_break"),
        name: ab.name.clone(),
    };
    f.plunger.stops.push(stop);
}

/// Whether a hat or a STOP-TIME is under way (so `abilities_step` has work).
pub fn abilities_live(f: &Fight) -> bool {
    !f.plunger.hats.is_empty() || !f.plunger.stops.is_empty()
}

/// The hats land and the STOP-TIMEs play their beats, for either ball.
pub fn abilities_step(f: &mut Fight) {
    // the hats
    for i in 0..f.plunger.hats.len() {
        let h = &f.plunger.hats[i];
        if h.done || f.t < h.t1 {
            continue;
        }
        let (owner, to, rad, dmg, dur) = (h.owner, h.to, h.radius, h.dmg, h.dur);
        f.plunger.hats[i].done = true;
        let o = 1 - owner;
        let op = f.balls[o].p;
        let inside = geom::hypot(geom::sub(op, to)) < rad + radius(f, o);
        f.ev(
            "derby_land",
            owner as i64,
            vec![Value::Float(to[0]), Value::Float(to[1]), Value::Int(i64::from(inside))],
        );
        if inside && f.damage(owner, o, dmg, "derby", op, 0) {
            let b = &mut f.balls[o];
            b.muffle = geom::py_max(b.muffle, dur);
            f.ev("muffle", o as i64, vec![Value::Int(owner as i64), Value::Float(dur)]);
        }
    }
    let t = f.t;
    f.plunger.hats.retain(|h| !h.done || t < h.t1 + h.dur + 1.0);

    // the STOP-TIMEs
    let beat = f.rules.beat;
    for i in 0..f.plunger.stops.len() {
        let s = f.plunger.stops[i].clone();
        let (k, o) = (s.owner, 1 - s.owner);
        if s.next < s.n && f.t + 1e-9 >= s.t0 + s.next as f64 * beat {
            if s.next < s.n - 1 {
                let ob = &mut f.balls[o];
                ob.stun = geom::py_max(ob.stun, s.freeze * beat);
                let toward = geom::sub(f.balls[o].p, f.balls[k].p);
                f.knock(k, toward, s.boost);
                f.ev("stop", k as i64, vec![Value::Int(s.next)]);
            } else {
                f.waves.push(Wave {
                    owner: k,
                    c: f.balls[k].p,
                    r: f.rules.r,
                    speed: s.speed,
                    rmax: s.radius,
                    dmg: s.dmg_break,
                    knock: s.knock_break,
                    kind: "break".to_string(),
                    hit: false,
                    name: s.name.clone(),
                    t0: f.t,
                    slow: None,
                    stun: None,
                    detune: None,
                    grow: false,
                    clears: true,
                });
                f.ev("break", k as i64, Vec::new());
            }
            let st = &mut f.plunger.stops[i];
            st.next += 1;
            st.hit = false;
            continue;
        }
        // a dash meets the enemy
        if !s.hit && s.next >= 1 && s.next < s.n {
            let (bp, op) = (f.balls[k].p, f.balls[o].p);
            let d = geom::sub(op, bp);
            if geom::hypot(d) < radius(f, k) + radius(f, o) + 0.15 {
                f.plunger.stops[i].hit = true;
                let at = geom::add(bp, geom::scale(d, 0.5));
                if f.damage(k, o, s.dmg, "stoptime", at, s.next) {
                    f.knock(o, d, s.knock);
                }
            }
        }
    }
    f.plunger.stops.retain(|s| s.next < s.n);
}

/// The record keys this kind brings: `PLG` when a fighter has a plunger.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) { &["PLG"] } else { &[] }
}

/// This kind's rows of one frame (`PLG`).
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut row = [NAN; 14];
    for j in 0..2 {
        let o = 7 * j;
        row[o + 2] = 0.0;
        row[o + 3] = f.balls[j].blade_len;
        if let Some(cup) = &f.plunger.cup[j] {
            row[o] = cup.p[0];
            row[o + 1] = cup.p[1];
            row[o + 2] = cup.phase.code();
        }
        if let Some(h) = f.plunger.hats.iter().rev().find(|h| h.owner == j && !h.done) {
            let u = ((f.t - h.t0) / (h.t1 - h.t0).max(1e-9)).clamp(0.0, 1.0);
            row[o + 4] = h.from[0] + (h.to[0] - h.from[0]) * u;
            row[o + 5] = h.from[1] + (h.to[1] - h.from[1]) * u;
            row[o + 6] = 4.0 * 1.6 * u * (1.0 - u);
        }
    }
    out.push_f64("PLG", &[2, 7], &row);
}

