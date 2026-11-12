//! Weapon kind `swallow`, TUBA's the bell: `fight.py` 1045 to 1130 (`swallow_setup`, `swallow_beat`, `swallow_step`
//! with the `swallow_rollers` rule at 1100).
//!
//! Every `every` beats (a haste divides it) the bell goes "oom": it inhales for `inhale` beats, and "pah" comes `pah`
//! beats after the oom. While it inhales, an enemy whose surface is within the suction reach (`blade_len`) is steered
//! hard toward the bell (its speed is kept); touching the bell it is swallowed: it rides inside the tuba (`held`: it
//! does not move and does not clash) until the pah spits it out like a cannonball toward the open ring. If it then
//! hits a rope within `spit_t` seconds it takes the cannonball's damage. A gulp that lands lengthens the reach.
//!
//! Python hangs this state on the ball (`B.sw`, `B.cannon`); here it is [`State`], `Fight::swallow`, indexed by
//! fighter. The reach itself is the ball's `blade_len`, the bell's aim its `th`. The generator is drawn once per
//! spit (the jitter of its direction) and never otherwise.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::Fight;
use crate::geom;
use crate::roster::{SwallowCfg, WeaponCfg};

/// The current oom-pah (`B.sw`, the dict `swallow_beat` makes).
#[derive(Clone, Debug, PartialEq)]
pub struct Oom {
    /// When the bell went "oom".
    pub t0: f64,
    /// When the inhale ends (brought forward to now when the gulp meets a shield).
    pub t_in: f64,
    /// When the "pah" comes.
    pub t_pah: f64,
    /// Whether the enemy was gulped.
    pub caught: bool,
    /// Whether the pah has come.
    pub done: bool,
}

/// After a spit (`B.cannon`): until `t_end` a rope hit of the spat enemy is the cannonball.
#[derive(Clone, Debug, PartialEq)]
pub struct Cannon {
    /// When the cannonball is over.
    pub t_end: f64,
    /// The first event not yet looked at.
    pub ev_i: usize,
}

/// Weapon kind `swallow`'s per-fight state (`Fight::swallow`): what `fight.py` hangs on the swallower's ball, indexed
/// by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.sw`: the current oom-pah, if any.
    pub sw: [Option<Oom>; 2],
    /// `B.cannon`: the spat enemy still flying, if any.
    pub cannon: [Option<Cannon>; 2],
}

/// `B.w['kind'] == 'swallow'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Swallow(_))
}

/// The bell's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a swallow.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a SwallowCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Swallow(w) => w,
        other => panic!("fighter {k} ({}) has no swallow weapon: {other:?}", cfg.name),
    }
}

/// `swallow_setup(k)` (1046 to 1052), from `run()` 5765 to 5767 (the same loop as the striker's setup, before it):
/// no oom-pah, no cannonball, and the bell starts facing the enemy.
pub fn setup(f: &mut Fight, k: usize) {
    f.swallow.sw[k] = None;
    f.swallow.cannon[k] = None;
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    // B.th = math.atan2(d[1], d[0]) % (2 * math.pi)
    f.balls[k].th = py::fmod_py(dmath::atan2(d[1], d[0]), 2.0 * dmath::PI);
}

/// `swallow_beat(k, kb)` (1053 to 1063), from `run()` 5936 to 5937: on beat `kb` of the fight, every
/// `max(1, int(round(every / ab_mult(k))))` beats, unless an oom-pah is still running, the bell goes "oom". Event
/// `oom`: when the inhale ends, when the pah comes, the reach.
pub fn beat(f: &mut Fight, k: usize, kb: i64) {
    let w = weapon(f, k);
    let m = f.ab_mult(k);
    // int(round(x)): half to even; a quotient too large for an i64 saturates, and `kb % n` is `kb` either way
    let n = py::round_half_even(w.every / m) as i64;
    let free = f.swallow.sw[k].as_ref().is_none_or(|s| s.done);
    if py::imod_py(kb, n.max(1)) == 0 && free {
        let t = f.t;
        let (t_in, t_pah) = (t + w.inhale * f.rules.beat / m, t + w.pah * f.rules.beat / m);
        f.swallow.sw[k] = Some(Oom {
            t0: t,
            t_in,
            t_pah,
            caught: false,
            done: false,
        });
        let reach = f.balls[k].blade_len;
        let data = vec![Value::Float(t_in), Value::Float(t_pah), Value::Float(reach)];
        f.ev("oom", k as i64, data);
    }
}

/// `swallow_step(k, dt)` (1064 to 1130), from `run()` 6133 to 6134. The bell turns toward the enemy (`aim_rate`).
/// A spat enemy that hits a rope while it flies takes `dmg_rope` (`cannon`). While the bell inhales, an enemy whose
/// surface is within the reach is steered toward the bell (`suck`, its speed kept); within `catch` of touching it is
/// gulped (`gulp`; a shield blocks it and ends the inhale), rides inside until the pah, and is spat toward the open
/// ring (`pah`, `spit`). A rolling enemy (#56) is not gulped unless the rule `swallow_rollers` is on.
///
/// Events: `block` (the shielded enemy, the swallower, x, y, `swallow`), `gulp` (x, y, the new reach), `pah` (0 and
/// the bell's aim when nothing was caught; 1 and the spit's direction).
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let (bp, op) = (f.balls[k].p, f.balls[1 - k].p);
    let rel = geom::sub(op, bp);
    let lim = w.aim_rate * dt;
    // B.th = (B.th + max(-lim, min(lim, _wrap(math.atan2(rel[1], rel[0]) - B.th)))) % (2 * math.pi)
    let th = f.balls[k].th;
    let dth = geom::wrap(dmath::atan2(rel[1], rel[0]) - th);
    f.balls[k].th = py::fmod_py(th + py::py_max(-lim, py::py_min(lim, dth)), 2.0 * dmath::PI);
    if let Some(cannon) = f.swallow.cannon[k].clone() {
        // the spat enemy: a rope hit while it flies is the cannonball
        let rope = f.events[cannon.ev_i..]
            .iter()
            .find(|e| e.kind == "rope" && e.fields[0] == Value::Int(1 - k as i64))
            .map(|e| [float(&e.fields[3]), float(&e.fields[4])]);
        if let Some(at) = rope {
            f.damage(k, 1 - k, w.dmg_rope, "cannon", at, 0);
            f.swallow.cannon[k] = None;
        } else if f.t > cannon.t_end {
            f.swallow.cannon[k] = None;
        } else {
            f.swallow.cannon[k] = Some(Cannon {
                t_end: cannon.t_end,
                ev_i: f.events.len(),
            });
        }
    }
    let Some(s) = f.swallow.sw[k].clone() else {
        return;
    };
    if s.done {
        return;
    }
    let mut s = s;
    if s.caught {
        // inside the tuba
        f.balls[1 - k].p = bp;
    } else if f.t < s.t_in {
        let dist = geom::hypot(rel);
        if dist - 2.0 * r < f.balls[k].blade_len && dist > 1e-6 {
            let ov = f.balls[1 - k].v;
            let sp = geom::norm(ov);
            // v = O.v - rel / dist * w['suck'] * dt
            let pull = geom::scale(geom::scale(geom::div(rel, dist), w.suck), dt);
            let v = geom::sub(ov, pull);
            // O.v = v / (np.linalg.norm(v) + 1e-12) * sp
            f.balls[1 - k].v = geom::scale(geom::div(v, geom::norm(v) + 1e-12), sp);
        }
        // #56: a rolling drum is too heavy to lift, unless the opt-in rule says otherwise
        let rolling = f.balls[1 - k].rolling && !f.rules.swallow_rollers.unwrap_or(false);
        if dist < 2.0 * r + w.catch && !rolling {
            // hx, hy = (B.p + rel / max(dist, 1e-6) * R).tolist()
            let h = geom::add(bp, geom::scale(geom::div(rel, py::py_max(dist, 1e-6)), r));
            if f.balls[1 - k].shield > 0.0 {
                let data = vec![
                    Value::Int(k as i64),
                    Value::Float(h[0]),
                    Value::Float(h[1]),
                    Value::Str("swallow".to_string()),
                ];
                f.ev("block", 1 - k as i64, data);
                s.t_in = f.t;
            } else if f.damage(k, 1 - k, w.dmg, "gulp", h, 0) {
                s.caught = true;
                let o = &mut f.balls[1 - k];
                o.held = true;
                o.p = bp;
                let b = &mut f.balls[k];
                b.blade_len = py::py_min(w.max_reach, b.blade_len + w.grow);
                let reach = b.blade_len;
                let data = vec![Value::Float(h[0]), Value::Float(h[1]), Value::Float(reach)];
                f.ev("gulp", k as i64, data);
            }
        }
    }
    if f.t + 1e-9 >= s.t_pah {
        s.done = true;
        f.swallow.sw[k] = Some(s.clone());
        let th = f.balls[k].th;
        if !s.caught {
            f.ev("pah", k as i64, vec![Value::Int(0), Value::Float(th)]);
            return;
        }
        // toward the open ring: the centre, or straight ahead when the tuba stands on it
        let to_open = if geom::hypot(bp) > 0.3 {
            [-bp[0], -bp[1]]
        } else {
            [dmath::cos(th), dmath::sin(th)]
        };
        let a = dmath::atan2(to_open[1], to_open[0]) + geom::radians(f.rng.uniform(-1.0, 1.0) * w.jitter);
        let d = [dmath::cos(a), dmath::sin(a)];
        let lim = f.rules.a - r - 1e-3;
        // O.p = np.clip(B.p + d * (2 * R + 0.02), -lim_, lim_)
        let out = geom::add(bp, geom::scale(d, 2.0 * r + 0.02));
        let v_shared = f.rules.v;
        let o = &mut f.balls[1 - k];
        o.p = [geom::np_clip(out[0], -lim, lim), geom::np_clip(out[1], -lim, lim)];
        o.v = geom::scale(d, v_shared);
        o.boost = py::py_max(o.boost, w.spit_boost);
        o.held = false;
        let at = o.p;
        f.ev("pah", k as i64, vec![Value::Int(1), Value::Float(a)]);
        f.damage(k, 1 - k, w.dmg_spit, "spit", at, 0);
        f.swallow.cannon[k] = Some(Cannon {
            t_end: f.t + w.spit_t,
            ev_i: f.events.len(),
        });
    } else {
        f.swallow.sw[k] = Some(s);
    }
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

#[cfg(test)]
mod tests;
