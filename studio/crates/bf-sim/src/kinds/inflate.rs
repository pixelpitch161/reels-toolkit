//! Weapon kind `inflate`, BAGPIPES's BAG INFLATE: `fight.py` 1763 to 1867 (`inflate_setup`, `inflate_start`,
//! `inflate_step`, `inflate_clash`) and its branch of `grid16_fire` (1487 to 1507). The per-ball radius, mass and speed
//! it sets are the ball's `rad`, `mass` and `vmul`, which `move` reads through `fight::radius` and `fight::mass`.
//!
//! Every `every` beats (on its own 16th grid; a haste divides it) the bag swells for `inflate_beats` beats: the ball's
//! radius and mass grow to `inf_scale` x and its speed falls to `slow` x. Then it lets go, aimed at the enemy (a seeded
//! wobble), and rockets at `fly_speed` x for the flight's length, zig-zagging (seeded turns at seeded times, alternate
//! sides, a little homing) as it deflates back to the shared radius. A clash while it swells is a body slam (scaled by
//! how swollen it is), while it flies a flight hit; landed hits grow the size and the flight.
//!
//! Python hangs this state on the ball (`B.inf`, `B.inf_scale`, `B.inf_fly`, `B.inf_cool`; `B.rad`, `B.mass`,
//! `B.vmul` are the ball's own fields here); here it is [`State`], `Fight::inflate`, indexed by fighter. The generator
//! is drawn at a release (the aim's wobble, the first zig-zag's time, its side) and at each zig-zag (its turn, the next
//! one's time), in Python's order.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{InflateCfg, WeaponCfg};

/// What a cycle is doing (`C['phase']`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Swelling (`'inflate'`).
    Inflate,
    /// The released balloon's flight (`'fly'`), with the next zig-zag's time and side.
    Fly {
        /// When the flight began (`C['t1']`).
        t1: f64,
        /// When the next zig-zag comes (`C['zig_t']`).
        zig_t: f64,
        /// The side of the last turn (`C['zig_s']`: 1.0 or -1.0).
        zig_s: f64,
    },
}

/// The running cycle (`B.inf`, the dict `inflate_start` makes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cycle {
    /// Swelling or flying.
    pub phase: Phase,
    /// When it started.
    pub t0: f64,
    /// Seconds of the swell.
    pub dur_i: f64,
    /// Seconds of the flight.
    pub dur_f: f64,
    /// The swollen size (x R) of this cycle.
    pub s: f64,
}

/// One inflater's state (what `inflate_setup` hangs on the ball).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bag {
    /// The running cycle (`B.inf`), if any.
    pub inf: Option<Cycle>,
    /// The swollen size a cycle swells to (`B.inf_scale`), +`grow` per landed hit.
    pub inf_scale: f64,
    /// The flight's length in beats (`B.inf_fly`), +`fly_grow` per landed hit.
    pub inf_fly: f64,
    /// Seconds until the next contact hit may land (`B.inf_cool`).
    pub inf_cool: f64,
}

/// This kind's per-fight state (`Fight::inflate`), indexed by fighter: `None` for a fighter that is not an inflater
/// (Python's ball then has none of these attributes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// Each inflater's bag.
    pub bags: [Option<Bag>; 2],
}

/// `B.w['kind'] == 'inflate'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Inflate(_))
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a InflateCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Inflate(w) => w,
        _ => unreachable!("not an inflater"),
    }
}

fn bag<'f>(f: &'f mut Fight, k: usize) -> &'f mut Bag {
    f.inflate.bags[k].as_mut().expect("inflate_setup ran")
}

/// `inflate_setup(k)` (1763 to 1776), from `run()` 5756 to 5762 (the `combo` / `flail` / `inflate` loop): the ball's
/// `rad`, `mass`, `vmul` (the ball keeps them from now on: `hasattr(B, 'mass')` stays true, so every clash of the fight
/// takes `move`'s unequal-mass branch), its bag, and its contact hook (`clash_hooks`, which `move` runs as [`clash`]).
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let b = &mut f.balls[k];
    b.rad = Some(r);
    b.mass = Some(1.0);
    b.vmul = Some(1.0);
    b.blade_len = w.scale0;
    f.inflate.bags[k] = Some(Bag {
        inf: None,
        inf_scale: w.scale0,
        inf_fly: w.fly_beats,
        inf_cool: 0.0,
    });
}

/// Its branch of `grid16_fire(k, k16)` (1487 to 1507: the grid test, then `inflate_start` 1777 to 1791), from `run()`
/// 5975 to 5980 after the caller's stun test.
pub fn grid16_fire(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    // main = max(1, int(round(4 * every))); step16 = max(1, int(round(main / ab_mult(k))))
    let main = (py::round_half_even(4.0 * w.every) as i64).max(1);
    let step16 = (py::round_half_even(main as f64 / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(k16, step16) != 0 {
        return;
    }
    start(f, k);
}

/// `inflate_start(k)` (1777 to 1791): a cycle starts unless one is running (the next start is then skipped).
fn start(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    if bag(f, k).inf.is_some() {
        return;
    }
    let mult = f.ab_mult(k);
    let beat = f.rules.beat;
    let dur_i = w.inflate_beats * beat / mult;
    let fly = py::py_min(bag(f, k).inf_fly, w.every - w.inflate_beats - w.rest_beats);
    let dur_f = py::py_max(0.25, fly) * beat / mult;
    let t = f.t;
    let s = bag(f, k).inf_scale;
    bag(f, k).inf = Some(Cycle {
        phase: Phase::Inflate,
        t0: t,
        dur_i,
        dur_f,
        s,
    });
    f.ev(
        "inflate",
        k as i64,
        vec![Value::Float(s), Value::Float(dur_i), Value::Float(dur_f)],
    );
}

/// `inflate_step(k, dt)` (1792 to 1838), from `run()` 6131 to 6132: the swell (radius, mass up, speed down), the
/// release, the zig-zag flight, deflating back to R.
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let cool = &mut bag(f, k).inf_cool;
    *cool = py::py_max(0.0, *cool - dt);
    let Some(c) = bag(f, k).inf else {
        return;
    };
    let s = c.s;
    match c.phase {
        Phase::Inflate => {
            let u = py::py_min(1.0, (f.t - c.t0) / c.dur_i);
            let e = u * u * (3.0 - 2.0 * u);
            let b = &mut f.balls[k];
            b.rad = Some(r * (1.0 + (s - 1.0) * e));
            b.mass = Some(1.0 + (s - 1.0) * e * w.mass_k);
            b.vmul = Some(1.0 - (1.0 - w.slow) * e);
            if u >= 1.0 {
                let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
                let a = dmath::atan2(d[1], d[0]) + geom::radians(f.rng.uniform(-1.0, 1.0) * w.aim_wobble);
                let v_shared = f.rules.v;
                let b = &mut f.balls[k];
                b.v = geom::scale([dmath::cos(a), dmath::sin(a)], v_shared);
                b.vmul = Some(w.fly_speed);
                let zig_t = f.t + f.rng.uniform(w.zig[0], w.zig[1]);
                let zig_s = if f.rng.random() < 0.5 { 1.0 } else { -1.0 };
                let t1 = f.t;
                if let Some(cy) = bag(f, k).inf.as_mut() {
                    cy.phase = Phase::Fly { t1, zig_t, zig_s };
                }
                let p = f.balls[k].p;
                f.ev(
                    "release",
                    k as i64,
                    vec![
                        Value::Float(s),
                        Value::Float(c.dur_f),
                        Value::Float(p[0]),
                        Value::Float(p[1]),
                    ],
                );
            }
        }
        Phase::Fly { t1, zig_t, zig_s } => {
            let u = py::py_min(1.0, (f.t - t1) / c.dur_f);
            let fl = dmath::pow(1.0 - u, 1.5);
            let b = &mut f.balls[k];
            b.rad = Some(r * (1.0 + (s - 1.0) * fl));
            b.mass = Some(1.0 + (s - 1.0) * fl * w.mass_k);
            b.vmul = Some(w.fly_speed * (1.0 - w.fly_fade * u));
            if f.t >= zig_t && u < 1.0 {
                let zig_s = -zig_s;
                let turn = geom::radians(f.rng.uniform(w.zig_turn[0], w.zig_turn[1])) * zig_s;
                let (c_, s_) = (dmath::cos(turn), dmath::sin(turn));
                let bv = f.balls[k].v;
                let v = geom::div(bv, geom::norm(bv) + 1e-12);
                let v = [c_ * v[0] - s_ * v[1], s_ * v[0] + c_ * v[1]];
                let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
                let d = geom::div(d, geom::norm(d) + 1e-12);
                let hm = w.home;
                let v: V2 = geom::add(geom::scale(v, 1.0 - hm), geom::scale(d, hm));
                let v_shared = f.rules.v;
                f.balls[k].v = geom::scale(geom::div(v, geom::norm(v) + 1e-12), v_shared);
                let zig_t = f.t + f.rng.uniform(w.zig[0], w.zig[1]);
                if let Some(cy) = bag(f, k).inf.as_mut() {
                    cy.phase = Phase::Fly { t1, zig_t, zig_s };
                }
            }
            if u >= 1.0 {
                let b = &mut f.balls[k];
                b.rad = Some(r);
                b.mass = Some(1.0);
                b.vmul = Some(1.0);
                bag(f, k).inf = None;
                let p = f.balls[k].p;
                f.ev("land", k as i64, vec![Value::Float(p[0]), Value::Float(p[1])]);
            }
        }
    }
}

/// `inflate_clash(k)` (1839 to 1867): the contact hook `move` runs after a clash (`clash_hooks`, 3233 to 3234), once
/// per inflater in the order `inflate_setup` added them: a body slam while swollen enough, a flight hit while it
/// rockets; a landed hit grows the size and the flight.
pub fn clash(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let Some(c) = bag(f, k).inf else {
        return;
    };
    if bag(f, k).inf_cool > 0.0 {
        return;
    }
    let r = f.rules.r;
    let s = c.s;
    let rad = crate::fight::radius(f, k);
    let (dmg, how, kn) = match c.phase {
        Phase::Inflate => {
            let big = (rad / r - 1.0) / py::py_max(1e-6, s - 1.0);
            if big < w.slam_min {
                return;
            }
            (w.slam_dmg * big, "bodyslam", w.knock * 0.8)
        }
        Phase::Fly { .. } => (w.fly_dmg, "flight", w.knock),
    };
    let p = f.balls[k].p;
    let rel = geom::sub(f.balls[1 - k].p, p);
    let dist = geom::hypot(rel);
    // (B.p + rel / max(dist, 1e-6) * B.rad).tolist()
    let at = geom::add(p, geom::scale(geom::div(rel, py::py_max(dist, 1e-6)), rad));
    let landed = f.damage(k, 1 - k, dmg, how, at, 0);
    f.knock(1 - k, rel, kn);
    bag(f, k).inf_cool = w.cool;
    if landed {
        let b = bag(f, k);
        b.inf_scale = py::py_min(w.scale_max, b.inf_scale + w.grow);
        b.inf_fly = py::py_min(w.fly_max, b.inf_fly + w.fly_grow);
        let scale = b.inf_scale;
        f.balls[k].blade_len = scale;
    }
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5763 to 5764: `RAD`, when a fighter
/// inflates.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if is(f, 0) || is(f, 1) { &["RAD"] } else { &[] }
}

/// This kind's rows of one frame: `snap`, `RAD` (5520 to 5521: `getattr(B, 'rad', R)` of both balls), into `out`.
/// `RAD` is not one of the kit's array keys (`story.REC_KEYS`), so it is a ragged list in a recording: each frame's
/// `[rad0, rad1]`.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let row = [crate::fight::radius(f, 0), crate::fight::radius(f, 1)];
    out.push_nested("RAD", Value::List(row.into_iter().map(Value::Float).collect()));
}
