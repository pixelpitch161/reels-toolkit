//! Ability kind `lash` (SYNTH's SAWTOOTH): `fight.py` `trigger` 2868 to 2869; 1661 to 1712 (`start_lash`, `lash_angle`,
//! `lash_step`).
//!
//! For `beats` beats, n arms (the tether's slots, at least `n_min`) of length `len` reach out of the ball around the aim
//! at the enemy (`spread` degrees apart) and sweep across `sweep` degrees in a saw-tooth (a linear sweep every
//! `period_beats`, then a snap back; arm j lags by `lag` of a period); a forward sweep that crosses the enemy cuts
//! (`dmg`, `knock`), each arm resting `cool` seconds. A rewinding enemy is not cut. None of this code draws from the
//! generator. IDENTITY CRISIS cannot copy a lash (it is not in `COPYABLE`), but nothing here assumes which side has it.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, radius, rewinding};
use crate::geom;
use crate::kinds::tether;
use crate::roster::AbilityCfg;

/// One entry of `self.lashes` (`start_lash`'s dict).
#[derive(Clone, Debug, PartialEq)]
pub struct Lash {
    /// The fighter whose arms they are.
    pub owner: usize,
    /// When it started.
    pub t0: f64,
    /// When it ends.
    pub t1: f64,
    /// The aim at the enemy when it started.
    pub aim: f64,
    /// The number of arms.
    pub n: i64,
    /// Their length beyond the ball's surface.
    pub len: f64,
    /// Radians between two arms.
    pub spread: f64,
    /// Radians an arm sweeps across.
    pub sweep: f64,
    /// Seconds a sweep takes.
    pub period: f64,
    /// Arm j lags by j x this of a period.
    pub lag: f64,
    /// A cut's damage.
    pub dmg: f64,
    /// A cut's knockback.
    pub knock: f64,
    /// Seconds an arm rests after a cut.
    pub cool: f64,
    /// When each arm last cut (`-9.0` before).
    pub rest: Vec<f64>,
    /// Each arm's (angle, phase) at the step before (`None` before its first).
    pub prev: Vec<Option<(f64, f64)>>,
}

/// This kind's per-fight state (`Fight::lash`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.lashes`: the lashes in progress (`start_lash`'s dicts).
    pub lashes: Vec<Lash>,
}

/// Whether `self.lashes` holds anything (Python's truth test of the list).
pub fn live(f: &Fight) -> bool {
    !f.lash.lashes.is_empty()
}

/// `trigger`, kind `lash` (2868 to 2869): `start_lash(k, ab)` (1661 to 1681). Event `lash`: the aim, the number of
/// arms, their length, the lash's length in seconds, the period, the sweep, the spread and the lag.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    // n = max(ab.get('n_min', 1), int(tc['slots'] + 1e-9) if tc else 0)
    let n_min = ab.get("n_min").unwrap_or(1.0) as i64;
    let have = tether::slots(f, k).map_or(0, |s| (s + 1e-9) as i64);
    let n = n_min.max(have);
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let aim = dmath::atan2(d[1], d[0]);
    let beat = f.rules.beat;
    let arms = usize::try_from(n).unwrap_or(0);
    let l = Lash {
        owner: k,
        t0: f.t,
        t1: f.t + param(ab, "beats") * beat,
        aim,
        n,
        len: param(ab, "len"),
        spread: geom::radians(param(ab, "spread")),
        sweep: geom::radians(param(ab, "sweep")),
        period: param(ab, "period_beats") * beat,
        lag: ab.get("lag").unwrap_or(0.15),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        cool: param(ab, "cool"),
        rest: vec![-9.0; arms],
        prev: vec![None; arms],
    };
    let data = vec![
        Value::Float(aim),
        Value::Int(n),
        Value::Float(l.len),
        Value::Float(l.t1 - l.t0),
        Value::Float(l.period),
        Value::Float(l.sweep),
        Value::Float(l.spread),
        Value::Float(l.lag),
    ];
    f.lash.lashes.push(l);
    f.ev("lash", k as i64, data);
}

/// `Fight.lash_angle(L, j, t)` (1682 to 1686): arm j's angle at time t and its phase (the renderer uses the same).
pub fn lash_angle(l: &Lash, j: i64, t: f64) -> (f64, f64) {
    let ph = py::fmod_py((t - l.t0) / l.period + j as f64 * l.lag, 1.0);
    let a = l.aim + (j as f64 - (l.n - 1) as f64 / 2.0) * l.spread + l.sweep * (ph - 0.5);
    (a, ph)
}

/// `lash_step(dt)` (1687 to 1712), from `run()` 6137 to 6138 (`if getattr(self, 'lashes', None)`): every arm's sweep
/// since the step before that crosses the enemy cuts it; finished lashes are dropped.
pub fn lash_step(f: &mut Fight, _dt: f64) {
    let r = f.rules.r;
    for i in 0..f.lash.lashes.len() {
        let k = f.lash.lashes[i].owner;
        let o = 1 - k;
        let r_o = radius(f, o);
        let rel = geom::sub(f.balls[o].p, f.balls[k].p);
        let dist = geom::hypot(rel);
        let phi = dmath::atan2(rel[1], rel[0]);
        for j in 0..f.lash.lashes[i].n {
            let ju = j as usize;
            let (a, ph) = lash_angle(&f.lash.lashes[i], j, f.t);
            let pa = f.lash.lashes[i].prev[ju];
            f.lash.lashes[i].prev[ju] = Some((a, ph));
            let l = &f.lash.lashes[i];
            let Some(pa) = pa else { continue };
            if ph < pa.1 || f.t < l.rest[ju] + l.cool {
                continue; // the snap back doesn't cut
            }
            if rewinding(f, o) || dist - r_o > l.len + r {
                continue;
            }
            let half = dmath::asin(py::py_min(1.0, r_o / py::py_max(dist, 1e-6)));
            if geom::swept(pa.0, a - pa.0, phi, half) {
                let (dmg, knock) = (l.dmg, l.knock);
                f.lash.lashes[i].rest[ju] = f.t;
                let m = py::py_max(dist, 1e-6);
                // hx, hy = (B.p + rel / max(dist, 1e-6) * (dist - rO)).tolist()
                let h = geom::add(f.balls[k].p, geom::scale(geom::div(rel, m), dist - r_o));
                f.damage(k, o, dmg, "lash", h, j);
                let side = [-dmath::sin(a), dmath::cos(a)];
                // side + 0.3 * rel / max(dist, 1e-6)
                let away = geom::add(side, geom::div(geom::scale(rel, 0.3), m));
                f.knock(o, away, knock);
            }
        }
    }
    let t = f.t;
    f.lash.lashes.retain(|l| t < l.t1);
}
