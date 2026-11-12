//! Weapon kind `spotlight`, ALTO SAX's SPOTLIGHT SOLO: `fight.py` 4369 to 4512 (`spot_setup`, `_spot_line`,
//! `spot_tick`, `spot_pos`, `spot_step`, `start_encore`), and the abilities `octave`, `altissimo` and `encore`
//! (`trigger` 2958 to 2965).
//!
//! Every `every` beats (`off16` 16ths after the bar line; a haste divides it) a spotlight sweeps the floor for
//! `sweep16` 16ths: a disc of light (radius `blade_len`) gliding along a straight line `span` long through the
//! enemy's spot predicted `lead` x its motion to mid-sweep, square to the enemy's heading (turned by up to `jitter`
//! degrees, drawn from the fight's generator), kept inside the ropes. On every 16th an enemy inside a live light
//! (within its radius + 0.6 R) burns. A sweep that burned grows the light and the burn when it ends.
//!
//! OCTAVE KEY adds a second light to every sweep, square to the first, there and back. ALTISSIMO narrows the light
//! and makes it burn hotter. ENCORE is one light that chases the enemy, with phone lights on every 8th.
//!
//! Python keeps the lights in `Fight.spots`, their ids in `Fight.next_spot`, the phone lights in `Fight.phones`, and
//! hangs `burn`, `oct_until`, `alt_until`, `alt_ab` and `encore` on the ball; here they are [`State`],
//! `Fight::spotlight`. The generator is drawn from once per sweep (the jitter, whatever `jitter` is) and twice per
//! phone light.

use bf_core::event::Value;
use bf_core::{dmath, py};

use crate::abilities::{ab_dur, param};
use crate::fight::{Fight, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, SpotlightCfg, WeaponCfg};

/// A light (an entry of `Fight.spots`, the dict `spot_tick` or `start_encore` makes).
#[derive(Clone, Debug, PartialEq)]
pub struct Spot {
    /// The fighter whose light it is.
    pub owner: usize,
    /// Its id.
    pub id: i64,
    /// Where a sweep starts (`a`; unused by the chase light).
    pub a: V2,
    /// Where a sweep ends (`b`).
    pub b: V2,
    /// Where the chase light is (`c`; unused by a sweep).
    pub c: V2,
    /// When it came on.
    pub t0: f64,
    /// When it goes out.
    pub t1: f64,
    /// Its radius.
    pub r: f64,
    /// The damage of one burn.
    pub dmg: f64,
    /// Whether it has burned the enemy.
    pub hit: bool,
    /// 0 a sweep, 1 the octave's second light, 2 ENCORE's chase light.
    pub kind: i64,
    /// 1 when it came on during ALTISSIMO.
    pub hot: i64,
    /// 2 when it runs there and back.
    pub passes: i64,
    /// Whether a burn of this light grows the next ones.
    pub grow: bool,
}

/// ALTISSIMO's numbers (`B.alt_ab`), with the time it lasts to (`B.alt_until`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alt {
    /// `B.alt_until`.
    pub until: f64,
    /// `ab['span_k']`: the line's length factor.
    pub span_k: f64,
    /// `ab['narrow']`: the radius factor.
    pub narrow: f64,
    /// `ab['hot']`: the damage factor.
    pub hot: f64,
}

/// A running ENCORE (`B.encore`): the ability's numbers, its end and its chase light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Encore {
    /// `ab['n_phone']`.
    pub n_phone: f64,
    /// `ab['spread']`.
    pub spread: f64,
    /// `ab['phone_r']`.
    pub phone_r: f64,
    /// `ab['phone_dmg']`.
    pub phone_dmg: f64,
    /// `ab['speed']`.
    pub speed: f64,
    /// When it ends.
    pub t1: f64,
    /// The id of its chase light (Python holds the light itself, the same dict `Fight.spots` holds).
    pub spot: i64,
}

/// Weapon kind `spotlight`'s per-fight state (`Fight::spotlight`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `Fight.spots`: the lights, oldest first.
    pub spots: Vec<Spot>,
    /// `Fight.next_spot`: the next light's id.
    pub next_spot: i64,
    /// `Fight.phones`: the phone lights of the last 8th, each with the time it came on.
    pub phones: Vec<(f64, V2)>,
    /// `B.burn` by fighter: the damage of a burn of its next sweep.
    pub burn: [f64; 2],
    /// `B.oct_until` by fighter (`None`: Python's `getattr(B, 'oct_until', -1.0)`).
    pub oct_until: [Option<f64>; 2],
    /// `B.alt_until` and `B.alt_ab` by fighter.
    pub alt: [Option<Alt>; 2],
    /// `B.encore` by fighter.
    pub encore: [Option<Encore>; 2],
}

/// `B.w['kind'] == 'spotlight'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Spotlight(_))
}

/// The spotlight's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a spotlight.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a SpotlightCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Spotlight(w) => w,
        other => panic!("fighter {k} ({}) has no spotlight weapon: {other:?}", cfg.name),
    }
}

/// `spot_setup(k)` (4387 to 4393), from `run()` 5888 to 5890: the light starts at `base` wide and burns `dmg`.
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    f.balls[k].blade_len = w.base;
    f.spotlight.burn[k] = w.dmg;
}

/// `_spot_line(k, mid, u, span)` (4394 to 4397): the two ends of a line `span` long through `mid` along `u`, each
/// held inside the ropes (0.25 short of them).
pub fn spot_line(a: f64, mid: V2, u: V2, span: f64) -> (V2, V2) {
    let lim = a - 0.25;
    // mid - u * span / 2, mid + u * span / 2
    let half = geom::div(geom::scale(u, span), 2.0);
    let (p, q) = (geom::sub(mid, half), geom::add(mid, half));
    (
        [geom::np_clip(p[0], -lim, lim), geom::np_clip(p[1], -lim, lim)],
        [geom::np_clip(q[0], -lim, lim), geom::np_clip(q[1], -lim, lim)],
    )
}

/// `spot_pos(Sp, t)` (4461 to 4469): where a light is at `t`: the chase light where it stands, a sweep eased along
/// its line (`passes` 2: there and back).
pub fn spot_pos(sp: &Spot, t: f64) -> V2 {
    if sp.kind == 2 {
        return sp.c;
    }
    let mut u = py::py_min(1.0, py::py_max(0.0, (t - sp.t0) / py::py_max(1e-9, sp.t1 - sp.t0)));
    if sp.passes == 2 {
        u = if u < 0.5 { 2.0 * u } else { 2.0 - 2.0 * u };
    }
    let u = u * u * (3.0 - 2.0 * u);
    geom::add(sp.a, geom::scale(geom::sub(sp.b, sp.a), u))
}

/// `spot_tick(k, k16)` (4399 to 4460), from `run()` 6076 to 6080, on every 16th: the burns of every live light,
/// ENCORE's phone lights on every 8th, and a new sweep on the weapon's grid.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    let r_ball = f.rules.r;
    let s16 = f.rules.beat / 4.0;
    let alt = f.spotlight.alt[k];
    // hot = self.t < getattr(B, 'alt_until', -1.0)
    let hot = alt.is_some_and(|a| f.t < a.until);
    let en = f.spotlight.encore[k];
    // burns on the 16th grid (every live light, once per 16th)
    for i in 0..f.spotlight.spots.len() {
        let sp = &f.spotlight.spots[i];
        if sp.owner != k || f.t > sp.t1 + 1e-9 || f.t + 1e-9 < sp.t0 {
            continue;
        }
        let (id, radius, dmg) = (sp.id, sp.r, sp.dmg);
        let c = spot_pos(sp, f.t);
        let o_p = f.balls[1 - k].p;
        let rel = geom::sub(o_p, c);
        let dist = geom::hypot(rel);
        if dist < radius + 0.6 * r_ball {
            if f.damage(k, 1 - k, dmg, "burn", o_p, 0) {
                f.spotlight.spots[i].hit = true;
                let away = if dist > 1e-3 { rel } else { f.balls[1 - k].v };
                f.knock(1 - k, away, w.knock);
            }
            f.ev(
                "burn",
                k as i64,
                vec![Value::Int(id), Value::Float(c[0]), Value::Float(c[1])],
            );
        }
    }
    if let Some(en) = en
        && py::imod_py(k16, 2) == 0
    {
        // ENCORE: the phone lights on every 8th
        let n = en.n_phone as i64;
        let o_p = f.balls[1 - k].p;
        let mut phones = Vec::new();
        for _ in 0..n {
            // O.p + self.rng.uniform(-1, 1, 2) * ab['spread']
            let mut d = [0.0; 2];
            f.rng.fill_uniform(-1.0, 1.0, &mut d);
            phones.push((f.t, geom::add(o_p, geom::scale(d, en.spread))));
        }
        f.spotlight.phones.clone_from(&phones);
        f.ev("phones", k as i64, vec![Value::Int(n)]);
        for (_, q) in phones {
            let o_p = f.balls[1 - k].p;
            if geom::hypot(geom::sub(o_p, q)) < en.phone_r && f.damage(k, 1 - k, en.phone_dmg, "phone", q, 0) {
                f.ev("phone_burn", k as i64, vec![Value::Float(q[0]), Value::Float(q[1])]);
            }
        }
    }
    // a new sweep on the weapon's grid
    if en.is_some() || stunned(f, k) || f.balls[k].smashed > 0.0 {
        return;
    }
    let m = f.ab_mult(k);
    // main16 = int(round(4 * w['every'])); (k16 - int(w.get('off16', 0))) % max(1, int(round(main16 / m)))
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    let every16 = (py::round_half_even(main16 as f64 / m) as i64).max(1);
    if py::imod_py(k16 - w.off16 as i64, every16) != 0 {
        return;
    }
    let (b_p, o_p, o_v) = (f.balls[k].p, f.balls[1 - k].p, f.balls[1 - k].v);
    let dur = w.sweep16 * s16;
    // mid = O.p + O.v * (dur / 2) * w.get('lead', 1.0)
    let mid = geom::add(o_p, geom::scale(geom::scale(o_v, dur / 2.0), w.lead));
    let hv = geom::div(o_v, geom::hypot(o_v) + 1e-9);
    let u = [-hv[1], hv[0]];
    let j = geom::radians(f.rng.uniform(-1.0, 1.0) * w.jitter);
    let (cj, sj) = (dmath::cos(j), dmath::sin(j));
    let mut u = [u[0] * cj - u[1] * sj, u[0] * sj + u[1] * cj];
    if geom::dot(u, geom::sub(b_p, mid)) > 0.0 {
        // the light comes in from the alto's side
        u = [-u[0], -u[1]];
    }
    let ab_alt = if hot { alt } else { None };
    let span = w.span * ab_alt.map_or(1.0, |a| a.span_k);
    let r = f.balls[k].blade_len * ab_alt.map_or(1.0, |a| a.narrow);
    let dmg = f.spotlight.burn[k] * ab_alt.map_or(1.0, |a| a.hot);
    let (t0, t1) = (f.t, f.t + dur);
    let octave = f.spotlight.oct_until[k].is_some_and(|until| f.t < until);
    // the sweep, then OCTAVE KEY's second light: square to it, twice as fast (there and back)
    let lights = [(u, 0, 1, true), ([-u[1], u[0]], 1, 2, false)];
    for &(dir, kind, passes, grow) in &lights[..if octave { 2 } else { 1 }] {
        let (a, b) = spot_line(f.rules.a, mid, dir, span);
        let sid = f.spotlight.next_spot;
        f.spotlight.next_spot += 1;
        f.spotlight.spots.push(Spot {
            owner: k,
            id: sid,
            a,
            b,
            c: [0.0, 0.0],
            t0,
            t1,
            r,
            dmg,
            hit: false,
            kind,
            hot: i64::from(hot),
            passes,
            grow,
        });
        let data = vec![
            Value::Int(sid),
            Value::Float(a[0]),
            Value::Float(a[1]),
            Value::Float(b[0]),
            Value::Float(b[1]),
            Value::Float(t0),
            Value::Float(t1),
            Value::Float(r),
            Value::Int(kind),
        ];
        f.ev("spot", k as i64, data);
    }
}

/// `spot_step(k, dt)` (4471 to 4500), from `run()` 6183 to 6184, every step: ENCORE's chase and end, and the end of
/// each light that has run out (`spot_end`, with whether it burned; a sweep that burned grows the light and the
/// burn).
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    if let Some(en) = f.spotlight.encore[k] {
        let t = f.t;
        // the chase light is in `spots` for as long as its ENCORE runs (only `start_encore` and the loop below
        // remove an owner's lights, and the loop keeps it while `B.encore` is set)
        let i = f.spotlight.spots.iter().position(|sp| sp.id == en.spot);
        if t >= en.t1 {
            f.spotlight.encore[k] = None;
            f.spotlight.phones.clear();
            if let Some(i) = i {
                let sp = &mut f.spotlight.spots[i];
                sp.t1 = t;
                let data = vec![Value::Int(sp.id), Value::Int(i64::from(sp.hit))];
                f.ev("spot_end", k as i64, data);
            }
        } else if let Some(i) = i {
            let d = geom::sub(f.balls[1 - k].p, f.spotlight.spots[i].c);
            let n = geom::hypot(d);
            let st = en.speed * f.rules.v * dt;
            if n > 1e-9 {
                // Sp['c'] + d / n * min(st, n)
                let sp = &mut f.spotlight.spots[i];
                sp.c = geom::add(sp.c, geom::scale(geom::div(d, n), py::py_min(st, n)));
            }
        }
    }
    let spots = std::mem::take(&mut f.spotlight.spots);
    let mut keep = Vec::with_capacity(spots.len());
    for sp in spots {
        if sp.owner == k && sp.kind != 2 && f.t > sp.t1 + 1e-9 {
            if sp.grow && sp.hit {
                let b = &mut f.balls[k];
                b.blade_len = py::py_min(w.max_r, b.blade_len + w.grow);
                f.spotlight.burn[k] = py::py_min(w.dmg_max, f.spotlight.burn[k] + w.dmg_grow);
            }
            f.ev(
                "spot_end",
                k as i64,
                vec![Value::Int(sp.id), Value::Int(i64::from(sp.hit))],
            );
            continue;
        }
        if sp.owner == k && sp.kind == 2 && f.spotlight.encore[k].is_none() && f.t > sp.t1 + 1e-9 {
            continue;
        }
        keep.push(sp);
    }
    f.spotlight.spots = keep;
}

/// `trigger`, kind `octave` (2958 to 2960): `B.oct_until = t + ab_dur(ab)`; event `octave` with that time.
pub fn start_octave(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + ab_dur(f, ab);
    f.spotlight.oct_until[k] = Some(until);
    f.ev("octave", k as i64, vec![Value::Float(until)]);
}

/// `trigger`, kind `altissimo` (2961 to 2963): `B.alt_until, B.alt_ab = t + ab_dur(ab), ab`; event `altissimo` with
/// that time.
pub fn start_altissimo(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + ab_dur(f, ab);
    f.spotlight.alt[k] = Some(Alt {
        until,
        span_k: param(ab, "span_k"),
        narrow: param(ab, "narrow"),
        hot: param(ab, "hot"),
    });
    f.ev("altissimo", k as i64, vec![Value::Float(until)]);
}

/// `trigger`, kind `encore` (2964 to 2965): `start_encore(k, ab)` (4501 to 4512): the fighter's sweeping lights
/// give way to one chase light that starts on the fighter itself, `max_r` x `r_k` wide, for `beats` beats; event
/// `encore` with its end.
///
/// # Panics
/// When fighter `k`'s weapon is not a spotlight (Python raises `KeyError` on `B.w['max_r']`). No other fighter can
/// fire it: `encore` is not one of the kinds IDENTITY CRISIS copies.
pub fn start_encore(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let w = weapon(f, k);
    let sid = f.spotlight.next_spot;
    f.spotlight.next_spot += 1;
    let t1 = f.t + param(ab, "beats") * f.rules.beat;
    let sp = Spot {
        owner: k,
        id: sid,
        a: [0.0, 0.0],
        b: [0.0, 0.0],
        c: f.balls[k].p,
        t0: f.t,
        t1,
        r: w.max_r * ab.get("r_k").unwrap_or(1.0),
        dmg: param(ab, "dmg"),
        hit: false,
        kind: 2,
        hot: 0,
        passes: 1,
        grow: false,
    };
    f.spotlight.spots.retain(|s| s.owner != k);
    f.spotlight.spots.push(sp);
    f.spotlight.encore[k] = Some(Encore {
        n_phone: param(ab, "n_phone"),
        spread: param(ab, "spread"),
        phone_r: param(ab, "phone_r"),
        phone_dmg: param(ab, "phone_dmg"),
        speed: param(ab, "speed"),
        t1,
        spot: sid,
    });
    f.ev("encore", k as i64, vec![Value::Float(t1)]);
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5892 to 5893: `SPOT`, when a fighter
/// has the kind.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) { &["SPOT"] } else { &[] }
}

/// This kind's rows of one frame: `snap`, `SPOT` (5671 to 5681): every light that is on (owner, x, y, radius, kind,
/// its start and end, hot, id), then the phone lights (the spotlight's fighter, x, y, 0.0, 3, the 8th they came on
/// and the next, 0, -1).
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut row = Vec::new();
    for sp in &f.spotlight.spots {
        if sp.t0 - 1e-9 <= f.t && f.t <= sp.t1 + 1e-9 {
            let c = spot_pos(sp, f.t);
            row.push(Value::List(vec![
                Value::Int(sp.owner as i64),
                Value::Float(c[0]),
                Value::Float(c[1]),
                Value::Float(sp.r),
                Value::Int(sp.kind),
                Value::Float(sp.t0),
                Value::Float(sp.t1),
                Value::Int(sp.hot),
                Value::Int(sp.id),
            ]));
        }
    }
    // int(next((j for j, Bj in enumerate((B0, B1)) if Bj.w['kind'] == 'spotlight'), 0))
    let owner = (0..2).find(|&j| is(f, j)).unwrap_or(0);
    for &(t, q) in &f.spotlight.phones {
        row.push(Value::List(vec![
            Value::Int(owner as i64),
            Value::Float(q[0]),
            Value::Float(q[1]),
            Value::Float(0.0),
            Value::Int(3),
            Value::Float(t),
            Value::Float(t + f.rules.beat / 2.0),
            Value::Int(0),
            Value::Int(-1),
        ]));
    }
    out.push_nested("SPOT", Value::List(row));
}

#[cfg(test)]
mod tests;
