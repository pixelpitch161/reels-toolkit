//! Weapon kind `foghorn`, CONTRABASS CLARINET's FOGHORN: the first kind made in the Studio (no `fight.py` original;
//! design `docs/studio/2026-11-16-contrabass-clarinet-design.md`).
//!
//! The bell turns toward the enemy at `aim_rate`. Every `every` beats (a haste divides it) it blows: a cone `spread`
//! degrees each side of the bell's aim, locked for the blast, reaching `blade_len` beyond the two balls' surfaces and
//! live for `dur` seconds. The first time the enemy is inside a blast it is hit (`dmg`, how `foghorn`) and shoved
//! along it (a knock with boost `push`); for the rest of the blast it is steered along it (`steer`). If the shoved
//! enemy hits a rope within `slam_t` seconds it takes the wall slam (`dmg_wall`, how `wall`), and the reach grows by
//! `grow` up to `max_reach`. A shield blocks the blast (the engine's `block`), and a blocked blast shoves nothing.
//!
//! Events: `horn` at a blast's start (aim, reach, half-angle in radians, seconds), the `hit`s above. None of this
//! code draws from the generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::Fight;
use crate::geom;
use crate::roster::{FoghornCfg, WeaponCfg};

/// A live blast.
#[derive(Clone, Debug, PartialEq)]
pub struct Blast {
    /// When it ends.
    pub t_end: f64,
    /// Its direction, radians (the bell's aim when it started).
    pub aim: f64,
    /// Its reach beyond the two surfaces.
    pub reach: f64,
    /// Whether it has touched the enemy (it hits once).
    pub touched: bool,
}

/// After a shove: until `t_end` a rope hit of the shoved enemy is the wall slam.
#[derive(Clone, Debug, PartialEq)]
pub struct Slam {
    /// When the window closes.
    pub t_end: f64,
    /// The first event not yet looked at.
    pub ev_i: usize,
}

/// Weapon kind `foghorn`'s per-fight state (`Fight::foghorn`), indexed by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The live blast, if any.
    pub blast: [Option<Blast>; 2],
    /// The open wall-slam window, if any.
    pub slam: [Option<Slam>; 2],
}

/// Fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Foghorn(_))
}

/// The foghorn's settings.
///
/// # Panics
/// When fighter `k`'s weapon is not a foghorn.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a FoghornCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Foghorn(w) => w,
        other => panic!("fighter {k} ({}) has no foghorn weapon: {other:?}", cfg.name),
    }
}

/// No blast, no window, and the bell starts facing the enemy.
pub fn setup(f: &mut Fight, k: usize) {
    f.foghorn.blast[k] = None;
    f.foghorn.slam[k] = None;
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    f.balls[k].th = py::fmod_py(dmath::atan2(d[1], d[0]), 2.0 * dmath::PI);
}

/// On beat `kb` of the fight, every `max(1, round(every / ab_mult))` beats, unless a blast is still live, the bell
/// blows. Event `horn`: the aim, the reach, the half-angle and the length.
pub fn beat(f: &mut Fight, k: usize, kb: i64) {
    let w = weapon(f, k);
    let n = py::round_half_even(w.every / f.ab_mult(k)) as i64;
    if py::imod_py(kb, n.max(1)) != 0 || f.foghorn.blast[k].is_some() {
        return;
    }
    let (aim, reach) = (f.balls[k].th, f.balls[k].blade_len);
    let half = geom::radians(w.spread);
    f.foghorn.blast[k] = Some(Blast {
        t_end: f.t + w.dur,
        aim,
        reach,
        touched: false,
    });
    let data = vec![
        Value::Float(aim),
        Value::Float(reach),
        Value::Float(half),
        Value::Float(w.dur),
    ];
    f.ev("horn", k as i64, data);
}

/// The bell's aim, the wall slam, and the live blast's cone: the hit and shove on its first touch, the steer after.
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let (bp, op) = (f.balls[k].p, f.balls[1 - k].p);
    let rel = geom::sub(op, bp);
    if f.foghorn.blast[k].is_none() {
        // the bell turns toward the enemy between blasts
        let lim = w.aim_rate * dt;
        let th = f.balls[k].th;
        let dth = geom::wrap(dmath::atan2(rel[1], rel[0]) - th);
        f.balls[k].th = py::fmod_py(th + py::py_max(-lim, py::py_min(lim, dth)), 2.0 * dmath::PI);
    }
    if let Some(slam) = f.foghorn.slam[k].clone() {
        let rope = f.events[slam.ev_i..]
            .iter()
            .find(|e| e.kind == "rope" && e.fields[0] == Value::Int(1 - k as i64))
            .map(|e| [float(&e.fields[3]), float(&e.fields[4])]);
        if let Some(at) = rope {
            f.foghorn.slam[k] = None;
            if f.damage(k, 1 - k, w.dmg_wall, "wall", at, 0) {
                let b = &mut f.balls[k];
                b.blade_len = py::py_min(w.max_reach, b.blade_len + w.grow);
            }
        } else if f.t > slam.t_end {
            f.foghorn.slam[k] = None;
        } else {
            f.foghorn.slam[k] = Some(Slam {
                t_end: slam.t_end,
                ev_i: f.events.len(),
            });
        }
    }
    let Some(mut blast) = f.foghorn.blast[k].clone() else {
        return;
    };
    if f.t + 1e-9 >= blast.t_end {
        f.foghorn.blast[k] = None;
        return;
    }
    let dist = geom::hypot(rel);
    let off = geom::wrap(dmath::atan2(rel[1], rel[0]) - blast.aim).abs();
    let inside = dist > 1e-6
        && dist - 2.0 * r < blast.reach
        && off <= geom::radians(w.spread)
        && !f.balls[1 - k].held;
    if inside {
        let along = [dmath::cos(blast.aim), dmath::sin(blast.aim)];
        if !blast.touched {
            blast.touched = true;
            let h = geom::sub(op, geom::scale(geom::div(rel, dist), r));
            if f.damage(k, 1 - k, w.dmg, "foghorn", h, 0) {
                // shoved along the blast, a little out from the bell's line
                let away = geom::add(along, geom::scale(geom::div(rel, dist), 0.5));
                f.knock(1 - k, away, w.push);
                f.foghorn.slam[k] = Some(Slam {
                    t_end: f.t + w.slam_t,
                    ev_i: f.events.len(),
                });
            }
        } else if f.balls[1 - k].shield <= 0.0 && f.foghorn.slam[k].is_some() {
            // the blast keeps pushing: steer the heading along it, the speed kept
            let ov = f.balls[1 - k].v;
            let sp = geom::norm(ov);
            let v = geom::add(ov, geom::scale(along, w.steer * dt));
            f.balls[1 - k].v = geom::scale(geom::div(v, geom::norm(v) + 1e-12), sp);
        }
    }
    f.foghorn.blast[k] = Some(blast);
}

/// A float field of an event (a rope event's `x` and `y`).
///
/// # Panics
/// When the field is not a float.
fn float(v: &Value) -> f64 {
    match v {
        Value::Float(x) => *x,
        other => panic!("a rope event's position is a float, not {other:?}"),
    }
}
