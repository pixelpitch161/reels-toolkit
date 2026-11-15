//! Ability kind `clamp` (ACCORDION's FULL BELLOWS): `fight.py` `trigger` 2876 to 2877; 1905 to 1926 (`clamp_start`,
//! `clamps_step`).
//!
//! The bellows stretch across the whole ring along the line to the enemy for `tele` beats, then clamp: a ring-wide
//! bite that lands unless the enemy is shielded, dragging it toward the accordion. A copied ability runs this code for
//! the copier's ball, so nothing here assumes which side has the kind.

use bf_core::dmath;
use bf_core::event::Value;

use crate::abilities::param;
use crate::fight::Fight;
use crate::geom;
use crate::roster::AbilityCfg;

/// One entry of `self.clamps` (`clamp_start`'s dict).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clamp {
    /// The fighter who called it.
    pub owner: usize,
    /// When it was called.
    pub t0: f64,
    /// When it clamps.
    pub t_snap: f64,
    /// The bellows' axis (radians, toward the enemy at the call).
    pub axis: f64,
    /// Its damage.
    pub dmg: f64,
    /// Its pull toward the accordion.
    pub knock: f64,
    /// Whether it has clamped.
    pub done: bool,
}

/// This kind's per-fight state (`Fight::clamp`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.clamps`: the clamps called and not yet snapped (`clamp_start`'s dicts).
    pub clamps: Vec<Clamp>,
}

/// Whether `self.clamps` holds anything (Python's truth test of the list).
pub fn live(f: &Fight) -> bool {
    !f.clamp.clamps.is_empty()
}

/// `trigger`, kind `clamp` (2876 to 2877): `clamp_start(k, ab)` (1905 to 1914).
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let bp = f.balls[k].p;
    let d = geom::sub(f.balls[1 - k].p, bp);
    let axis = dmath::atan2(d[1], d[0]);
    let t_snap = f.t + param(ab, "tele") * f.rules.beat;
    f.clamp.clamps.push(Clamp {
        owner: k,
        t0: f.t,
        t_snap,
        axis,
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        done: false,
    });
    let lead = t_snap - f.t;
    f.ev(
        "clamp",
        k as i64,
        vec![
            Value::Float(axis),
            Value::Float(lead),
            Value::Float(bp[0]),
            Value::Float(bp[1]),
        ],
    );
}

/// `clamps_step()` (1915 to 1926), from `run()` 6201 to 6202 (`if self.clamps`).
pub fn clamps_step(f: &mut Fight) {
    for i in 0..f.clamp.clamps.len() {
        let c = &f.clamp.clamps[i];
        if c.done || f.t + 1e-9 < c.t_snap {
            continue;
        }
        let (k, dmg, knock) = (c.owner, c.dmg, c.knock);
        f.clamp.clamps[i].done = true;
        let op = f.balls[1 - k].p;
        let landed = f.damage(k, 1 - k, dmg, "clamp", op, 0);
        let away = geom::sub(f.balls[k].p, op);
        f.knock(1 - k, away, knock);
        f.ev(
            "clamp_snap",
            k as i64,
            vec![Value::Int(i64::from(landed)), Value::Float(op[0]), Value::Float(op[1])],
        );
    }
    f.clamp.clamps.retain(|c| !c.done);
}
