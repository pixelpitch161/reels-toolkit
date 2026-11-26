//! Ability kind `rewind` (TURNTABLES's REWIND): `fight.py` `trigger` 2866 to 2867; 1927 to 1998 (`_rewind_ab`,
//! `_rewind_then`, `rewind_ready`, `start_rewind`, `rewind_step`); the gate in `abilities_step` 3035 to 3036. A
//! rewinding ball (the ball's `rw_left` above 0, `fight::rewinding`) is intangible: `damage`, `move` and the kinds that
//! test it skip it.
//!
//! Each rewinder keeps a record of its own state, one entry per substep, `window` s deep (`rw_hist`). REWIND undoes
//! the last `window` s of damage taken (HP back to its recorded value then, a `heal` event) and zips the ball back
//! along its recorded path to where it was then in `zip_beats` beats, intangible while it zips; then it carries on
//! with the heading it had then, at the shared speed (a `rewound` event), and its record starts afresh. It waits until
//! that would undo at least `min_undo` HP. A copied rewind (IDENTITY CRISIS) runs this code for the copier's ball:
//! with no record of its own (only fighters whose roster has the kind keep one) it does nothing, as in Python. None of
//! this code draws from the fight's generator.

use std::collections::VecDeque;

use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::roster::{AbilityCfg, AbilityKind};

/// One entry of `B.rw_hist`: `(self.t, B.p.copy(), B.v.copy(), B.hp)`.
pub type Then = (f64, V2, V2, f64);

/// One rewinder's state: `B.rw_hist` and the zip in progress (`rw_path`, `rw_dur`, `rw_hp`, `rw_v`; `rw_left` is the
/// ball's own field).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rewinder {
    /// `B.rw_hist`: a deque of `maxlen` entries (`None` before the first rewind step, as `hasattr` reads it).
    pub hist: Option<VecDeque<Then>>,
    /// The deque's `maxlen`: `int(round(window / DT)) + 3`.
    pub maxlen: usize,
    /// The path back, now to then.
    pub path: Vec<V2>,
    /// The zip's length in seconds.
    pub dur: f64,
    /// HP at the start and at the end of the zip.
    pub hp: (f64, f64),
    /// The velocity it had then.
    pub v: V2,
}

/// This ability's per-fight state (`Fight::rewind`): each ball's record and zip.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// By ball.
    pub balls: [Rewinder; 2],
}

/// `_rewind_ab(B)` (1927 to 1928): the first ability of kind `rewind` of the ball's own roster entry.
fn rewind_ab<'a>(f: &Fight<'a>, k: usize) -> &'a AbilityCfg {
    let cfg = f.balls[k].cfg;
    [&cfg.unlock1, &cfg.unlock2, &cfg.low]
        .into_iter()
        .find(|ab| ab.kind == AbilityKind::Rewind)
        .expect("a rewinder has a rewind ability (the rewinders list says so)")
}

/// `_rewind_then(B, ab)` (1930 to 1939): the ball's recorded state `window` s ago, or `None` when the record does not
/// reach back that far.
fn then(f: &Fight, k: usize, ab: &AbilityCfg) -> Option<Then> {
    let window = param(ab, "window");
    let h = f.rewind.balls[k].hist.as_ref()?;
    let first = h.front()?;
    if first.0 > f.t - window + 1e-6 {
        return None;
    }
    h.iter().find(|e| e.0 >= f.t - window - 1e-9).copied()
}

/// `trigger`, kind `rewind` (2866 to 2867): `start_rewind(k, ab)` (1946 to 1962).
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let Some(th) = then(f, k, ab) else {
        return;
    };
    let rw = &f.rewind.balls[k];
    let hist = rw.hist.as_ref().expect("then() found a record");
    // pts = [h[1] for h in B.rw_hist if h[0] >= then[0] - 1e-9]; B.rw_path = np.array(pts[::-1] + [then[1]])
    let mut path: Vec<V2> = hist.iter().filter(|h| h.0 >= th.0 - 1e-9).map(|h| h.1).collect();
    path.reverse();
    path.push(th.1);
    let dur = param(ab, "zip_beats") * f.rules.beat;
    let hp = f.balls[k].hp;
    let heal = py::py_max(0.0, py::py_min(f.rules.hp, th.3) - hp);
    let rw = &mut f.rewind.balls[k];
    rw.path = path;
    rw.dur = dur;
    rw.hp = (hp, hp + heal);
    rw.v = th.2;
    f.balls[k].rw_left = dur;
    f.ev("heal", k as i64, vec![Value::Float(heal), Value::Float(th.0)]);
}

/// `rewind_ready(k, ab)` (1940 to 1945, with `_rewind_then` 1930 to 1939): the gate in `abilities_step` 3035 to 3036:
/// a rewind fires only when it would undo at least `min_undo` HP.
pub fn ready(f: &Fight, k: usize, ab: &AbilityCfg) -> bool {
    match then(f, k, ab) {
        Some(th) => th.3 - f.balls[k].hp >= ab.get("min_undo").unwrap_or(0.0),
        None => false,
    }
}

/// `rewind_step(dt, rewinders)` (1963 to 1998), from `run()` 6116 to 6117, right after `move` (`ks`: the fighters with
/// a `rewind` ability, in ball order).
pub fn rewind_step(f: &mut Fight, dt: f64, ks: &[usize]) {
    for &k in ks {
        if f.rewind.balls[k].hist.is_none() {
            let window = param(rewind_ab(f, k), "window");
            let maxlen = py::round_half_even(window / f.rules.dt) as usize + 3;
            let rw = &mut f.rewind.balls[k];
            rw.maxlen = maxlen;
            rw.hist = Some(VecDeque::with_capacity(maxlen));
        }
        if f.balls[k].rw_left > 0.0 {
            let b = &mut f.balls[k];
            b.rw_left = py::py_max(0.0, b.rw_left - dt);
            let rw = &f.rewind.balls[k];
            let u = 1.0 - b.rw_left / rw.dur;
            let n = rw.path.len();
            let fl = u * (n - 1) as f64;
            let i0 = (fl as usize).min(n - 1);
            let i1 = (i0 + 1).min(n - 1);
            // B.rw_path[i0] + (B.rw_path[i1] - B.rw_path[i0]) * (f - i0)
            b.p = geom::add(
                rw.path[i0],
                geom::scale(geom::sub(rw.path[i1], rw.path[i0]), fl - i0 as f64),
            );
            b.hp = rw.hp.0 + (rw.hp.1 - rw.hp.0) * u;
            if b.rw_left <= 0.0 {
                b.v = geom::scale(geom::div(rw.v, geom::norm(rw.v) + 1e-12), f.rules.v);
                b.boost = 0.0;
                let p = b.p;
                f.rewind.balls[k].hist.as_mut().expect("made above").clear();
                f.ev("rewound", k as i64, vec![Value::Float(p[0]), Value::Float(p[1])]);
            }
            continue;
        }
        let entry = (f.t, f.balls[k].p, f.balls[k].v, f.balls[k].hp);
        let rw = &mut f.rewind.balls[k];
        let h = rw.hist.as_mut().expect("made above");
        if h.len() == rw.maxlen {
            h.pop_front();
        }
        h.push_back(entry);
    }
}
