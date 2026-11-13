//! Ability kind `lights_out` (BASS CLARINET's LIGHTS OUT): `fight.py` `trigger` 2895 to 2896; 2767 to 2800
//! (`start_lights_out`, `lights_out_step`).
//!
//! For the ability's duration the fighter sinks into shadow (`Ball::shadow`, the seconds left): it cannot be hit,
//! clashes with nothing and starts no attack (`damage`, `move`, `_stunned`), and before every `move` it is steered
//! toward the enemy, turning at most `turn` radians a second with a boost of at least `home` - 1. A whirl in
//! progress is pulled first. At the end it erupts where it is: an enemy within `radius` plus 0.6 of a ball radius
//! takes `dmg` and is knocked away by `knock`. Events `lights_out` (the owner, the time of the eruption) and `erupt`
//! (the owner, where, hit 0 / 1); the eruption's `hit` is of kind `erupt`. Recorded as `LO` (each ball's seconds of
//! shadow left) when a fighter has the kind.
//!
//! Everything is about "ball `k`" and its enemy `1 - k`: [`start`] keeps a copy of the ability it was given (the
//! fighter's own, or one IDENTITY CRISIS copied) in ball `k`'s [`Lo`], and [`lights_out_step`] reads only that copy.
//! None of this code draws from the fight's generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::{ab_dur, has, param};
use crate::fight::Fight;
use crate::geom;
use crate::kinds::anchor;
use crate::record::Frames;
use crate::roster::{AbilityCfg, AbilityKind};

/// `B.lo`: `dict(ab=ab, t0=self.t, t_end=self.t + self.ab_dur(ab))`.
#[derive(Clone, Debug, PartialEq)]
pub struct Lo {
    /// The ability that started it (its `turn`, `home`, `radius`, `dmg`, `knock`).
    pub ab: AbilityCfg,
    /// When it sank.
    pub t0: f64,
    /// When it erupts.
    pub t_end: f64,
}

/// This ability's per-fight state (`Fight::lights_out`): each ball's `B.lo` (`None`: not in shadow).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// By ball.
    pub balls: [Option<Lo>; 2],
}

/// `trigger`, kind `lights_out`: `trigger` 2895 to 2896, `start_lights_out(k, ab)` (2768 to 2774): ball `k` pulls its
/// peg if it whirls, and sinks.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    // a whirl in progress: pull the peg first
    if f.balls[k].anc.is_some() {
        anchor::anchor_release(f, k);
    }
    let t_end = f.t + ab_dur(f, ab);
    f.lights_out.balls[k] = Some(Lo {
        ab: ab.clone(),
        t0: f.t,
        t_end,
    });
    // B.shadow = B.lo['t_end'] - self.t
    f.balls[k].shadow = t_end - f.t;
    f.ev("lights_out", k as i64, vec![Value::Float(t_end)]);
}

/// `lights_out_step(k, dt)` (2776 to 2800), from `run()` 6110 to 6112, for both balls (a copied one too) while any
/// fighter has the kind: before `move`, steer the shadow toward the enemy; at the end, erupt.
pub fn lights_out_step(f: &mut Fight, k: usize, dt: f64) {
    let o = 1 - k;
    let Some(lo) = &f.lights_out.balls[k] else {
        return;
    };
    let t_end = lo.t_end;
    f.balls[k].shadow = py::py_max(1e-6, t_end - f.t);
    if f.t + 1e-9 >= t_end {
        let (radius, dmg, knock) = (param(&lo.ab, "radius"), param(&lo.ab, "dmg"), param(&lo.ab, "knock"));
        f.lights_out.balls[k] = None;
        f.balls[k].shadow = 0.0;
        let (bp, op, ov) = (f.balls[k].p, f.balls[o].p, f.balls[o].v);
        let rel = geom::sub(op, bp);
        let dist = geom::hypot(rel);
        let hit = dist < radius + f.rules.r * 0.6;
        if hit {
            f.damage(k, o, dmg, "erupt", op, 0);
            f.knock(o, if dist > 1e-3 { rel } else { ov }, knock);
        }
        let data = vec![Value::Float(bp[0]), Value::Float(bp[1]), Value::Int(i64::from(hit))];
        f.ev("erupt", k as i64, data);
        return;
    }
    let (turn, home) = (param(&lo.ab, "turn"), param(&lo.ab, "home"));
    let b = &f.balls[k];
    let rel = geom::sub(f.balls[o].p, b.p);
    let want = dmath::atan2(rel[1], rel[0]);
    let cur = dmath::atan2(b.v[1], b.v[0]);
    // a = cur + max(-ab['turn'] * dt, min(ab['turn'] * dt, _wrap(want - cur)))
    let a = cur + py::py_max(-turn * dt, py::py_min(turn * dt, geom::wrap(want - cur)));
    // B.v = np.array([math.cos(a), math.sin(a)]) * float(np.linalg.norm(B.v))
    let speed = geom::norm(b.v);
    let b = &mut f.balls[k];
    b.v = [dmath::cos(a) * speed, dmath::sin(a) * speed];
    b.boost = py::py_max(b.boost, home - 1.0);
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5806 to 5808: `LO` when a fighter has
/// a `lights_out` in its own roster entry (a copied one alone adds no key).
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| has(f, k, &AbilityKind::LightsOut)) {
        &["LO"]
    } else {
        &[]
    }
}

/// This kind's rows of one frame: `snap`, `LO` (5562 to 5563), into `out` (only the keys [`record_keys`] gave): each
/// ball's seconds of shadow left.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    out.push_f64("LO", &[2], &[f.balls[0].shadow, f.balls[1].shadow]);
}

#[cfg(test)]
mod tests;
