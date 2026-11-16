//! Weapon kind `needle`, PICCOLO's PIERCING NOTE, and its abilities `over_band` (OVER THE BAND) and `shatter` (GLASS
//! SHATTER); its TRILL is the engine's `haste`. A Studio kind: the Python kit never had it, so this module is the
//! source of truth (spec `docs/superpowers/specs/2026-11-16-piccolo-fighter-design.md`).
//!
//! On its grid (`every` beats; haste divides it) a piccolo that is not stunned fires a needle at the enemy as a bank
//! shot: aimed at the enemy (leading its motion) mirrored in one rope, the ropes taken in turn. A needle flies
//! straight at `speed` and bounces off the ropes; after `int(n_vol)` bounces it fizzles at the next rope. It passes
//! through its owner; when it passes through the enemy (within `hit_r` + the enemy's radius) it hits once, for `dmg`
//! and a knock along its flight, and keeps flying: it pierces. A landed PIERCING NOTE hit grows the owner's `n_vol`
//! by `grow` (up to `n_max`): one more bounce for every later needle.
//!
//! - OVER THE BAND: for `dur_beats` beats the owner's needles and shards cut through shields: a shielded enemy is hit
//!   as if it had none (event `pierce`).
//! - GLASS SHATTER: `n` shards break off the ropes one after another over `beats` beats, at points spread evenly
//!   round the ring (starting behind the enemy), each flying at the enemy's position at that moment at `speed`; a
//!   shard hits once for `dmg` and is gone, or breaks at the next rope.
//!
//! Events: `needle` (owner; id, x, y, vx, vy, kind), `needle_bounce` (owner; id, bounces so far, x, y), `needle_hit`
//! (owner; id, x, y), `needle_end` (owner; id, `fizzle` / `spent`, bounces, x, y), `over_band` (owner; until),
//! `pierce` (owner; x, y), `shatter` (owner; n). A hit is of kind `needle` (its sub-number the bounces) or `shard`.
//! Recorded as `NEEDLES`: each needle in the air `(owner, id, x, y, vx, vy, bounces, kind, hit)` with kind 0 for
//! PIERCING NOTE, 1 for a GLASS SHATTER shard, and hit 1 once it has pierced the enemy.
//!
//! None of this draws from the generator, so a fight without a piccolo is unchanged.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, radius, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, NeedleCfg, WeaponCfg};

/// Which move fired a needle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeedleKind {
    /// PIERCING NOTE's (the weapon).
    Note,
    /// A GLASS SHATTER shard.
    Shard,
}

impl NeedleKind {
    /// Its name in events and hits.
    pub fn name(self) -> &'static str {
        match self {
            NeedleKind::Note => "needle",
            NeedleKind::Shard => "shard",
        }
    }
}

/// One needle (or shard) in the air.
#[derive(Clone, Debug, PartialEq)]
pub struct Needle {
    /// The fighter that fired it.
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
    /// Damage of its hit.
    pub dmg: f64,
    /// Knockback of its hit.
    pub knock: f64,
    /// Its head's radius.
    pub hit_r: f64,
    /// It has hit the enemy (a needle hits once).
    pub hit: bool,
    /// Which move fired it.
    pub kind: NeedleKind,
}

/// A GLASS SHATTER in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Shower {
    /// The fighter it belongs to.
    pub owner: usize,
    /// When it started.
    pub t0: f64,
    /// When its last shard breaks off.
    pub t1: f64,
    /// The bearing from the centre to the point behind the enemy where the first shard breaks off.
    pub start: f64,
    /// How many shards.
    pub n: i64,
    /// How many have broken off.
    pub emitted: i64,
    /// Units a second each flies.
    pub speed: f64,
    /// Damage of each.
    pub dmg: f64,
    /// Knockback of each.
    pub knock: f64,
}

/// The kind's per-fight state (`Fight::needle`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The needles in the air, in the order they were fired.
    pub needles: Vec<Needle>,
    /// The next needle's number.
    pub next_id: i64,
    /// Bank-shot rope turn (per fighter).
    pub rope: [i64; 2],
    /// OVER THE BAND runs until (per fighter).
    pub over_until: [f64; 2],
    /// GLASS SHATTER showers in progress.
    pub showers: Vec<Shower>,
}

/// Fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Needle(_))
}

/// Needles can be in the ring: a fighter is a piccolo or has GLASS SHATTER (a copied move included).
pub fn live(f: &Fight) -> bool {
    (0..2).any(|k| is(f, k) || crate::abilities::has(f, k, &crate::roster::AbilityKind::Shatter))
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a NeedleCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Needle(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not needle", other.kind()),
    }
}

/// Setup: the bounces start at `n0`.
pub fn setup(f: &mut Fight, k: usize) {
    f.balls[k].n_vol = weapon(f, k).n0;
}

/// A needle leaves `p0` along the unit direction `d`.
#[allow(clippy::too_many_arguments)]
fn fire(f: &mut Fight, k: usize, p0: V2, d: V2, speed: f64, hit: [f64; 3], b_max: i64, kind: NeedleKind) {
    let [dmg, knock, hit_r] = hit;
    let lim = f.rules.a - hit_r;
    let p = [geom::np_clip(p0[0], -lim, lim), geom::np_clip(p0[1], -lim, lim)];
    let v = geom::scale(d, speed);
    let id = f.needle.next_id;
    f.needle.next_id += 1;
    f.needle.needles.push(Needle {
        owner: k,
        id,
        p,
        v,
        b: 0,
        b_max,
        dmg,
        knock,
        hit_r,
        hit: false,
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
    f.ev("needle", k as i64, data);
}

/// The grid tick: on its grid a piccolo that is not stunned fires a bank shot at the enemy.
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
    // aim at the enemy (leading its motion by the flight's time) mirrored in one of the ropes, the ropes taken in turn
    // (right, top, left, bottom): every shot ricochets at least once before it can land
    let (me, them) = (f.balls[k].p, &f.balls[1 - k]);
    let lim = f.rules.a - w.hit_r;
    let rope = py::imod_py(f.needle.rope[k], 4);
    f.needle.rope[k] += 1;
    let mut tgt = geom::add(them.p, geom::scale(them.v, 2.0 * lim / w.speed));
    let (ax, sign) = [(0, 1.0), (1, 1.0), (0, -1.0), (1, -1.0)][rope as usize];
    tgt[ax] = 2.0 * lim * sign - tgt[ax];
    let d = geom::sub(tgt, me);
    let n = geom::norm(d);
    let d = if n > 1e-9 { geom::div(d, n) } else { [1.0, 0.0] };
    let p0 = geom::add(me, geom::scale(d, radius(f, k) + w.hit_r));
    let b_max = ((f.balls[k].n_vol + 1e-9) as i64).max(1);
    fire(f, k, p0, d, w.speed, [w.dmg, w.knock, w.hit_r], b_max, NeedleKind::Note);
}

/// `over_band`: OVER THE BAND runs for `dur_beats` beats.
pub fn start_over_band(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + param(ab, "dur_beats") * f.rules.beat;
    f.needle.over_until[k] = until;
    f.ev("over_band", k as i64, vec![Value::Float(until)]);
}

/// `shatter`: a shower of `n` shards over `beats` beats.
pub fn start_shatter(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let e = f.balls[1 - k].p;
    // the first shard breaks off the rope behind the enemy (seen from the centre)
    let start = if geom::hypot(e) > 1e-9 {
        dmath::atan2(e[1], e[0])
    } else {
        0.0
    };
    let n = (param(ab, "n") as i64).max(1);
    f.needle.showers.push(Shower {
        owner: k,
        t0: f.t,
        t1: f.t + param(ab, "beats") * f.rules.beat,
        start,
        n,
        emitted: 0,
        speed: param(ab, "speed"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
    });
    f.ev("shatter", k as i64, vec![Value::Int(n)]);
}

/// The point on the ropes at bearing `a` from the centre.
fn rope_point(lim: f64, a: f64) -> V2 {
    let d = [dmath::cos(a), dmath::sin(a)];
    let m = d[0].abs().max(d[1].abs()).max(1e-9);
    [d[0] * lim / m, d[1] * lim / m]
}

/// The showers break off the shards whose time has come.
fn showers_step(f: &mut Fight) {
    let hit_r = 0.08;
    let lim = f.rules.a - hit_r;
    for i in 0..f.needle.showers.len() {
        let sh = f.needle.showers[i].clone();
        let s = (f.t - sh.t0) / (sh.t1 - sh.t0).max(1e-6);
        let due = ((s * sh.n as f64).floor() as i64 + 1).clamp(0, sh.n);
        for j in sh.emitted..due {
            // golden-angle steps spread the shards evenly round the ring, never two in a row on one rope
            let a = sh.start + 2.399_963_229_728_653 * j as f64;
            let p0 = rope_point(lim, a);
            let d = geom::sub(f.balls[1 - sh.owner].p, p0);
            let n = geom::norm(d);
            let d = if n > 1e-9 {
                geom::div(d, n)
            } else {
                geom::scale(p0, -1.0 / lim)
            };
            fire(
                f,
                sh.owner,
                p0,
                d,
                sh.speed,
                [sh.dmg, sh.knock, hit_r],
                0,
                NeedleKind::Shard,
            );
        }
        f.needle.showers[i].emitted = due.max(sh.emitted);
    }
    f.needle.showers.retain(|sh| sh.emitted < sh.n);
}

/// The later step: the showers break off shards, then every needle flies, bounces and pierces.
pub fn step(f: &mut Fight, dt: f64) {
    showers_step(f);
    let a = f.rules.a;
    let mut i = 0;
    while i < f.needle.needles.len() {
        let mut m = f.needle.needles[i].clone();
        let p_prev = m.p;
        m.p = geom::add(m.p, geom::scale(m.v, dt));
        let lim = a - m.hit_r;
        let mut gone = None;
        for ax in 0..2 {
            // a shard has just left a rope: it breaks only at another one
            if m.p[ax].abs() > lim && !(m.kind == NeedleKind::Shard && m.v[ax] * m.p[ax] < 0.0) {
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
                f.ev("needle_bounce", m.owner as i64, data);
            }
        }
        let o = 1 - m.owner;
        if !m.hit && f.winner.is_none() {
            // a fast needle is tested along its whole step, so it cannot jump over a ball
            let op = f.balls[o].p;
            let reach = radius(f, o) + m.hit_r;
            if seg_dist(p_prev, m.p, op) < reach || geom::hypot(geom::sub(op, m.p)) < reach {
                m.hit = true;
                land(f, &m, o, op);
                if m.kind == NeedleKind::Shard {
                    gone = Some("spent");
                }
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
                f.ev("needle_end", m.owner as i64, data);
                f.needle.needles.remove(i);
            }
            None => {
                f.needle.needles[i] = m;
                i += 1;
            }
        }
    }
}

/// The distance from `c` to the segment `a`..`b`.
fn seg_dist(a: V2, b: V2, c: V2) -> f64 {
    let ab = geom::sub(b, a);
    let l2 = ab[0] * ab[0] + ab[1] * ab[1];
    if l2 < 1e-18 {
        return geom::hypot(geom::sub(c, a));
    }
    let t = (((c[0] - a[0]) * ab[0] + (c[1] - a[1]) * ab[1]) / l2).clamp(0.0, 1.0);
    geom::hypot(geom::sub(c, geom::add(a, geom::scale(ab, t))))
}

/// A needle passes through the enemy `o` at `op`: its hit (through a shield under OVER THE BAND), the bounces' growth.
fn land(f: &mut Fight, m: &Needle, o: usize, op: V2) {
    let k = m.owner;
    f.ev(
        "needle_hit",
        k as i64,
        vec![Value::Int(m.id), Value::Float(m.p[0]), Value::Float(m.p[1])],
    );
    let over = f.needle.over_until[k] > f.t;
    let shield = f.balls[o].shield;
    if over && shield > 0.0 {
        f.balls[o].shield = 0.0;
        f.ev("pierce", k as i64, vec![Value::Float(op[0]), Value::Float(op[1])]);
    }
    let landed = f.damage(k, o, m.dmg, m.kind.name(), op, m.b);
    if over && shield > 0.0 {
        f.balls[o].shield = shield;
    }
    if !landed {
        return;
    }
    f.knock(o, m.v, m.knock);
    if m.kind == NeedleKind::Note
        && let WeaponCfg::Needle(w) = &f.balls[k].cfg.weapon
    {
        let b = &mut f.balls[k];
        b.n_vol = py::py_min(w.n_max, b.n_vol + w.grow);
    }
}

/// The record keys this kind adds: `NEEDLES` when needles can be in the ring.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if live(f) { &["NEEDLES"] } else { &[] }
}

/// This kind's rows of one frame: every needle in the air.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let rows = f.needle.needles.iter().map(|m| {
        Value::List(vec![
            Value::Int(m.owner as i64),
            Value::Int(m.id),
            Value::Float(m.p[0]),
            Value::Float(m.p[1]),
            Value::Float(m.v[0]),
            Value::Float(m.v[1]),
            Value::Int(m.b),
            Value::Int(i64::from(m.kind == NeedleKind::Shard)),
            Value::Int(i64::from(m.hit)),
        ])
    });
    out.push_nested("NEEDLES", Value::List(rows.collect()));
}
