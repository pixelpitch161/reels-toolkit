//! What UKULELE's look and its part both derive from the recording: its SURF'S UP waves (`_surfs()` of #33's
//! `render.py` 89 to 116: each wave's end replayed on the engine's substep grid) and its UKE-NADO waterspouts
//! (`SPOUTS`, `spout_at`: the path from the recording's `X33_SPOUT`).

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::recording::{Array, Recording};
use bf_core::roster::kinds::surf::surf_dist;

/// One SURF'S UP wave: `dict(t0, owner, main, h, o, u, power, hw, t_end, crashed)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Surf {
    /// When it was strummed.
    pub t0: f64,
    /// Who rides it.
    pub owner: usize,
    /// On the weapon's own grid.
    pub main: bool,
    /// The wave's height (`blade_len` then).
    pub h: f64,
    /// Where its front started.
    pub o: [f64; 2],
    /// Where it rolls.
    pub u: [f64; 2],
    /// The ride's speed burst.
    pub power: f64,
    /// The front's half width.
    pub hw: f64,
    /// When it ended (the ropes or its life), at most the K.O.
    pub t_end: f64,
    /// Whether it broke on the ropes before its life ran out.
    pub crashed: bool,
}

/// One UKE-NADO: `dict(t0, owner, p0, t_end)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Spout {
    /// When it started.
    pub t0: f64,
    /// Who sent it.
    pub owner: usize,
    /// Where it started.
    pub p0: [f64; 2],
    /// When it ended, at most the K.O.
    pub t_end: f64,
}

/// The rules the replay needs (the recording's `rules`): `(A, V, DT, KNOCK_T)`.
fn rules(rec: &Recording) -> (f64, f64, f64, f64) {
    let get = |k: &str, d: f64| match rec.rules.get(k) {
        Some(Value::Float(x)) => *x,
        Some(Value::Int(n)) => *n as f64,
        _ => d,
    };
    (
        get("A", 3.0),
        get("V", 2.4),
        get("DT", 1.0 / 240.0),
        get("KNOCK_T", 0.25),
    )
}

fn mine(e: &Event, kind: &str, k: usize) -> bool {
    e.kind == kind && e.who() == Some(k)
}

/// `Fight.surf_dist(tau, RULES.V, power, RULES.KNOCK_T)` with the recording's rules.
pub fn dist(rec: &Recording, tau: f64, power: f64) -> f64 {
    let (_, v, _, kt) = rules(rec);
    surf_dist(tau, v, power, kt)
}

/// `SURFS`: fighter `k`'s waves, in event order; `life` is its weapon's `life`, `t_ko` the K.O. (or the end).
pub fn surfs(rec: &Recording, k: usize, life: f64, t_ko: f64) -> Vec<Surf> {
    let (a, v, dt, kt) = rules(rec);
    rec.events
        .iter()
        .filter(|e| mine(e, "surf", k))
        .map(|e| {
            let o = [e.f(5), e.f(6)];
            let u = [dmath::cos(e.f(7)), dmath::sin(e.f(7))];
            let (power, hw) = (e.f(8), e.f(9));
            let mut m = 0u32;
            let mut tau;
            loop {
                tau = f64::from(m) * dt;
                let d = surf_dist(tau, v, power, kt);
                let c = [o[0] + u[0] * d, o[1] + u[1] * d];
                if c[0].abs() >= a - 0.05 || c[1].abs() >= a - 0.05 || tau >= life || m > 100_000 {
                    break;
                }
                m += 1;
            }
            let end = e.t + f64::from(m) * dt;
            Surf {
                t0: e.t,
                owner: k,
                main: e.i(3) != 0,
                h: e.f(4),
                o,
                u,
                power,
                hw,
                t_end: end.min(t_ko),
                crashed: tau < life && end <= t_ko,
            }
        })
        .collect()
}

/// `SPOUTS`: fighter `k`'s waterspouts, each ending at its `spout_end` (or the K.O.).
pub fn spouts(rec: &Recording, k: usize, t_ko: f64) -> Vec<Spout> {
    let ev = &rec.events;
    ev.iter()
        .filter(|e| mine(e, "spout", k))
        .map(|e| {
            let end = ev
                .iter()
                .find(|x| mine(x, "spout_end", k) && x.t >= e.t)
                .map_or(t_ko, |x| x.t);
            Spout {
                t0: e.t,
                owner: k,
                p0: [e.f(3), e.f(4)],
                t_end: end.min(t_ko),
            }
        })
        .collect()
}

/// `SPOUT_PATH`: the recording's `X33_SPOUT` points `(t, owner, x, y)` in order (its per-frame rows joined), or
/// none.
pub fn spout_path(rec: &Recording) -> Vec<[f64; 4]> {
    let Some(Array::Nested(Value::List(rows))) = rec.arrays.get("X33_SPOUT") else {
        return Vec::new();
    };
    let num = |v: &Value| match v {
        Value::Float(x) => *x,
        Value::Int(n) => *n as f64,
        _ => 0.0,
    };
    rows.iter()
        .filter_map(|r| match r {
            Value::List(pts) => Some(pts),
            _ => None,
        })
        .flatten()
        .filter_map(|q| match q {
            Value::List(v) if v.len() == 4 => Some([num(&v[0]), num(&v[1]), num(&v[2]), num(&v[3])]),
            _ => None,
        })
        .collect()
}

/// Where spout `sp` is at `tt` (clamped to its end) along `path`: `spout_at`'s position.
pub fn spout_pos(sp: &Spout, path: &[[f64; 4]], t: f64) -> [f64; 2] {
    let tt = t.min(sp.t_end);
    let i = path.partition_point(|q| q[0] < tt);
    if i == 0 || path.is_empty() {
        sp.p0
    } else if i >= path.len() {
        let q = path[path.len() - 1];
        [q[2], q[3]]
    } else {
        let (a, b) = (path[i - 1], path[i]);
        let u = (tt - a[0]) / (b[0] - a[0]).max(1e-9);
        [a[2] * (1.0 - u) + b[2] * u, a[3] * (1.0 - u) + b[3] * u]
    }
}
