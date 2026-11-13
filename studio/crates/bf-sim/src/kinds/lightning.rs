//! Weapon kind `lightning`, ELECTRIC's LIGHTNING RIFF: `fight.py` 3845 to 3922 (`POSTS`, `lightning_setup`,
//! `lightning_tick`, `bolt_source`, `bolt`), and the abilities `fork` and `tapping` (`trigger` 2933 to 2938).
//!
//! Every `every` beats on the 16th grid the fighter starts a riff of `int(n_vol)` notes on the 16th slots `slots`
//! (the first `int(n_vol)`, played in time order). Every note is an instant bolt: from the headstock when the enemy is
//! within `reach` of the ball's surface, else from the corner post nearest the enemy when the enemy is within
//! `post_r` of it, else a miss that crackles out `reach` toward the enemy. A bolt hits for `dmg` (the hit `bolt`, sub
//! the source + 1) and knocks the enemy `knock` away from where it came; the first landed bolt of a riff adds `grow`
//! notes (up to `n_max`). DISTORTION (`fork`) multiplies both reaches by `reach_k` for a while; TAPPING (`tapping`)
//! adds a bolt on every 8th for `beats` beats, `dmg_k` x the damage, not growing the riff. A stunned or smashed
//! fighter plays no notes.
//!
//! Python hangs this state on the ball (`riff_id`, `riff_n`, `riff_slots`, `riff_landed`, `fork_until`, `fork_ab`,
//! `tap_until`, `tap_ab`); here it is [`State`], `Fight::lightning`, indexed by fighter. The note count is the ball's
//! `n_vol`. The fight's generator is never drawn.

use std::collections::BTreeSet;

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::{ab_dur, param};
use crate::fight::{self, Fight};
use crate::geom;
use crate::roster::{AbilityCfg, LightningCfg, WeaponCfg};
use bf_core::roster::kinds::lightning::POSTS;

/// Weapon kind `lightning`'s per-fight state (`Fight::lightning`): what `fight.py` hangs on the ball, indexed by
/// fighter.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// `B.riff_id`: the riff under way (-1 before the first).
    pub riff_id: [i64; 2],
    /// `B.riff_n`: its notes.
    pub riff_n: [i64; 2],
    /// `B.riff_slots`: its 16th slots, sorted.
    pub riff_slots: [Vec<f64>; 2],
    /// `B.riff_landed`: the riffs one of whose bolts landed.
    pub riff_landed: [BTreeSet<i64>; 2],
    /// `B.fork_until`: when DISTORTION ends (-1.0: none yet).
    pub fork_until: [f64; 2],
    /// `B.fork_ab.get('reach_k', 1.0)` (read only while DISTORTION lasts).
    pub fork_reach_k: [f64; 2],
    /// `B.tap_until`: when TAPPING ends (-1.0: none yet).
    pub tap_until: [f64; 2],
    /// `B.tap_ab.get('dmg_k', 1.0)` (read only while TAPPING lasts).
    pub tap_dmg_k: [f64; 2],
}

impl Default for State {
    fn default() -> Self {
        State {
            riff_id: [-1; 2],
            riff_n: [0; 2],
            riff_slots: Default::default(),
            riff_landed: Default::default(),
            fork_until: [-1.0; 2],
            fork_reach_k: [1.0; 2],
            tap_until: [-1.0; 2],
            tap_dmg_k: [1.0; 2],
        }
    }
}

/// `B.w['kind'] == 'lightning'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Lightning(_))
}

/// The fighter's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not lightning.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a LightningCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Lightning(w) => w,
        other => panic!("fighter {k} ({}) has no lightning weapon: {other:?}", cfg.name),
    }
}

/// `lightning_setup(k)` (3857 to 3864), from `run()` 5854 to 5856: the notes at `n0` (default 2), no riff, no
/// DISTORTION, no TAPPING.
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    f.balls[k].n_vol = w.n0.unwrap_or(2.0);
    let s = &mut f.lightning;
    s.riff_id[k] = -1;
    s.riff_n[k] = 0;
    s.riff_slots[k] = Vec::new();
    s.riff_landed[k] = BTreeSet::new();
    s.fork_until[k] = -1.0;
    s.fork_reach_k[k] = 1.0;
    s.tap_until[k] = -1.0;
    s.tap_dmg_k[k] = 1.0;
}

/// `slots[:n]`, with Python's slice of a list: a negative `n` drops that many from the end.
fn first(slots: &[f64], n: i64) -> &[f64] {
    let len = slots.len() as i64;
    let end = if n < 0 { (len + n).max(0) } else { n.min(len) };
    &slots[..end as usize]
}

/// `lightning_tick(k, k16)` (3865 to 3881), from `run()` 6050 to 6054: on 16th `k16` of the fight, a new riff on the
/// bar line (event `riff`: its number and notes), a riff note on its slots, and TAPPING's bolt on every 8th.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    if fight::stunned(f, k) || f.balls[k].smashed > 0.0 {
        return;
    }
    let w = weapon(f, k);
    // main16 = int(round(4 * w['every'])) (the roster refuses 0); pos = k16 % main16
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    let pos = py::imod_py(k16, main16);
    if pos == 0 {
        let s = &mut f.lightning;
        s.riff_id[k] += 1;
        // int(B.n_vol + 1e-9): toward zero
        s.riff_n[k] = (f.balls[k].n_vol + 1e-9) as i64;
        let mut slots = first(&w.slots, s.riff_n[k]).to_vec();
        // sorted(...): the slots are numbers, never NaN (JSON has none); stable, as Python's
        slots.sort_by(|a, b| a.partial_cmp(b).expect("a slot is a number"));
        s.riff_slots[k] = slots;
        let data = vec![Value::Int(s.riff_id[k]), Value::Int(s.riff_n[k])];
        f.ev("riff", k as i64, data);
    }
    // `pos in B.riff_slots`, then `B.riff_slots.index(pos)`: the first equal slot
    let at = f.lightning.riff_slots[k].iter().position(|&s| s == pos as f64);
    if let Some(note) = at {
        bolt(f, k, note as i64, false);
    }
    if f.t < f.lightning.tap_until[k] + 1e-9 && py::imod_py(k16, 2) == 0 {
        bolt(f, k, -1, true);
    }
}

/// `bolt_source(k, fork)` (3883 to 3895): where this bolt strikes from: `Some(-1)` the headstock, `Some(0..3)` a
/// post, `None` out of range (a miss, from the ball); and the point it leaves from.
fn bolt_source(f: &Fight, k: usize, fork: bool) -> (Option<i64>, f64, f64) {
    let w = weapon(f, k);
    let (bp, op) = (f.balls[k].p, f.balls[1 - k].p);
    let (a, r) = (f.rules.a, f.rules.r);
    let rk = if fork { f.lightning.fork_reach_k[k] } else { 1.0 };
    // gap = float(np.hypot(*(O.p - B.p))) - 2 * R
    let gap = geom::hypot(geom::sub(op, bp)) - 2.0 * r;
    if gap <= w.reach * rk {
        return (Some(-1), bp[0], bp[1]);
    }
    // dp = [float(np.hypot(O.p[0] - sx * A, O.p[1] - sy * A)) for sx, sy in self.POSTS]; j = int(np.argmin(dp))
    let dp = POSTS.map(|(sx, sy)| dmath::np_hypot(op[0] - sx * a, op[1] - sy * a));
    let mut j = 0;
    for (i, &d) in dp.iter().enumerate().skip(1) {
        if d < dp[j] {
            j = i;
        }
    }
    if dp[j] <= w.post_r * rk {
        return (Some(j as i64), POSTS[j].0 * a, POSTS[j].1 * a);
    }
    (None, bp[0], bp[1])
}

/// `bolt(k, note, tap)` (3897 to 3922): one bolt. Event `bolt`: the note (-1 for a tap), the source (-1 the
/// headstock or a miss, 0..3 a post), where it leaves and where it ends, whether it hit, the riff, DISTORTION, a tap.
fn bolt(f: &mut Fight, k: usize, note: i64, tap: bool) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let fork = f.t < f.lightning.fork_until[k] + 1e-9;
    let (src, x0, y0) = bolt_source(f, k, fork);
    let o = [x0, y0];
    let op = f.balls[1 - k].p;
    let d = geom::sub(op, o);
    let dist = geom::hypot(d);
    let u = geom::div(d, py::py_max(dist, 1e-9));
    let hit = src.is_some();
    let [x1, y1] = if hit {
        // where it strikes the enemy's glass
        geom::sub(op, geom::scale(u, r))
    } else {
        // a miss crackles out its reach toward the enemy
        let rk = if fork { f.lightning.fork_reach_k[k] } else { 1.0 };
        geom::add(o, geom::scale(u, r + w.reach * rk))
    };
    let riff_id = f.lightning.riff_id[k];
    let data = vec![
        Value::Int(note),
        Value::Int(src.unwrap_or(-1)),
        Value::Float(x0),
        Value::Float(y0),
        Value::Float(x1),
        Value::Float(y1),
        Value::Int(hit as i64),
        Value::Int(riff_id),
        Value::Int(fork as i64),
        Value::Int(tap as i64),
    ];
    f.ev("bolt", k as i64, data);
    let Some(src) = src else {
        return;
    };
    let dmg = w.dmg * if tap { f.lightning.tap_dmg_k[k] } else { 1.0 };
    let landed = f.damage(k, 1 - k, dmg, "bolt", [x1, y1], src + 1);
    f.knock(1 - k, u, w.knock);
    if landed && !tap && f.lightning.riff_landed[k].insert(riff_id) {
        // a riff that lands counts once: one more note next riff
        let b = &mut f.balls[k];
        b.n_vol = py::py_min(w.n_max, b.n_vol + w.grow);
    }
}

/// `trigger`, kind `fork` (2933 to 2935): `fork_until`, `fork_ab`. Event `fork`: when it ends.
pub fn start_fork(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + ab_dur(f, ab);
    f.lightning.fork_until[k] = until;
    f.lightning.fork_reach_k[k] = ab.get("reach_k").unwrap_or(1.0);
    f.ev("fork", k as i64, vec![Value::Float(until)]);
}

/// `trigger`, kind `tapping` (2936 to 2938): `tap_until` (`beats` x beat), `tap_ab`. Event `tapping`: when it ends.
pub fn start_tapping(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + param(ab, "beats") * f.rules.beat;
    f.lightning.tap_until[k] = until;
    f.lightning.tap_dmg_k[k] = ab.get("dmg_k").unwrap_or(1.0);
    f.ev("tapping", k as i64, vec![Value::Float(until)]);
}
