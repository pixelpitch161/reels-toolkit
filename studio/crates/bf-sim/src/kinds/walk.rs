//! Weapon kind `walk`, CONTRABASS's WALKING BASS: `fight.py` 3965 to 4040 (`walk_setup`, `walk_blocked`, `walk_tick`,
//! `walk_step`, `_step_land`), and the walker's side of the clash in `move` (3190 to 3208, 3227 to 3231).
//!
//! The double bass does not glide: it walks. Every `every` beats on the 16th grid (haste divides it: DOUBLE TIME walks
//! on the 8ths) it takes one heavy step of `step16` 16ths, timed so the footfall lands on the grid, toward the enemy's
//! spot `lead` x its motion over the step ahead (turned by up to `jitter` degrees, seeded), `stride` long at most and
//! never past contact (`gap`). While a step is under way the ball is walking: nothing knocks it, no grapple holds it,
//! and in a clash it gives no way (planted, it gives none either: it is a walker). Each footfall is a floor-quake: a
//! ring of kind `step` from the landing spot out to `blade_len` beyond its surface, which grows by `grow` per landed
//! quake up to `max_reach`. Between steps it stands planted and only slides when knocked; it takes no step while its
//! knock boost is above `stagger`, while held, stunned, in pieces or stampeding.
//!
//! Python hangs the step on the ball: `B.wstep` is [`State::wstep`] (`Fight::walk`), by fighter. The quake's reach
//! is the ball's `blade_len`.
//!
//! Draws from the fight's generator, in Python's order: one `uniform(-1, 1)` per step taken (the jitter, 4008).

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::{Fight, Wave, stunned};
use crate::geom::{self, V2, py_max, py_min};
use crate::record::Frames;
use crate::roster::{WalkCfg, WeaponCfg};

#[cfg(test)]
mod kit_vectors;

/// A step under way (`B.wstep`, `walk_tick` 4014): from `p0` to `p1`, from `t0` to `t1`.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// Where the step started.
    pub p0: V2,
    /// Where it lands.
    pub p1: V2,
    /// When it started.
    pub t0: f64,
    /// When it lands, on the grid.
    pub t1: f64,
    /// 1 when it started under haste (DOUBLE TIME), else 0.
    pub kind: i64,
}

/// Weapon kind `walk`'s per-fight state (`Fight::walk`): what `fight.py` hangs on the ball for this kind, by fighter.
/// `None` for a fighter that is not a walking bass, and between its steps.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.wstep`.
    pub wstep: [Option<Step>; 2],
}

/// `B.w['kind'] == 'walk'`: fighter `k`'s weapon is this kind. It is also Python's `getattr(B, 'wstep', ...)` having
/// a ball to look at: only `walk_setup` gives a ball `wstep`.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Walk(_))
}

/// The walking bass's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k` is not a walking bass.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a WalkCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Walk(w) => w,
        other => panic!("fighter {k} ({}) is not a walking bass: {other:?}", cfg.name),
    }
}

/// `walk_setup(k)` (3977 to 3982), from `run()` 5876 to 5878: a walker, planted, no step, the quake at its `base`
/// reach.
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let b = &mut f.balls[k];
    b.walker = true;
    b.walking = false;
    b.blade_len = w.base;
    f.walk.wstep[k] = None;
}

/// `walk_blocked(k)` (3984 to 3988): stunned (or sunk in shadow), held in an embrace, in pieces, or stampeding.
pub fn blocked(f: &Fight, k: usize) -> bool {
    stunned(f, k) || f.balls[k].grabbed || f.balls[k].smashed > 0.0 || f.stampede.stamp[k].is_some()
}

/// `max(1, int(round(x / m)))`: a count of 16ths under the haste factor `m`, at least 1.
fn sixteenths(x: f64, m: f64) -> i64 {
    (py::round_half_even(x / m) as i64).max(1)
}

/// `walk_tick(k, k16)` (3990 to 4016), from `run()` 6065 to 6069, on every 16th `k16` of the fight.
///
/// A step that lands within the next substep lands now ([`land`]). Then, unless a step is under way, the ball is
/// blocked or still sliding from a knock above `stagger`, a step starts when its footfall would land on the walk's
/// grid (`step`: from, to, landing time, 1 under haste).
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    if let Some(s) = &f.walk.wstep[k]
        && f.t + f.rules.dt + 1e-9 >= s.t1
    {
        // due within a substep: land on the grid
        land(f, k);
    }
    if f.walk.wstep[k].is_some() || blocked(f, k) || f.balls[k].boost > w.stagger {
        return;
    }
    let m = f.ab_mult(k);
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    let st16 = sixteenths(w.step16, m);
    // if (k16 + st16) % max(1, int(round(main16 / m))): the footfall lands ON the grid (it leaves st16 before)
    if py::imod_py(k16 + st16, sixteenths(main16 as f64, m)) != 0 {
        return;
    }
    let dur = st16 as f64 * f.rules.beat / 4.0;
    let (bp, op, ov) = (f.balls[k].p, f.balls[1 - k].p, f.balls[1 - k].v);
    // tgt = O.p + O.v * dur * w.get('lead', 0.0)
    let tgt = [op[0] + ov[0] * dur * w.lead, op[1] + ov[1] * dur * w.lead];
    let d = geom::sub(tgt, bp);
    let dist = geom::hypot(d);
    let u = if dist > 1e-6 { geom::div(d, dist) } else { [1.0, 0.0] };
    let j = geom::radians(f.rng.uniform(-1.0, 1.0) * w.jitter);
    let (cj, sj) = (dmath::cos(j), dmath::sin(j));
    let u = [u[0] * cj - u[1] * sj, u[0] * sj + u[1] * cj];
    let ln = py_min(w.stride, py_max(0.0, dist - 2.0 * r - w.gap));
    let lim = f.rules.a - r;
    let p1 = [
        geom::np_clip(bp[0] + u[0] * ln, -lim, lim),
        geom::np_clip(bp[1] + u[1] * ln, -lim, lim),
    ];
    // its heading
    f.balls[k].v = geom::scale(u, f.rules.v);
    let t1 = f.t + dur;
    let kind = i64::from(m > 1.0);
    f.walk.wstep[k] = Some(Step {
        p0: bp,
        p1,
        t0: f.t,
        t1,
        kind,
    });
    f.balls[k].walking = true;
    let data = vec![
        Value::Float(bp[0]),
        Value::Float(bp[1]),
        Value::Float(p1[0]),
        Value::Float(p1[1]),
        Value::Float(t1),
        Value::Int(kind),
    ];
    f.ev("step", k as i64, data);
}

/// `walk_step(k, dt)` (4018 to 4028), from `run()` 6173 to 6174: a step under way lands when its time comes, else the
/// ball swings along it (`u * u * (3 - 2 * u)`: lift, swing, plant).
pub fn step(f: &mut Fight, k: usize, _dt: f64) {
    let Some(s) = &f.walk.wstep[k] else {
        return;
    };
    if f.t + 1e-9 >= s.t1 {
        land(f, k);
        return;
    }
    let u = (f.t - s.t0) / py_max(1e-9, s.t1 - s.t0);
    let e = u * u * (3.0 - 2.0 * u);
    f.balls[k].p = [s.p0[0] + (s.p1[0] - s.p0[0]) * e, s.p0[1] + (s.p1[1] - s.p0[1]) * e];
}

/// `_step_land(k)` (4030 to 4039): the ball is planted where the step lands and a quake ring of kind `step` leaves the
/// spot, out to `blade_len` beyond its surface; it grows the reach when it lands and clears no projectile. Event
/// (`step_land`: where, the reach, 1 under haste).
fn land(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let s = f.walk.wstep[k].take().expect("a step is under way");
    let r = f.rules.r;
    let b = &mut f.balls[k];
    b.p = s.p1;
    b.walking = false;
    let (p, reach) = (b.p, b.blade_len);
    f.waves.push(Wave {
        owner: k,
        c: p,
        r,
        speed: w.speed,
        rmax: r + reach,
        dmg: w.dmg,
        knock: w.knock,
        kind: "step".to_string(),
        hit: false,
        name: w.name.clone(),
        t0: f.t,
        slow: None,
        stun: None,
        detune: None,
        grow: true,
        clears: false,
    });
    let data = vec![
        Value::Float(p[0]),
        Value::Float(p[1]),
        Value::Float(reach),
        Value::Int(s.kind),
    ];
    f.ev("step_land", k as i64, data);
}

/// `move` 3203 to 3205: a walker pushed back off the ropes in a clash has its step (`wstep`, if any) restarted where it
/// now is: `S_['p0'], S_['p1'] = W_.p.copy(), W_.p.copy()`. A walking bass standing between steps (`wstep` None) is
/// left as it is.
pub fn clash_pinned(f: &mut Fight, w: usize) {
    let p = f.balls[w].p;
    if let Some(s) = &mut f.walk.wstep[w] {
        s.p0 = p;
        s.p1 = p;
    }
}

/// `effects_step` 3078 to 3082, a ring with `grow` (the walk's step ring, 4036 to 4038) that landed: `Bw.blade_len =
/// min(Bw.w['max_reach'], Bw.blade_len + Bw.w['grow'])` for the ring's owner. (`pull_grow`: a walk entry never holds
/// it, the roster refuses the key.)
pub fn ring_grow(f: &mut Fight, owner: usize) {
    let w = weapon(f, owner);
    let b = &mut f.balls[owner];
    b.blade_len = py_min(w.max_reach, b.blade_len + w.grow);
}

/// The record keys this kind adds (`run()` 5880 to 5881): `WALK`, when a fighter's weapon is a walk.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if is(f, 0) || is(f, 1) { &["WALK"] } else { &[] }
}

/// This kind's rows of one frame: `snap`, `WALK` (5634 to 5642), a `(2, 4)` float row, -1.0 where nothing is live:
/// the step's progress, start and landing time while one is under way, and the reach of any walker (a marching
/// sousaphone, a rolling bass drum and a strutting tenor are walkers too). `studio/docs/recording-format.md` lists
/// the columns.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut wk = [-1.0; 8];
    for j in 0..2 {
        let row = &mut wk[j * 4..(j + 1) * 4];
        if let Some(s) = &f.walk.wstep[j] {
            row[0] = (f.t - s.t0) / py_max(1e-9, s.t1 - s.t0);
            row[1] = s.t0;
            row[2] = s.t1;
        }
        if f.balls[j].walker {
            row[3] = f.balls[j].blade_len;
        }
    }
    out.push_f64("WALK", &[2, 4], &wk);
}
