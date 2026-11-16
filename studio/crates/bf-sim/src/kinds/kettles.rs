//! Weapon kind `kettles`, TIMPANI's TWO KETTLES, and its abilities `pedal_slide` (PEDAL SLIDE) and `full_set` (FULL
//! SET). A Studio kind: the Python kit never had it, so this module is the source of truth (spec
//! `docs/superpowers/specs/2026-11-16-timpani-fighter-design.md`).
//!
//! At setup the timpani becomes a hopper (it stands still and slides only when knocked) and two kettles are set on
//! the floor on a diagonal at `spot` x the ring's half-size: the low one (`r_low`) on its own side, the high one
//! (`r_high`) across. On its grid (`every` beats; haste divides it) a timpani that is not stunned and not in the air
//! hops to the next kettle, `air16` 16ths in the air (a haste divides that too, at least one). Landing booms that
//! kettle: a ring leaves the kettle's centre and hits the enemy once when its front reaches it (`dmg_low` /
//! `dmg_high`, a knock away from the kettle). A hit by two different kettles within `pair_t` seconds adds the
//! interval hit (`dmg_pair`). The enemy shoves a kettle it overlaps (the kettle slides out to touch it, inside the
//! ropes).
//!
//! - PEDAL SLIDE: for `dur_beats` beats every kettle glides toward the enemy at `speed`; each kettle's first touch
//!   squeezes it (`dmg`, `knock` away from that kettle).
//! - FULL SET: two more kettles (`r_mid`) at the other corners of the diagonal square; the hops go round all four, and
//!   on every beat one kettle the timpani is not standing on booms by itself, in turn.
//!
//! Events: `kettles` (owner; per kettle x, y, radius), `kettle_hop` (owner; from, to, landing time),
//! `kettle_boom` (owner; kettle, x, y, reach), `pedal_slide` (owner; until), `full_set` (owner; kettles). Hits of
//! kind `kettle` (sub-number: the kettle), `interval` and `squeeze`. Recorded as `KETTLES` (each kettle
//! `(owner, i, x, y, radius)`), `KRINGS` (each live ring `(owner, kettle, x, y, radius now, reach)`) and `KHOP`
//! (each hop in the air `(owner, x0, y0, x1, y1, t0, t1)`).
//!
//! None of this draws from the generator, so a fight without a timpani is unchanged.

use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, radius, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, KettlesCfg, WeaponCfg};

/// One kettle on the floor.
#[derive(Clone, Debug, PartialEq)]
pub struct Kettle {
    /// The timpani it belongs to.
    pub owner: usize,
    /// Where it stands.
    pub p: V2,
    /// Its radius.
    pub r: f64,
    /// 0 low, 1 high, 2 a FULL SET kettle.
    pub pitch: i64,
    /// PEDAL SLIDE's squeeze is spent for this kettle.
    pub squeezed: bool,
}

/// A kettle's ring, growing.
#[derive(Clone, Debug, PartialEq)]
pub struct Ring {
    /// The timpani that boomed it.
    pub owner: usize,
    /// The kettle (index among the owner's kettles).
    pub kettle: usize,
    /// Its centre.
    pub p: V2,
    /// Its radius now.
    pub rad: f64,
    /// Units a second it grows.
    pub speed: f64,
    /// How far it reaches.
    pub reach: f64,
    /// Its damage.
    pub dmg: f64,
    /// Its knockback.
    pub knock: f64,
    /// It has hit.
    pub hit: bool,
}

/// A hop in the air.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hop {
    /// Where it left the floor.
    pub p0: V2,
    /// Where it lands.
    pub p1: V2,
    /// When it left.
    pub t0: f64,
    /// When it lands.
    pub t1: f64,
    /// The kettle it lands on.
    pub to: usize,
}

/// The kind's per-fight state (`Fight::kettles`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// Each fighter's kettles.
    pub kettles: [Vec<Kettle>; 2],
    /// The rings growing.
    pub rings: Vec<Ring>,
    /// Each fighter's hop in the air.
    pub hop: [Option<Hop>; 2],
    /// The kettle each fighter stands on (or last left).
    pub on: [usize; 2],
    /// Where each fighter is in its hop order.
    pub turn: [usize; 2],
    /// FULL SET's beat booms: the next in turn.
    pub beat_turn: [usize; 2],
    /// When each fighter's kettle last landed a hit, per kettle.
    pub last_hit: [Vec<f64>; 2],
    /// PEDAL SLIDE runs until.
    pub slide_until: [f64; 2],
    /// PEDAL SLIDE's speed, damage and knock.
    pub slide: [[f64; 3]; 2],
    /// FULL SET is on.
    pub full: [bool; 2],
}

/// Fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Kettles(_))
}

/// Kettles can be in the ring.
pub fn live(f: &Fight) -> bool {
    (0..2).any(|k| is(f, k))
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a KettlesCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Kettles(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not kettles", other.kind()),
    }
}

/// The side fighter `k` started on, as signs of x and y (never 0).
fn side(f: &Fight, k: usize) -> V2 {
    let p = f.balls[k].p;
    let s = |x: f64| if x < 0.0 { -1.0 } else { 1.0 };
    [s(p[0]), s(p[1])]
}

/// Setup: a hopper with its two kettles on the diagonal.
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k).clone();
    let d = w.spot * f.rules.a;
    let s = side(f, k);
    f.balls[k].hopper = true;
    f.balls[k].air = false;
    f.kettles.kettles[k] = vec![
        Kettle {
            owner: k,
            p: [s[0] * d, s[1] * d],
            r: w.r_low,
            pitch: 0,
            squeezed: false,
        },
        Kettle {
            owner: k,
            p: [-s[0] * d, -s[1] * d],
            r: w.r_high,
            pitch: 1,
            squeezed: false,
        },
    ];
    f.kettles.last_hit[k] = vec![f64::NEG_INFINITY; 2];
    f.kettles.hop[k] = None;
    // it stands "on" the high kettle so its first hop goes to the low one
    f.kettles.on[k] = 1;
    f.kettles.turn[k] = 1;
    announce(f, k, "kettles");
}

/// The event listing fighter `k`'s kettles.
fn announce(f: &mut Fight, k: usize, kind: &str) {
    let mut data = Vec::new();
    for c in &f.kettles.kettles[k] {
        data.extend([Value::Float(c.p[0]), Value::Float(c.p[1]), Value::Float(c.r)]);
    }
    f.ev(kind, k as i64, data);
}

/// The hop order: two kettles alternate; FULL SET's four go round the square.
fn order(n: usize) -> &'static [usize] {
    if n >= 4 { &[0, 2, 1, 3] } else { &[0, 1] }
}

/// The grid tick: a hop on its grid; under FULL SET a beat boom on every beat.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    if f.kettles.full[k] && py::imod_py(k16, 4) == 0 && f.winner.is_none() {
        let n = f.kettles.kettles[k].len();
        let ord = order(n);
        let mut i = ord[f.kettles.beat_turn[k] % ord.len()];
        if i == f.kettles.on[k] {
            f.kettles.beat_turn[k] += 1;
            i = ord[f.kettles.beat_turn[k] % ord.len()];
        }
        f.kettles.beat_turn[k] += 1;
        boom(f, k, i);
    }
    if stunned(f, k) || f.kettles.hop[k].is_some() {
        return;
    }
    let w = weapon(f, k).clone();
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    let ev16 = (py::round_half_even(main16 as f64 / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(k16, ev16) != 0 {
        return;
    }
    let n = f.kettles.kettles[k].len();
    let ord = order(n);
    f.kettles.turn[k] = (f.kettles.turn[k] + 1) % ord.len();
    let to = ord[f.kettles.turn[k]];
    let air16 = (py::round_half_even(w.air16 / f.ab_mult(k)) as i64).max(1);
    let air = air16 as f64 * f.rules.beat / 4.0;
    let p0 = f.balls[k].p;
    let lim = f.rules.a - radius(f, k);
    let tgt = f.kettles.kettles[k][to].p;
    let p1 = [geom::np_clip(tgt[0], -lim, lim), geom::np_clip(tgt[1], -lim, lim)];
    let j = geom::sub(p1, p0);
    let l = geom::hypot(j);
    if l > 1e-6 {
        let v = f.rules.v;
        f.balls[k].v = [j[0] / l * v, j[1] / l * v];
    }
    let t1 = f.t + air;
    f.kettles.hop[k] = Some(Hop {
        p0,
        p1,
        t0: f.t,
        t1,
        to,
    });
    f.balls[k].air = true;
    let from = f.kettles.on[k] as i64;
    f.ev(
        "kettle_hop",
        k as i64,
        vec![Value::Int(from), Value::Int(to as i64), Value::Float(t1)],
    );
}

/// Kettle `i` of fighter `k` booms: a ring leaves its centre.
fn boom(f: &mut Fight, k: usize, i: usize) {
    let w = weapon(f, k).clone();
    let c = f.kettles.kettles[k][i].clone();
    let (speed, reach, dmg, knock) = match c.pitch {
        0 => (w.speed_low, w.reach_low, w.dmg_low, w.knock_low),
        1 => (w.speed_high, w.reach_high, w.dmg_high, w.knock_high),
        _ => (
            0.5 * (w.speed_low + w.speed_high),
            0.5 * (w.reach_low + w.reach_high),
            0.5 * (w.dmg_low + w.dmg_high),
            0.5 * (w.knock_low + w.knock_high),
        ),
    };
    f.kettles.rings.push(Ring {
        owner: k,
        kettle: i,
        p: c.p,
        rad: c.r,
        speed,
        reach,
        dmg,
        knock,
        hit: false,
    });
    let data = vec![
        Value::Int(i as i64),
        Value::Float(c.p[0]),
        Value::Float(c.p[1]),
        Value::Float(reach),
    ];
    f.ev("kettle_boom", k as i64, data);
}

/// Fighter `k` lands its hop: it stands on the kettle (stepped out past the enemy if that is inside it) and the
/// kettle booms.
fn land(f: &mut Fight, k: usize) {
    let h = f.kettles.hop[k].take().expect("a hop to land");
    let r = radius(f, k);
    f.balls[k].p = h.p1;
    f.balls[k].air = false;
    let o = 1 - k;
    let o_p = f.balls[o].p;
    let rel = geom::sub(f.balls[k].p, o_p);
    let dr = geom::hypot(rel);
    let s = r + radius(f, o) + 0.02;
    if dr < s {
        let lim = f.rules.a - r;
        let u = if dr > 1e-6 { geom::div(rel, dr) } else { [1.0, 0.0] };
        f.balls[k].p = [
            geom::np_clip(o_p[0] + u[0] * s, -lim, lim),
            geom::np_clip(o_p[1] + u[1] * s, -lim, lim),
        ];
    }
    f.kettles.on[k] = h.to;
    if f.winner.is_none() {
        boom(f, k, h.to);
    }
}

/// `pedal_slide`: the kettles glide at the enemy for `dur_beats` beats.
pub fn start_pedal_slide(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + param(ab, "dur_beats") * f.rules.beat;
    f.kettles.slide_until[k] = until;
    f.kettles.slide[k] = [param(ab, "speed"), param(ab, "dmg"), param(ab, "knock")];
    for c in &mut f.kettles.kettles[k] {
        c.squeezed = false;
    }
    f.ev("pedal_slide", k as i64, vec![Value::Float(until)]);
}

/// `full_set`: two more kettles at the other corners.
pub fn start_full_set(f: &mut Fight, k: usize, _ab: &AbilityCfg) {
    if !is(f, k) || f.kettles.full[k] {
        return;
    }
    let w = weapon(f, k).clone();
    let d = w.spot * f.rules.a;
    let s = side(f, k);
    for p in [[s[0] * d, -s[1] * d], [-s[0] * d, s[1] * d]] {
        f.kettles.kettles[k].push(Kettle {
            owner: k,
            p,
            r: w.r_mid,
            pitch: 2,
            squeezed: false,
        });
        f.kettles.last_hit[k].push(f64::NEG_INFINITY);
    }
    f.kettles.full[k] = true;
    // the hop order restarts where the timpani stands
    let ord = order(4);
    f.kettles.turn[k] = ord.iter().position(|&i| i == f.kettles.on[k]).unwrap_or(0);
    announce(f, k, "full_set");
}

/// The later step: hops fly and land, kettles slide and get shoved, rings grow and hit.
pub fn step(f: &mut Fight, dt: f64) {
    let a = f.rules.a;
    for k in 0..2 {
        if !is(f, k) {
            continue;
        }
        if let Some(h) = f.kettles.hop[k] {
            if f.t + 1e-9 >= h.t1 {
                land(f, k);
            } else {
                let u = (f.t - h.t0) / py::py_max(1e-9, h.t1 - h.t0);
                f.balls[k].p = [h.p0[0] + (h.p1[0] - h.p0[0]) * u, h.p0[1] + (h.p1[1] - h.p0[1]) * u];
            }
        }
        let o = 1 - k;
        let (op, orad, o_air) = (f.balls[o].p, radius(f, o), f.balls[o].air);
        let sliding = f.kettles.slide_until[k] > f.t;
        let [s_speed, s_dmg, s_knock] = f.kettles.slide[k];
        for i in 0..f.kettles.kettles[k].len() {
            let mut c = f.kettles.kettles[k][i].clone();
            let touch = orad + c.r;
            if sliding {
                let d = geom::sub(op, c.p);
                let n = geom::hypot(d);
                if n > touch {
                    let stepl = py::py_min(s_speed * dt, n - touch);
                    c.p = geom::add(c.p, geom::scale(d, stepl / n));
                }
                if !c.squeezed && geom::hypot(geom::sub(op, c.p)) <= touch + 0.03 && f.winner.is_none() {
                    c.squeezed = true;
                    let away = geom::sub(op, c.p);
                    if f.damage(k, o, s_dmg, "squeeze", op, i as i64) {
                        f.knock(o, away, s_knock);
                    }
                }
            }
            if !o_air {
                let d = geom::sub(c.p, op);
                let n = geom::hypot(d);
                if n < touch {
                    let u = if n > 1e-6 { geom::div(d, n) } else { [1.0, 0.0] };
                    c.p = geom::add(op, geom::scale(u, touch));
                }
            }
            let lim = a - c.r;
            c.p = [geom::np_clip(c.p[0], -lim, lim), geom::np_clip(c.p[1], -lim, lim)];
            f.kettles.kettles[k][i] = c;
        }
    }
    let mut j = 0;
    while j < f.kettles.rings.len() {
        let mut g = f.kettles.rings[j].clone();
        g.rad += g.speed * dt;
        let o = 1 - g.owner;
        if !g.hit && f.winner.is_none() && !f.balls[o].air {
            let op = f.balls[o].p;
            let d = geom::hypot(geom::sub(op, g.p));
            if g.rad + radius(f, o) >= d && g.rad - g.speed * dt <= d + radius(f, o) {
                g.hit = true;
                ring_hit(f, &g, o, op);
            }
        }
        if g.rad >= g.reach {
            f.kettles.rings.remove(j);
        } else {
            f.kettles.rings[j] = g;
            j += 1;
        }
    }
}

/// A ring reaches the enemy `o` at `op`: its hit, and the interval when another kettle hit it lately.
fn ring_hit(f: &mut Fight, g: &Ring, o: usize, op: V2) {
    let k = g.owner;
    if !f.damage(k, o, g.dmg, "kettle", op, g.kettle as i64) {
        return;
    }
    f.knock(o, geom::sub(op, g.p), g.knock);
    let w = weapon(f, k).clone();
    let other = f.kettles.last_hit[k]
        .iter()
        .enumerate()
        .any(|(i, &t)| i != g.kettle && f.t - t <= w.pair_t);
    if other && w.dmg_pair > 0.0 {
        f.damage(k, o, w.dmg_pair, "interval", op, g.kettle as i64);
        for t in f.kettles.last_hit[k].iter_mut() {
            *t = f64::NEG_INFINITY;
        }
    } else if let Some(t) = f.kettles.last_hit[k].get_mut(g.kettle) {
        *t = f.t;
    }
}

/// The record keys this kind adds.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if live(f) { &["KETTLES", "KRINGS", "KHOP"] } else { &[] }
}

/// This kind's rows of one frame.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let kettles = f.kettles.kettles.iter().enumerate().flat_map(|(k, cs)| {
        cs.iter().enumerate().map(move |(i, c)| {
            Value::List(vec![
                Value::Int(k as i64),
                Value::Int(i as i64),
                Value::Float(c.p[0]),
                Value::Float(c.p[1]),
                Value::Float(c.r),
            ])
        })
    });
    out.push_nested("KETTLES", Value::List(kettles.collect()));
    let rings = f.kettles.rings.iter().map(|g| {
        Value::List(vec![
            Value::Int(g.owner as i64),
            Value::Int(g.kettle as i64),
            Value::Float(g.p[0]),
            Value::Float(g.p[1]),
            Value::Float(g.rad),
            Value::Float(g.reach),
        ])
    });
    out.push_nested("KRINGS", Value::List(rings.collect()));
    let hops = (0..2).filter_map(|k| {
        f.kettles.hop[k].map(|h| {
            Value::List(vec![
                Value::Int(k as i64),
                Value::Float(h.p0[0]),
                Value::Float(h.p0[1]),
                Value::Float(h.p1[0]),
                Value::Float(h.p1[1]),
                Value::Float(h.t0),
                Value::Float(h.t1),
            ])
        })
    });
    out.push_nested("KHOP", Value::List(hops.collect()));
}
