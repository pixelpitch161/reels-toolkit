//! Weapon kind `mallets`, MARIMBA's MALLET RUN, and its three abilities `tremolo` (TREMOLO), `dead_stroke` (DEAD
//! STROKE) and `five_octaves` (FIVE OCTAVES). A Studio kind: the Python kit never had it, so this module is the
//! source of truth (spec `docs/superpowers/specs/2026-11-16-marimba-fighter-design.md`).
//!
//! On its grid (`every` beats; haste divides it) a marimba that is not stunned throws a mallet at the enemy while
//! fewer than `int(n_vol)` of its own MALLET RUN mallets are in the air. A mallet flies straight at `speed` and
//! bounces off the ropes; every bounce adds `dmg_step` to what it hits for. It passes through its owner, hits the
//! enemy it touches (within `hit_r` + the enemy's radius) and is gone; after `bounces` bounces it fizzles at the next
//! rope. A landed MALLET RUN hit grows the owner's `n_vol` by `grow` (up to `n_max`).
//!
//! - TREMOLO: for `dur_beats` beats each landed hit of the owner's mallets lands again as a hit of kind `tremolo`
//!   worth `mult` of the first.
//! - DEAD STROKE: for `dur_beats` beats the owner is armed; its next mallet to land pins the enemy (the engine's
//!   stun) for `pin_beats` beats, and the arming is spent.
//! - FIVE OCTAVES: `n` mallets of kind `octave` leave the ball one after another over `beats` beats, turning once
//!   round from the enemy's bearing, each at `speed`, bouncing at most `bounces` times (default 1), `dmg` a hit.
//!
//! Events: `mallet` (owner; id, x, y, vx, vy, kind), `mallet_bounce` (owner; id, bounces so far, x, y),
//! `mallet_end` (owner; id, `hit` / `fizzle`, bounces, x, y), `tremolo` (owner; until), `dead_stroke` (owner; until)
//! and `pinned` (the pinned fighter; owner, seconds), `five_octaves` (owner; n). A hit is of kind `mallet` or
//! `octave` (its sub-number the bounces) and, under TREMOLO, `tremolo`. Recorded as `MALLETS`: each mallet in the
//! air `(owner, id, x, y, vx, vy, bounces, kind)` with kind 0 for MALLET RUN, 1 for FIVE OCTAVES.
//!
//! None of this draws from the generator, so a fight without a marimba is unchanged.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, radius, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, MalletsCfg, WeaponCfg};

/// Which move threw a mallet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MalletKind {
    /// MALLET RUN's (the weapon).
    Run,
    /// FIVE OCTAVES'.
    Octave,
}

impl MalletKind {
    /// Its name in events and hits.
    pub fn name(self) -> &'static str {
        match self {
            MalletKind::Run => "mallet",
            MalletKind::Octave => "octave",
        }
    }
}

/// One mallet in the air.
#[derive(Clone, Debug, PartialEq)]
pub struct Mallet {
    /// The fighter that threw it.
    pub owner: usize,
    /// Its number (per fight).
    pub id: i64,
    /// Where it is.
    pub p: V2,
    /// Its velocity.
    pub v: V2,
    /// Wall bounces so far.
    pub b: i64,
    /// Bounces before it fizzles.
    pub b_max: i64,
    /// Damage before any bounce.
    pub dmg: f64,
    /// Damage added per bounce.
    pub dmg_step: f64,
    /// Knockback of its hit.
    pub knock: f64,
    /// Its head's radius.
    pub hit_r: f64,
    /// Which move threw it.
    pub kind: MalletKind,
}

/// A FIVE OCTAVES fan in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Fan {
    /// The fighter throwing it.
    pub owner: usize,
    /// When it started.
    pub t0: f64,
    /// When its last mallet leaves.
    pub t1: f64,
    /// The bearing to the enemy when it started.
    pub aim: f64,
    /// How many mallets.
    pub n: i64,
    /// How many have left.
    pub emitted: i64,
    /// Units a second each flies.
    pub speed: f64,
    /// Damage of each.
    pub dmg: f64,
    /// Knockback of each.
    pub knock: f64,
    /// Bounces before each fizzles.
    pub bounces: i64,
}

/// The kind's per-fight state (`Fight::mallets`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The mallets in the air, in the order they were thrown.
    pub mallets: Vec<Mallet>,
    /// The next mallet's number.
    pub next_id: i64,
    /// TREMOLO runs until (per fighter).
    pub tremolo_until: [f64; 2],
    /// TREMOLO's `mult` (per fighter).
    pub tremolo_mult: [f64; 2],
    /// DEAD STROKE is armed until (per fighter).
    pub dead_until: [f64; 2],
    /// DEAD STROKE's pin, in seconds (per fighter).
    pub dead_pin: [f64; 2],
    /// FIVE OCTAVES fans in progress.
    pub fans: Vec<Fan>,
}

/// Fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Mallets(_))
}

/// Mallets can be in the ring: a fighter is a marimba or has FIVE OCTAVES (a copied move included).
pub fn live(f: &Fight) -> bool {
    (0..2).any(|k| is(f, k) || crate::abilities::has(f, k, &crate::roster::AbilityKind::FiveOctaves))
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a MalletsCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Mallets(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not mallets", other.kind()),
    }
}

/// Setup: the volley starts at `n0`.
pub fn setup(f: &mut Fight, k: usize) {
    f.balls[k].n_vol = weapon(f, k).n0;
}

/// How many of fighter `k`'s MALLET RUN mallets are in the air.
fn in_air(f: &Fight, k: usize) -> i64 {
    f.mallets
        .mallets
        .iter()
        .filter(|m| m.owner == k && m.kind == MalletKind::Run)
        .count() as i64
}

/// A mallet leaves ball `k` along the unit direction `d`.
#[allow(clippy::too_many_arguments)]
fn throw(f: &mut Fight, k: usize, d: V2, speed: f64, hit: [f64; 4], b_max: i64, kind: MalletKind) {
    let [dmg, dmg_step, knock, hit_r] = hit;
    let r = radius(f, k);
    let lim = f.rules.a - hit_r;
    let p0 = geom::add(f.balls[k].p, geom::scale(d, r + hit_r));
    let p = [geom::np_clip(p0[0], -lim, lim), geom::np_clip(p0[1], -lim, lim)];
    let v = geom::scale(d, speed);
    let id = f.mallets.next_id;
    f.mallets.next_id += 1;
    f.mallets.mallets.push(Mallet {
        owner: k,
        id,
        p,
        v,
        b: 0,
        b_max,
        dmg,
        dmg_step,
        knock,
        hit_r,
        kind,
    });
    let data = vec![
        Value::Int(id),
        Value::Float(p[0]),
        Value::Float(p[1]),
        Value::Float(v[0]),
        Value::Float(v[1]),
        Value::Str(kind.name().to_string()),
    ];
    f.ev("mallet", k as i64, data);
}

/// The grid tick: on its grid a marimba that is not stunned throws a mallet at the enemy (leading its motion by a
/// quarter of the flight) while it has fewer than `int(n_vol)` in the air.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    if stunned(f, k) {
        return;
    }
    let w = weapon(f, k).clone();
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    let ev16 = (py::round_half_even(main16 as f64 / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(k16, ev16) != 0 {
        return;
    }
    if in_air(f, k) >= (f.balls[k].n_vol + 1e-9) as i64 {
        return;
    }
    // a bank shot: aim at the enemy (leading its motion by a quarter of the flight) mirrored in one of the ropes, the
    // ropes taken in turn (right, top, left, bottom), so every throw ricochets at least once before it can land
    let (me, them) = (f.balls[k].p, &f.balls[1 - k]);
    let lim = f.rules.a - w.hit_r;
    let rope = py::imod_py(f.mallets.next_id, 4);
    let mut tgt = geom::add(them.p, geom::scale(them.v, 0.25 * 2.0 * lim / w.speed));
    let (ax, sign) = [(0, 1.0), (1, 1.0), (0, -1.0), (1, -1.0)][rope as usize];
    tgt[ax] = 2.0 * lim * sign - tgt[ax];
    let d = geom::sub(tgt, me);
    let n = geom::norm(d);
    let d = if n > 1e-9 { geom::div(d, n) } else { [1.0, 0.0] };
    throw(
        f,
        k,
        d,
        w.speed,
        [w.dmg, w.dmg_step, w.knock, w.hit_r],
        w.bounces.round() as i64,
        MalletKind::Run,
    );
}

/// `tremolo`: TREMOLO runs for `dur_beats` beats.
pub fn start_tremolo(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + param(ab, "dur_beats") * f.rules.beat;
    f.mallets.tremolo_until[k] = until;
    f.mallets.tremolo_mult[k] = param(ab, "mult");
    f.ev("tremolo", k as i64, vec![Value::Float(until)]);
}

/// `dead_stroke`: armed for `dur_beats` beats; the next mallet to land pins for `pin_beats` beats.
pub fn start_dead_stroke(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + param(ab, "dur_beats") * f.rules.beat;
    f.mallets.dead_until[k] = until;
    f.mallets.dead_pin[k] = param(ab, "pin_beats") * f.rules.beat;
    f.ev("dead_stroke", k as i64, vec![Value::Float(until)]);
}

/// `five_octaves`: a fan of `n` mallets over `beats` beats.
pub fn start_five_octaves(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let aim = dmath::atan2(d[1], d[0]);
    let n = (param(ab, "n") as i64).max(1);
    f.mallets.fans.push(Fan {
        owner: k,
        t0: f.t,
        t1: f.t + param(ab, "beats") * f.rules.beat,
        aim,
        n,
        emitted: 0,
        speed: param(ab, "speed"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        bounces: ab.get("bounces").unwrap_or(1.0).round() as i64,
    });
    f.ev("five_octaves", k as i64, vec![Value::Int(n)]);
}

/// The fans emit the mallets whose time has come.
fn fans_step(f: &mut Fight) {
    for i in 0..f.mallets.fans.len() {
        let fan = f.mallets.fans[i].clone();
        let s = (f.t - fan.t0) / (fan.t1 - fan.t0).max(1e-6);
        let due = ((s * fan.n as f64).floor() as i64 + 1).clamp(0, fan.n);
        for j in fan.emitted..due {
            let a = fan.aim + 2.0 * dmath::PI * j as f64 / fan.n as f64;
            let d = [dmath::cos(a), dmath::sin(a)];
            throw(
                f,
                fan.owner,
                d,
                fan.speed,
                [fan.dmg, 0.0, fan.knock, 0.1],
                fan.bounces,
                MalletKind::Octave,
            );
        }
        f.mallets.fans[i].emitted = due.max(fan.emitted);
    }
    f.mallets.fans.retain(|fan| fan.emitted < fan.n);
}

/// The later step: the fans emit, then every mallet flies, bounces and hits.
pub fn step(f: &mut Fight, dt: f64) {
    fans_step(f);
    let a = f.rules.a;
    let mut i = 0;
    while i < f.mallets.mallets.len() {
        let mut m = f.mallets.mallets[i].clone();
        m.p = geom::add(m.p, geom::scale(m.v, dt));
        let lim = a - m.hit_r;
        let mut gone = None;
        for ax in 0..2 {
            if m.p[ax].abs() > lim {
                if m.b >= m.b_max {
                    gone = Some("fizzle");
                    m.p[ax] = m.p[ax].clamp(-lim, lim);
                    break;
                }
                m.p[ax] = 2.0 * lim * m.p[ax].signum() - m.p[ax];
                m.v[ax] = -m.v[ax];
                m.b += 1;
                let data = vec![
                    Value::Int(m.id),
                    Value::Int(m.b),
                    Value::Float(m.p[0]),
                    Value::Float(m.p[1]),
                ];
                f.ev("mallet_bounce", m.owner as i64, data);
            }
        }
        let o = 1 - m.owner;
        if gone.is_none() && f.winner.is_none() {
            let op = f.balls[o].p;
            if geom::hypot(geom::sub(op, m.p)) < radius(f, o) + m.hit_r {
                gone = Some("hit");
                land(f, &m, o, op);
            }
        }
        match gone {
            Some(why) => {
                let data = vec![
                    Value::Int(m.id),
                    Value::Str(why.to_string()),
                    Value::Int(m.b),
                    Value::Float(m.p[0]),
                    Value::Float(m.p[1]),
                ];
                f.ev("mallet_end", m.owner as i64, data);
                f.mallets.mallets.remove(i);
            }
            None => {
                f.mallets.mallets[i] = m;
                i += 1;
            }
        }
    }
}

/// A mallet reaches the enemy `o` at `op`: its hit, TREMOLO's second hit, DEAD STROKE's pin, the volley's growth.
fn land(f: &mut Fight, m: &Mallet, o: usize, op: V2) {
    let k = m.owner;
    let dmg = m.dmg + m.dmg_step * m.b as f64;
    let landed = f.damage(k, o, dmg, m.kind.name(), op, m.b);
    if !landed {
        return;
    }
    f.knock(o, m.v, m.knock);
    if f.mallets.tremolo_until[k] > f.t {
        let mult = f.mallets.tremolo_mult[k];
        f.damage(k, o, dmg * mult, "tremolo", op, m.b);
    }
    if f.mallets.dead_until[k] > f.t {
        f.mallets.dead_until[k] = 0.0;
        let pin = f.mallets.dead_pin[k];
        let b = &mut f.balls[o];
        b.stun = geom::py_max(b.stun, pin);
        f.ev("pinned", o as i64, vec![Value::Int(k as i64), Value::Float(pin)]);
    }
    if m.kind == MalletKind::Run
        && let WeaponCfg::Mallets(w) = &f.balls[k].cfg.weapon
    {
        let b = &mut f.balls[k];
        b.n_vol = py::py_min(w.n_max, b.n_vol + w.grow);
    }
}

/// The record keys this kind adds: `MALLETS` when mallets can be in the ring.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if live(f) { &["MALLETS"] } else { &[] }
}

/// This kind's rows of one frame: every mallet in the air.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let rows = f.mallets.mallets.iter().map(|m| {
        Value::List(vec![
            Value::Int(m.owner as i64),
            Value::Int(m.id),
            Value::Float(m.p[0]),
            Value::Float(m.p[1]),
            Value::Float(m.v[0]),
            Value::Float(m.v[1]),
            Value::Int(m.b),
            Value::Int(i64::from(m.kind == MalletKind::Octave)),
        ])
    });
    out.push_nested("MALLETS", Value::List(rows.collect()));
}
