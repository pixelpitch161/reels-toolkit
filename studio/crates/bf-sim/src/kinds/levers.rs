//! Weapon kind `levers`, OCTOBASS's LEVER PUNCH, and its three abilities `foot_pedal` (FOOT PEDAL), `seven_levers`
//! (SEVEN LEVERS) and `below_hearing` (BELOW HEARING). A Studio kind: the Python kit never had it, so this module is
//! the source of truth (spec `docs/superpowers/specs/2026-11-21-octobass-fighter-design.md`).
//!
//! Every `every16` sixteenths (haste divides it) an octobass that is not stunned and not busy with SEVEN LEVERS or
//! BELOW HEARING fires a volley of `floor(n)` lever punches, one every `gap16` sixteenths (`n` starts at `n0`, grows
//! by `grow` per landed punch, up to `n_max`). The arms sit `spread` degrees apart round the line to the enemy. A
//! punch's butt rides the ball's edge; its felt fist shoots straight at the enemy (aim locked when it fires) over
//! `out16` sixteenths up to `reach` ball radii. **A fist that touches the enemy stops there**: it hits once (`dmg`, a
//! knock of `knock` along the punch), holds where it stopped for `hold16` sixteenths, then pulls back over `back16`.
//! A fist that misses holds at full reach and pulls back the same way.
//!
//! - FOOT PEDAL: a pedal plate is set where the enemy is; `tele` beats later it snaps up and an enemy within `radius`
//!   ball radii of it takes `dmg` (a hit of kind `pedal`) and is stunned for `stun_beats` beats.
//! - SEVEN LEVERS: seven punches `gap` seconds apart, their butts fanned over `fan` degrees round the line to the
//!   enemy, each aimed at the enemy, reaching `reach` ball radii and stopping on it for `dmg` (kind `seven`); the
//!   seventh adds a knock of `knock_last`. The volleys pause until the seventh has pulled back.
//! - BELOW HEARING: one ring grows from the ball at `speed` ring units a second (the music drops out meanwhile); when
//!   its edge reaches the enemy, but not before `min_s` seconds (1.2 when left out), or after `max_s` seconds, it hits for `dmg` (kind `below`), knocks `knock` and
//!   stuns for `stun_beats` beats. The volleys pause while it grows.
//!
//! The abilities run for whichever ball fires them (a copied one too).
//!
//! Events: `lever` (owner; arm, 0 = weapon / 1 = SEVEN LEVERS), `pedal_set` (owner; x, y, snap time), `pedal_snap`
//! (owner; 0/1 landed), `seven_levers` (owner; until), `below_start` (owner; started), `below_hit` (owner; 0/1
//! landed). Recorded as `LEVERS`: each arm in flight `(owner, kind 0/1, ext, butt x, butt y, tip x, tip y, held
//! 0/1)`, a set pedal `(owner, 2, left 0..1, x, y, radius, 0, 0)` and a growing ring `(owner, 3, age, x, y, r, 0, 0)`.
//!
//! None of this draws from the generator, so a fight without an octobass is unchanged.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, radius, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, AbilityKind, LeversCfg, WeaponCfg};

/// A fist's radius (it touches within this plus the enemy's radius).
pub const FIST_R: f64 = 0.07;

/// One lever arm in flight.
#[derive(Clone, Debug, PartialEq)]
pub struct Arm {
    /// 0 = a LEVER PUNCH, 1 = one of SEVEN LEVERS.
    pub kind: usize,
    /// When it fired.
    pub t0: f64,
    /// Seconds to reach out, to hold, to pull back.
    pub out: f64,
    pub hold: f64,
    pub back: f64,
    /// The unit direction it punches along (locked when it fires).
    pub dir: V2,
    /// Its butt's angle off the punch direction's bearing, radians (the butt rides the ball).
    pub butt: f64,
    /// Its full reach from the butt.
    pub reach: f64,
    /// Damage and knock when it lands.
    pub dmg: f64,
    pub knock: f64,
    /// When and how far out it stopped on the enemy.
    pub hit: Option<(f64, f64)>,
}

/// A punch waiting to fire (the rest of a volley, or of SEVEN LEVERS).
#[derive(Clone, Debug, PartialEq)]
pub struct Queued {
    pub at: f64,
    pub kind: usize,
    /// The butt's offset off the line to the enemy, radians.
    pub off: f64,
    pub out: f64,
    pub hold: f64,
    pub back: f64,
    pub reach: f64,
    pub dmg: f64,
    pub knock: f64,
}

/// A set FOOT PEDAL.
#[derive(Clone, Debug, PartialEq)]
pub struct Pedal {
    pub p: V2,
    pub t0: f64,
    pub at: f64,
    pub r: f64,
    pub dmg: f64,
    pub stun: f64,
}

/// A growing BELOW HEARING ring.
#[derive(Clone, Debug, PartialEq)]
pub struct Below {
    pub t0: f64,
    pub speed: f64,
    pub min_s: f64,
    pub max_s: f64,
    pub dmg: f64,
    pub knock: f64,
    pub stun: f64,
}

/// The kind's per-fight state (`Fight::levers`), indexed by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// Arms a volley has now (its floor is fired).
    pub n: [f64; 2],
    /// Arms in flight.
    pub arms: [Vec<Arm>; 2],
    /// Punches waiting to fire.
    pub queue: [Vec<Queued>; 2],
    /// SEVEN LEVERS keeps the volleys paused until.
    pub seven_until: [f64; 2],
    pub pedal: [Option<Pedal>; 2],
    pub below: [Option<Below>; 2],
}

/// Fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Levers(_))
}

/// Levers, a pedal or a ring can be in the ring: a fighter is an octobass or has one of its abilities.
pub fn live(f: &Fight) -> bool {
    (0..2).any(|k| {
        is(f, k)
            || crate::abilities::has(f, k, &AbilityKind::FootPedal)
            || crate::abilities::has(f, k, &AbilityKind::SevenLevers)
            || crate::abilities::has(f, k, &AbilityKind::BelowHearing)
    })
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a LeversCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Levers(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not levers", other.kind()),
    }
}

/// Setup: a volley starts with `n0` arms.
pub fn setup(f: &mut Fight, k: usize) {
    f.levers.n[k] = weapon(f, k).n0;
}

fn rot(d: V2, a: f64) -> V2 {
    let (c, s) = (dmath::cos(a), dmath::sin(a));
    [d[0] * c - d[1] * s, d[0] * s + d[1] * c]
}

fn unit(d: V2) -> V2 {
    let n = geom::norm(d);
    if n > 1e-9 { geom::div(d, n) } else { [1.0, 0.0] }
}

/// How far out arm `a` is at time `t` (0..1), whether it is held on the enemy, and whether it is done.
pub fn ext(a: &Arm, t: f64) -> (f64, bool, bool) {
    if let Some((th, eh)) = a.hit {
        if t < th + a.hold {
            return (eh, true, false);
        }
        let e = eh * (1.0 - (t - th - a.hold) / a.back.max(1e-9));
        return (e.max(0.0), false, e <= 0.0);
    }
    let s = (t - a.t0) / a.out.max(1e-9);
    if s < 1.0 {
        return (s.max(0.0), false, false);
    }
    let back = t - a.t0 - a.out - a.hold;
    if back < 0.0 {
        return (1.0, false, false);
    }
    let e = 1.0 - back / a.back.max(1e-9);
    (e.max(0.0), false, e <= 0.0)
}

/// Where arm `a` of ball `k` is now: its butt, its tip, how far out (0..1) and whether it is held on the enemy.
pub fn place(f: &Fight, k: usize, a: &Arm) -> (V2, V2, f64, bool) {
    let (e, held, _) = ext(a, f.t);
    let butt = geom::add(f.balls[k].p, geom::scale(rot(a.dir, a.butt), radius(f, k)));
    let tip = geom::add(butt, geom::scale(a.dir, a.reach * e));
    (butt, tip, e, held)
}

fn busy(f: &Fight, k: usize) -> bool {
    f.levers.seven_until[k] > f.t || f.levers.below[k].is_some()
}

/// The grid tick: on its volley grid an octobass that is not stunned and not busy queues a volley.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    if stunned(f, k) || busy(f, k) {
        return;
    }
    let w = weapon(f, k).clone();
    let main16 = py::round_half_even(w.every16) as i64;
    let ev16 = (py::round_half_even(main16 as f64 / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(k16, ev16) != 0 {
        return;
    }
    let sixteenth = f.rules.beat / 4.0 / f.ab_mult(k);
    let n = (f.levers.n[k].floor() as usize).max(1);
    let r = radius(f, k);
    for i in 0..n {
        // arms side by side round the line: 1 -> [0], 2 -> [-s/2, +s/2], 3 -> [-s, 0, +s]
        let off = (i as f64 - (n as f64 - 1.0) / 2.0) * w.spread.to_radians();
        f.levers.queue[k].push(Queued {
            at: f.t + i as f64 * w.gap16 * sixteenth,
            kind: 0,
            off,
            out: w.out16 * sixteenth,
            hold: w.hold16 * sixteenth,
            back: w.back16 * sixteenth,
            reach: w.reach * r,
            dmg: w.dmg,
            knock: w.knock,
        });
    }
}

fn fire(f: &mut Fight, k: usize, q: &Queued) {
    let r = radius(f, k);
    let (me, them) = (f.balls[k].p, f.balls[1 - k].p);
    let d0 = unit(geom::sub(them, me));
    let bd = rot(d0, q.off);
    let butt = geom::add(me, geom::scale(bd, r));
    let dir = unit(geom::sub(them, butt));
    let butt_a = dmath::atan2(bd[1], bd[0]) - dmath::atan2(dir[1], dir[0]);
    let n = f.levers.arms[k].len() as i64;
    f.levers.arms[k].push(Arm {
        kind: q.kind,
        t0: f.t,
        out: q.out,
        hold: q.hold,
        back: q.back,
        dir,
        butt: butt_a,
        reach: q.reach,
        dmg: q.dmg,
        knock: q.knock,
        hit: None,
    });
    f.ev("lever", k as i64, vec![Value::Int(n), Value::Int(q.kind as i64)]);
}

/// `foot_pedal`: the plate is set where the enemy is now.
pub fn start_foot_pedal(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let p = f.balls[1 - k].p;
    let at = f.t + param(ab, "tele") * f.rules.beat;
    f.levers.pedal[k] = Some(Pedal {
        p,
        t0: f.t,
        at,
        r: param(ab, "radius") * f.rules.r,
        dmg: param(ab, "dmg"),
        stun: param(ab, "stun_beats") * f.rules.beat,
    });
    f.ev(
        "pedal_set",
        k as i64,
        vec![Value::Float(p[0]), Value::Float(p[1]), Value::Float(at)],
    );
}

/// `seven_levers`: seven punches queued `gap` seconds apart; the volleys pause until the last has pulled back.
pub fn start_seven_levers(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let gap = param(ab, "gap");
    let fan = param(ab, "fan").to_radians();
    let sixteenth = f.rules.beat / 4.0;
    let r = radius(f, k);
    let (out, hold, back) = (sixteenth * 0.75, sixteenth * 0.75, sixteenth * 1.5);
    f.levers.queue[k].retain(|q| q.kind != 0);
    for i in 0..7 {
        let last = i == 6;
        f.levers.queue[k].push(Queued {
            at: f.t + i as f64 * gap,
            kind: 1,
            off: (i as f64 - 3.0) / 3.0 * fan / 2.0,
            out,
            hold: if last { hold * 2.0 } else { hold },
            back,
            reach: param(ab, "reach") * r,
            dmg: param(ab, "dmg"),
            knock: if last { param(ab, "knock_last") } else { 0.05 },
        });
    }
    let until = f.t + 6.0 * gap + out + 2.0 * hold + back;
    f.levers.seven_until[k] = until;
    f.ev("seven_levers", k as i64, vec![Value::Float(until)]);
}

/// `below_hearing`: the ring starts growing.
pub fn start_below_hearing(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    f.levers.below[k] = Some(Below {
        t0: f.t,
        speed: param(ab, "speed"),
        min_s: ab.get("min_s").unwrap_or(1.2),
        max_s: param(ab, "max_s"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        stun: param(ab, "stun_beats") * f.rules.beat,
    });
    f.levers.queue[k].retain(|q| q.kind != 0);
    f.ev("below_start", k as i64, vec![Value::Float(f.t)]);
}

/// The later step: queued punches fire; every arm reaches out, stops on the enemy or pulls back; pedals snap; rings
/// grow and land.
pub fn step(f: &mut Fight) {
    for k in 0..2 {
        let o = 1 - k;
        // queued punches whose time has come (in order)
        let mut i = 0;
        while i < f.levers.queue[k].len() {
            if f.t + 1e-9 >= f.levers.queue[k][i].at {
                let q = f.levers.queue[k].remove(i);
                if q.kind == 1 || !stunned(f, k) {
                    fire(f, k, &q);
                }
            } else {
                i += 1;
            }
        }
        // the arms
        let mut a = 0;
        while a < f.levers.arms[k].len() {
            let arm = f.levers.arms[k][a].clone();
            let (e, _, done) = ext(&arm, f.t);
            if done {
                f.levers.arms[k].remove(a);
                continue;
            }
            let going_out = arm.hit.is_none() && f.t < arm.t0 + arm.out;
            if going_out && f.winner.is_none() {
                let (_, tip, _, _) = place(f, k, &arm);
                let op = f.balls[o].p;
                if geom::hypot(geom::sub(op, tip)) < radius(f, o) + FIST_R {
                    f.levers.arms[k][a].hit = Some((f.t, e));
                    land(f, k, o, &arm, tip);
                }
            }
            a += 1;
        }
        pedal_step(f, k);
        below_step(f, k);
    }
}

/// A punch of ball `k` stops on the enemy `o` at `at`: its hit, and a landed LEVER PUNCH grows the volley.
fn land(f: &mut Fight, k: usize, o: usize, a: &Arm, at: V2) {
    let how = if a.kind == 1 { "seven" } else { "lever" };
    if !f.damage(k, o, a.dmg, how, at, 0) {
        return;
    }
    f.knock(o, a.dir, a.knock);
    if a.kind == 0 && is(f, k) {
        let w = weapon(f, k);
        f.levers.n[k] = py::py_min(w.n_max, f.levers.n[k] + w.grow);
    }
}

fn pedal_step(f: &mut Fight, k: usize) {
    let Some(pd) = f.levers.pedal[k].clone() else {
        return;
    };
    if f.t + 1e-9 < pd.at {
        return;
    }
    f.levers.pedal[k] = None;
    let o = 1 - k;
    let mut landed = false;
    if f.winner.is_none() && geom::hypot(geom::sub(f.balls[o].p, pd.p)) < pd.r + radius(f, o) {
        landed = f.damage(k, o, pd.dmg, "pedal", pd.p, 0);
        if landed {
            let b = &mut f.balls[o];
            b.stun = geom::py_max(b.stun, pd.stun);
        }
    }
    f.ev("pedal_snap", k as i64, vec![Value::Int(i64::from(landed))]);
}

/// The ring's radius now.
pub fn below_r(f: &Fight, b: &Below) -> f64 {
    (f.t - b.t0).max(0.0) * b.speed
}

fn below_step(f: &mut Fight, k: usize) {
    let Some(b) = f.levers.below[k].clone() else {
        return;
    };
    let o = 1 - k;
    let r = below_r(f, &b);
    let d = geom::sub(f.balls[o].p, f.balls[k].p);
    let reached = r + radius(f, o) >= geom::hypot(d);
    let age = f.t - b.t0;
    if (!reached || age < b.min_s) && age < b.max_s {
        return;
    }
    f.levers.below[k] = None;
    let mut landed = false;
    if f.winner.is_none() {
        let at = geom::sub(f.balls[o].p, geom::scale(unit(d), radius(f, o)));
        landed = f.damage(k, o, b.dmg, "below", at, 0);
        if landed {
            f.knock(o, unit(d), b.knock);
            let bo = &mut f.balls[o];
            bo.stun = geom::py_max(bo.stun, b.stun);
        }
    }
    f.ev("below_hit", k as i64, vec![Value::Int(i64::from(landed))]);
}

/// The record keys this kind adds: `LEVERS` when levers, a pedal or a ring can be in the ring.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if live(f) { &["LEVERS"] } else { &[] }
}

fn row(v: [f64; 8]) -> Value {
    Value::List(
        v.iter()
            .enumerate()
            .map(|(i, x)| if i < 2 { Value::Int(*x as i64) } else { Value::Float(*x) })
            .collect(),
    )
}

/// This kind's rows of one frame: every arm in flight, every set pedal, every growing ring.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut rows = Vec::new();
    for k in 0..2 {
        for a in &f.levers.arms[k] {
            let (butt, tip, e, held) = place(f, k, a);
            rows.push(row([
                k as f64,
                a.kind as f64,
                e,
                butt[0],
                butt[1],
                tip[0],
                tip[1],
                f64::from(u8::from(held)),
            ]));
        }
        if let Some(pd) = &f.levers.pedal[k] {
            let left = ((pd.at - f.t) / (pd.at - pd.t0).max(1e-9)).clamp(0.0, 1.0);
            rows.push(row([k as f64, 2.0, left, pd.p[0], pd.p[1], pd.r, 0.0, 0.0]));
        }
        if let Some(b) = &f.levers.below[k] {
            let p = f.balls[k].p;
            rows.push(row([k as f64, 3.0, f.t - b.t0, p[0], p[1], below_r(f, b), 0.0, 0.0]));
        }
    }
    out.push_nested("LEVERS", Value::List(rows));
}
