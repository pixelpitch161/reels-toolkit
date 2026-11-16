//! Weapon kind `wingbeat`, FLUGELHORN's WINGBEAT, and its FLANKING CALL (`flank`) and TAKE WING (`take_wing`): a
//! Studio kind (no `fight.py` original; design `docs/studio/2026-11-16-flugelhorn-design.md`).
//!
//! The aim turns toward the enemy at `aim_rate`. Every `every` beats (a haste divides it) the wings clap shut in
//! front: an enemy within `blade_len` beyond the two surfaces and `spread` degrees of the aim is hit (`dmg`, how
//! `wingbeat`) and blown back along the aim (knock `knock`); a landed clap grows the reach by `grow` up to
//! `max_reach`. A shield blocks it (the engine's `block`).
//!
//! FLANKING CALL steers the ball round to a point `side` units beside the enemy (on the side it is already nearer)
//! with a speed boost `boost`, for its `dur`; within `reach` of the surfaces it claps from there (`dmg`, how `flank`,
//! a sideways knock `knock`) and the swoop ends. TAKE WING spreads the wings for `tele` beats, then claps everything
//! within `radius` (`dmg`, how `take_wing`, knock `knock`) and hastes the ball (`mult`, its `dur`). Both work for
//! either ball, so a copied one works too.
//!
//! Events: `clap` on every clap (aim, reach, half-angle in radians), `flank` when a swoop starts (its side, +1 or -1,
//! and its end time), `take_wing` when the wings spread (the clap's time and radius), `wing_clap` when the giant
//! clap lands (radius). None of this code draws from the generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::{ab_dur, param};
use crate::fight::Fight;
use crate::geom;
use crate::roster::{AbilityCfg, WeaponCfg, WingbeatCfg};

/// A FLANKING CALL swoop under way.
#[derive(Clone, Debug, PartialEq)]
pub struct Flank {
    /// When it ends if it never gets there.
    pub t_end: f64,
    /// Which side of the enemy it goes for: +1 to the left of the line from the enemy to the ball, -1 to the right.
    pub side: f64,
    /// How far beside the enemy it aims.
    pub off: f64,
    /// Its speed boost.
    pub boost: f64,
    /// Within this of the surfaces it claps.
    pub reach: f64,
    /// The clap's damage and knock.
    pub dmg: f64,
    /// The clap's knock.
    pub knock: f64,
}

/// A TAKE WING under way: the wings are spread until `t_clap`.
#[derive(Clone, Debug, PartialEq)]
pub struct Take {
    /// When the giant clap lands.
    pub t_clap: f64,
    /// Its radius from the ball's centre.
    pub radius: f64,
    /// Its damage.
    pub dmg: f64,
    /// Its knock.
    pub knock: f64,
    /// The glide's speed (a haste's multiplier).
    pub mult: f64,
    /// The glide's length, seconds.
    pub dur: f64,
}

/// The per-fight state (`Fight::wingbeat`), indexed by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// A FLANKING CALL under way, if any.
    pub flank: [Option<Flank>; 2],
    /// A TAKE WING under way, if any.
    pub take: [Option<Take>; 2],
}

/// Fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Wingbeat(_))
}

/// The wingbeat's settings.
///
/// # Panics
/// When fighter `k`'s weapon is not a wingbeat.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a WingbeatCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Wingbeat(w) => w,
        other => panic!("fighter {k} ({}) has no wingbeat weapon: {other:?}", cfg.name),
    }
}

/// The aim starts facing the enemy.
pub fn setup(f: &mut Fight, k: usize) {
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    f.balls[k].th = py::fmod_py(dmath::atan2(d[1], d[0]), 2.0 * dmath::PI);
}

/// On beat `kb` of the fight, every `max(1, round(every / ab_mult))` beats, the wings clap.
pub fn beat(f: &mut Fight, k: usize, kb: i64) {
    let w = weapon(f, k);
    let n = py::round_half_even(w.every / f.ab_mult(k)) as i64;
    if py::imod_py(kb, n.max(1)) != 0 {
        return;
    }
    let r = f.rules.r;
    let (aim, reach) = (f.balls[k].th, f.balls[k].blade_len);
    let half = geom::radians(w.spread);
    f.ev(
        "clap",
        k as i64,
        vec![Value::Float(aim), Value::Float(reach), Value::Float(half)],
    );
    let rel = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let dist = geom::hypot(rel);
    let off = geom::wrap(dmath::atan2(rel[1], rel[0]) - aim).abs();
    if dist < 1e-6 || dist - 2.0 * r > reach || off > half || f.balls[1 - k].held {
        return;
    }
    let u = geom::div(rel, dist);
    let at = geom::sub(f.balls[1 - k].p, geom::scale(u, r));
    if f.damage(k, 1 - k, w.dmg, "wingbeat", at, 0) {
        let along = [dmath::cos(aim), dmath::sin(aim)];
        f.knock(1 - k, geom::add(along, u), w.knock);
        let b = &mut f.balls[k];
        b.blade_len = py::py_min(w.max_reach, b.blade_len + w.grow);
    }
}

/// The aim turns toward the enemy.
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    let rel = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let lim = w.aim_rate * dt;
    let th = f.balls[k].th;
    let dth = geom::wrap(dmath::atan2(rel[1], rel[0]) - th);
    f.balls[k].th = py::fmod_py(th + py::py_max(-lim, py::py_min(lim, dth)), 2.0 * dmath::PI);
}

/// FLANKING CALL's trigger: the swoop goes for the side of the enemy the ball is already on.
pub fn start_flank(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let (bp, op) = (f.balls[k].p, f.balls[1 - k].p);
    let ov = f.balls[1 - k].v;
    // the side of the enemy's heading the ball is on: it comes round from there
    let cross = ov[0] * (bp[1] - op[1]) - ov[1] * (bp[0] - op[0]);
    let side = if cross >= 0.0 { 1.0 } else { -1.0 };
    let fl = Flank {
        t_end: f.t + ab_dur(f, ab),
        side,
        off: param(ab, "side"),
        boost: param(ab, "boost"),
        reach: param(ab, "reach"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
    };
    f.ev("flank", k as i64, vec![Value::Float(side), Value::Float(fl.t_end)]);
    f.wingbeat.flank[k] = Some(fl);
}

/// TAKE WING's trigger: the wings spread, the clap comes `tele` beats later.
pub fn start_take_wing(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let tk = Take {
        t_clap: f.t + param(ab, "tele") * f.rules.beat,
        radius: param(ab, "radius"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        mult: param(ab, "mult"),
        dur: ab_dur(f, ab),
    };
    f.ev(
        "take_wing",
        k as i64,
        vec![Value::Float(tk.t_clap), Value::Float(tk.radius)],
    );
    f.wingbeat.take[k] = Some(tk);
}

/// Whether either ball has a swoop or a TAKE WING under way.
pub fn abilities_live(f: &Fight) -> bool {
    f.wingbeat.flank.iter().any(Option::is_some) || f.wingbeat.take.iter().any(Option::is_some)
}

/// The swoops and the giant claps, for either ball.
pub fn abilities_step(f: &mut Fight, dt: f64) {
    let r = f.rules.r;
    for k in 0..2 {
        if let Some(fl) = f.wingbeat.flank[k].clone() {
            let (bp, op) = (f.balls[k].p, f.balls[1 - k].p);
            let rel = geom::sub(op, bp);
            let dist = geom::hypot(rel);
            if f.t + 1e-9 >= fl.t_end || dist < 1e-6 {
                f.wingbeat.flank[k] = None;
            } else if dist - 2.0 * r <= fl.reach {
                // there: the clap from the side
                f.wingbeat.flank[k] = None;
                let u = geom::div(rel, dist);
                let at = geom::sub(op, geom::scale(u, r));
                if f.damage(k, 1 - k, fl.dmg, "flank", at, 0) {
                    let sideways = [-u[1] * fl.side, u[0] * fl.side];
                    f.knock(1 - k, geom::add(sideways, u), fl.knock);
                }
            } else {
                // steer toward the point beside the enemy, at the shared speed with the boost
                let u = geom::div(rel, dist);
                let beside = [op[0] + u[1] * fl.side * fl.off, op[1] - u[0] * fl.side * fl.off];
                let to = geom::sub(beside, bp);
                let n = geom::norm(to);
                if n > 1e-9 {
                    let sp = geom::norm(f.balls[k].v);
                    let want = geom::scale(geom::div(to, n), sp);
                    let mix = py::py_min(1.0, 10.0 * dt);
                    let v = geom::add(geom::scale(f.balls[k].v, 1.0 - mix), geom::scale(want, mix));
                    f.balls[k].v = geom::scale(geom::div(v, geom::norm(v) + 1e-12), sp);
                }
                let b = &mut f.balls[k];
                b.boost = py::py_max(b.boost, fl.boost);
            }
        }
        if let Some(tk) = f.wingbeat.take[k].clone() {
            if f.t + 1e-9 < tk.t_clap {
                continue;
            }
            f.wingbeat.take[k] = None;
            f.ev("wing_clap", k as i64, vec![Value::Float(tk.radius)]);
            let rel = geom::sub(f.balls[1 - k].p, f.balls[k].p);
            let dist = geom::hypot(rel);
            if dist <= tk.radius && dist > 1e-6 {
                let u = geom::div(rel, dist);
                let at = geom::sub(f.balls[1 - k].p, geom::scale(u, r));
                if f.damage(k, 1 - k, tk.dmg, "take_wing", at, 0) {
                    f.knock(1 - k, u, tk.knock);
                }
            }
            let b = &mut f.balls[k];
            b.haste = tk.dur;
            b.haste_mult = tk.mult;
        }
    }
}
