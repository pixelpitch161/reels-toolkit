//! Weapon kind `slapback`, MELLOPHONE's SLAPBACK, and its three abilities `horns_up` (HORNS UP), `press_box` (PRESS
//! BOX) and `stadium_echo` (STADIUM ECHO). A Studio kind: the Python kit never had it, so this module is the source
//! of truth (spec `docs/superpowers/specs/2026-11-16-mellophone-fighter-design.md`).
//!
//! On its grid (`every` beats; haste divides it) a mellophone that is not stunned blows a wave at the enemy: a wide
//! arc (half-width `half_w`) flying straight at `speed`. A wave passes through its owner, hits the enemy it touches
//! (within `half_w` + the enemy's radius) for `dmg` + `dmg_step` x its generation and is gone. A wave that reaches a
//! rope stops there; if it has echoes left, `delay` beats later its echo leaves that rope point, aimed at the enemy
//! where it is then, one generation up and one echo fewer (a real slapback: the sound off the far wall, late). A first
//! wave carries `int(n_echo)` echoes; a landed SLAPBACK hit grows the owner's `n_vol` by `grow` (up to `n_max`).
//!
//! - HORNS UP: `n` stabs leave the ball one a 16th, straight at the enemy (`speed`, `dmg`, knock `knock`, half-width
//!   0.12, no echoes); the last one stuns for `stun_beats` beats.
//! - PRESS BOX: for `dur_beats` beats every echo of the owner leaves as two waves, `spread` degrees either side.
//! - STADIUM ECHO: one giant wave (`half_w`, `speed`, `dmg`, `dmg_step` a leg, `knock`) that passes through the enemy
//!   (one hit a leg) and echoes off every rope after `delay` beats, until `beats` beats from its start.
//!
//! Events: `slap` (owner; id, x, y, vx, vy, kind, generation), `slap_rope` (owner; id, x, y, 1 if it will echo
//! else 0), `slap_end` (owner; id, `hit` / `rope`), `horns_up` (owner; n), `press_box` (owner; until),
//! `stadium_echo` (owner; until), `horns_stun` (the stunned fighter; owner, seconds). A hit is of kind `slapback`,
//! `stab` or `stadium`, its sub-number the generation. Recorded as `SLAP`: each wave `(owner, id, x, y, vx, vy,
//! generation, half_w, kind)` (kind 0 SLAPBACK, 1 a stab, 2 STADIUM ECHO), and `SLAPQ`: each echo waiting on a rope
//! `(owner, x, y, seconds until it leaves, generation, kind)`.
//!
//! None of this draws from the generator, so a fight without a mellophone is unchanged.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::{has, param};
use crate::fight::{Fight, radius, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, AbilityKind, SlapbackCfg, WeaponCfg};

/// Which move blew a wave.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveKind {
    /// SLAPBACK's (the weapon) and its echoes.
    Slap,
    /// A HORNS UP stab.
    Stab,
    /// STADIUM ECHO's giant wave and its echoes.
    Stadium,
}

impl WaveKind {
    /// Its name in events and hits.
    pub fn name(self) -> &'static str {
        match self {
            WaveKind::Slap => "slapback",
            WaveKind::Stab => "stab",
            WaveKind::Stadium => "stadium",
        }
    }

    /// Its number in the record.
    fn code(self) -> i64 {
        match self {
            WaveKind::Slap => 0,
            WaveKind::Stab => 1,
            WaveKind::Stadium => 2,
        }
    }
}

/// What a wave carries, the same for every echo of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    /// Which move.
    pub kind: WaveKind,
    /// Units a second.
    pub speed: f64,
    /// Half its width.
    pub half_w: f64,
    /// Damage of generation 0.
    pub dmg: f64,
    /// Damage added a generation.
    pub dmg_step: f64,
    /// Knockback of its hit.
    pub knock: f64,
    /// Seconds from the rope to the echo.
    pub delay: f64,
}

/// One wave in the air.
#[derive(Clone, Debug, PartialEq)]
pub struct Wave {
    /// The fighter that blew it.
    pub owner: usize,
    /// Its number (per fight).
    pub id: i64,
    /// Where its middle is.
    pub p: V2,
    /// Its velocity.
    pub v: V2,
    /// Its generation: 0 for a wave from the bell, one more each echo.
    pub generation: i64,
    /// Echoes it still has.
    pub echoes: i64,
    /// Seconds its stun lasts on a hit (0: none).
    pub stun: f64,
    /// Whether it has hit (a STADIUM ECHO leg passes through and hits once).
    pub touched: bool,
    /// What it carries.
    pub shape: Shape,
}

/// An echo waiting on a rope.
#[derive(Clone, Debug, PartialEq)]
pub struct Pending {
    /// The fighter whose wave it is.
    pub owner: usize,
    /// The rope point it leaves from.
    pub p: V2,
    /// When it leaves.
    pub t_fire: f64,
    /// Its generation.
    pub generation: i64,
    /// Echoes it will still have.
    pub echoes: i64,
    /// What it carries.
    pub shape: Shape,
}

/// A HORNS UP volley in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Stabs {
    /// The fighter blowing it.
    pub owner: usize,
    /// When it started.
    pub t0: f64,
    /// How many stabs.
    pub n: i64,
    /// How many have left.
    pub emitted: i64,
    /// Seconds between two.
    pub gap: f64,
    /// The last stab's stun, seconds.
    pub stun: f64,
    /// What each carries.
    pub shape: Shape,
}

/// The kind's per-fight state (`Fight::slapback`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The waves in the air, in the order they left.
    pub waves: Vec<Wave>,
    /// The echoes waiting on the ropes, in the order they landed.
    pub pending: Vec<Pending>,
    /// HORNS UP volleys in progress.
    pub stabs: Vec<Stabs>,
    /// The next wave's number.
    pub next_id: i64,
    /// PRESS BOX lights the ropes until (per fighter).
    pub press_until: [f64; 2],
    /// PRESS BOX's split, radians either side (per fighter).
    pub press_spread: [f64; 2],
    /// STADIUM ECHO keeps echoing until (per fighter).
    pub stadium_until: [f64; 2],
}

/// Fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Slapback(_))
}

/// Waves can be in the ring: a fighter is a mellophone or has one of its moves (a copied move included).
pub fn live(f: &Fight) -> bool {
    (0..2).any(|k| {
        is(f, k)
            || has(f, k, &AbilityKind::HornsUp)
            || has(f, k, &AbilityKind::PressBox)
            || has(f, k, &AbilityKind::StadiumEcho)
    })
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a SlapbackCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Slapback(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not slapback", other.kind()),
    }
}

/// Setup: the echoes start at `n0`.
pub fn setup(f: &mut Fight, k: usize) {
    f.balls[k].n_vol = weapon(f, k).n0;
}

/// The unit vector from `a` toward `b` (along x when they meet).
fn toward(a: V2, b: V2) -> V2 {
    let d = geom::sub(b, a);
    let n = geom::norm(d);
    if n > 1e-9 { geom::div(d, n) } else { [1.0, 0.0] }
}

/// A unit vector turned by `ang` radians.
fn turn(d: V2, ang: f64) -> V2 {
    let (c, s) = (dmath::cos(ang), dmath::sin(ang));
    [d[0] * c - d[1] * s, d[0] * s + d[1] * c]
}

/// A wave of fighter `k` leaves `p` along the unit direction `d`.
fn blow(f: &mut Fight, k: usize, p: V2, d: V2, generation: i64, echoes: i64, stun: f64, shape: Shape) {
    let lim = f.rules.a - 0.02;
    let p = [geom::np_clip(p[0], -lim, lim), geom::np_clip(p[1], -lim, lim)];
    let v = geom::scale(d, shape.speed);
    let id = f.slapback.next_id;
    f.slapback.next_id += 1;
    let data = vec![
        Value::Int(id),
        Value::Float(p[0]),
        Value::Float(p[1]),
        Value::Float(v[0]),
        Value::Float(v[1]),
        Value::Str(shape.kind.name().to_string()),
        Value::Int(generation),
    ];
    f.ev("slap", k as i64, data);
    f.slapback.waves.push(Wave {
        owner: k,
        id,
        p,
        v,
        generation,
        echoes,
        stun,
        touched: false,
        shape,
    });
}

/// A wave leaves fighter `k`'s bell along `d`.
fn from_bell(f: &mut Fight, k: usize, d: V2, echoes: i64, stun: f64, shape: Shape) {
    let p = geom::add(f.balls[k].p, geom::scale(d, radius(f, k) + 0.05));
    blow(f, k, p, d, 0, echoes, stun, shape);
}

/// The grid tick: on its grid a mellophone that is not stunned blows a wave at the enemy, leading its motion a
/// little, carrying `int(n_vol)` echoes.
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
    let (me, them) = (f.balls[k].p, &f.balls[1 - k]);
    let flight = geom::hypot(geom::sub(them.p, me)) / w.speed;
    let tgt = geom::add(them.p, geom::scale(them.v, 0.5 * flight));
    let d = toward(me, tgt);
    let shape = Shape {
        kind: WaveKind::Slap,
        speed: w.speed,
        half_w: w.half_w,
        dmg: w.dmg,
        dmg_step: w.dmg_step,
        knock: w.knock,
        delay: w.delay * f.rules.beat,
    };
    let echoes = (f.balls[k].n_vol + 1e-9) as i64;
    from_bell(f, k, d, echoes, 0.0, shape);
}

/// `horns_up`: `n` stabs, one a 16th.
pub fn start_horns_up(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let n = (param(ab, "n") as i64).max(1);
    let shape = Shape {
        kind: WaveKind::Stab,
        speed: param(ab, "speed"),
        half_w: 0.12,
        dmg: param(ab, "dmg"),
        dmg_step: 0.0,
        knock: param(ab, "knock"),
        delay: 0.0,
    };
    f.slapback.stabs.push(Stabs {
        owner: k,
        t0: f.t,
        n,
        emitted: 0,
        gap: f.rules.beat / 4.0,
        stun: param(ab, "stun_beats") * f.rules.beat,
        shape,
    });
    f.ev("horns_up", k as i64, vec![Value::Int(n)]);
}

/// `press_box`: echoes split for `dur_beats` beats.
pub fn start_press_box(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + param(ab, "dur_beats") * f.rules.beat;
    f.slapback.press_until[k] = until;
    f.slapback.press_spread[k] = geom::radians(param(ab, "spread"));
    f.ev("press_box", k as i64, vec![Value::Float(until)]);
}

/// `stadium_echo`: one giant wave that keeps echoing for `beats` beats.
pub fn start_stadium_echo(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + param(ab, "beats") * f.rules.beat;
    f.slapback.stadium_until[k] = until;
    let shape = Shape {
        kind: WaveKind::Stadium,
        speed: param(ab, "speed"),
        half_w: param(ab, "half_w"),
        dmg: param(ab, "dmg"),
        dmg_step: param(ab, "dmg_step"),
        knock: param(ab, "knock"),
        delay: param(ab, "delay") * f.rules.beat,
    };
    let d = toward(f.balls[k].p, f.balls[1 - k].p);
    f.ev("stadium_echo", k as i64, vec![Value::Float(until)]);
    from_bell(f, k, d, i64::MAX / 4, 0.0, shape);
}

/// The volleys and the waiting echoes leave whose time has come.
fn emit(f: &mut Fight) {
    for i in 0..f.slapback.stabs.len() {
        let s = f.slapback.stabs[i].clone();
        let due = (((f.t - s.t0) / s.gap.max(1e-6)).floor() as i64 + 1).clamp(0, s.n);
        for j in s.emitted..due {
            let k = s.owner;
            let d = toward(f.balls[k].p, f.balls[1 - k].p);
            let stun = if j == s.n - 1 { s.stun } else { 0.0 };
            from_bell(f, k, d, 0, stun, s.shape.clone());
        }
        f.slapback.stabs[i].emitted = due.max(s.emitted);
    }
    f.slapback.stabs.retain(|s| s.emitted < s.n);
    let mut i = 0;
    while i < f.slapback.pending.len() {
        if f.slapback.pending[i].t_fire > f.t + 1e-9 {
            i += 1;
            continue;
        }
        let q = f.slapback.pending.remove(i);
        let k = q.owner;
        let d = toward(q.p, f.balls[1 - k].p);
        let split = q.shape.kind == WaveKind::Slap && f.slapback.press_until[k] > f.t;
        if split {
            let a = f.slapback.press_spread[k];
            blow(f, k, q.p, turn(d, a), q.generation, q.echoes, 0.0, q.shape.clone());
            blow(f, k, q.p, turn(d, -a), q.generation, q.echoes, 0.0, q.shape);
        } else {
            blow(f, k, q.p, d, q.generation, q.echoes, 0.0, q.shape);
        }
    }
}

/// The later step: the volleys and echoes leave, then every wave flies, hits, and lands on a rope.
pub fn step(f: &mut Fight, dt: f64) {
    emit(f);
    let lim = f.rules.a - 0.02;
    let mut i = 0;
    while i < f.slapback.waves.len() {
        let mut w = f.slapback.waves[i].clone();
        w.p = geom::add(w.p, geom::scale(w.v, dt));
        let mut gone = None;
        let o = 1 - w.owner;
        if f.winner.is_none() && !w.touched {
            let op = f.balls[o].p;
            if geom::hypot(geom::sub(op, w.p)) < radius(f, o) + w.shape.half_w {
                w.touched = true;
                land(f, &w, o, op);
                if w.shape.kind != WaveKind::Stadium {
                    gone = Some("hit");
                }
            }
        }
        if gone.is_none() && (w.p[0].abs() > lim || w.p[1].abs() > lim) {
            gone = Some("rope");
            w.p = [w.p[0].clamp(-lim, lim), w.p[1].clamp(-lim, lim)];
            let k = w.owner;
            let t_fire = f.t + w.shape.delay;
            let echo = w.echoes > 0
                && (w.shape.kind != WaveKind::Stadium || t_fire < f.slapback.stadium_until[k])
                && f.winner.is_none();
            let data = vec![
                Value::Int(w.id),
                Value::Float(w.p[0]),
                Value::Float(w.p[1]),
                Value::Int(i64::from(echo)),
            ];
            f.ev("slap_rope", k as i64, data);
            if echo {
                f.slapback.pending.push(Pending {
                    owner: k,
                    p: w.p,
                    t_fire,
                    generation: w.generation + 1,
                    echoes: w.echoes - 1,
                    shape: w.shape.clone(),
                });
            }
        }
        match gone {
            Some(why) => {
                let data = vec![Value::Int(w.id), Value::Str(why.to_string())];
                f.ev("slap_end", w.owner as i64, data);
                f.slapback.waves.remove(i);
            }
            None => {
                f.slapback.waves[i] = w;
                i += 1;
            }
        }
    }
}

/// A wave reaches the enemy `o` at `op`: its hit, a stab's stun, the echoes' growth.
fn land(f: &mut Fight, w: &Wave, o: usize, op: V2) {
    let k = w.owner;
    let dmg = w.shape.dmg + w.shape.dmg_step * w.generation as f64;
    if !f.damage(k, o, dmg, w.shape.kind.name(), op, w.generation) {
        return;
    }
    f.knock(o, w.v, w.shape.knock);
    if w.stun > 0.0 {
        let b = &mut f.balls[o];
        b.stun = geom::py_max(b.stun, w.stun);
        f.ev("horns_stun", o as i64, vec![Value::Int(k as i64), Value::Float(w.stun)]);
    }
    if w.shape.kind == WaveKind::Slap
        && let WeaponCfg::Slapback(cfg) = &f.balls[k].cfg.weapon
    {
        let b = &mut f.balls[k];
        b.n_vol = py::py_min(cfg.n_max, b.n_vol + cfg.grow);
    }
}

/// The record keys this kind adds: `SLAP` and `SLAPQ` when waves can be in the ring.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if live(f) { &["SLAP", "SLAPQ"] } else { &[] }
}

/// This kind's rows of one frame: every wave in the air and every echo waiting.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let rows = f.slapback.waves.iter().map(|w| {
        Value::List(vec![
            Value::Int(w.owner as i64),
            Value::Int(w.id),
            Value::Float(w.p[0]),
            Value::Float(w.p[1]),
            Value::Float(w.v[0]),
            Value::Float(w.v[1]),
            Value::Int(w.generation),
            Value::Float(w.shape.half_w),
            Value::Int(w.shape.kind.code()),
        ])
    });
    out.push_nested("SLAP", Value::List(rows.collect()));
    let rows = f.slapback.pending.iter().map(|q| {
        Value::List(vec![
            Value::Int(q.owner as i64),
            Value::Float(q.p[0]),
            Value::Float(q.p[1]),
            Value::Float((q.t_fire - f.t).max(0.0)),
            Value::Int(q.generation),
            Value::Int(q.shape.kind.code()),
        ])
    });
    out.push_nested("SLAPQ", Value::List(rows.collect()));
}
