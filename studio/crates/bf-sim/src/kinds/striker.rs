//! Weapon kind `striker`, the snake charmer (the FLUTE): `fight.py`'s `striker_setup`, `striker_beat`,
//! `striker_fire` and `striker_step` (lines 1132 to 1211), and the opt-in rule `swat_cobras` (1213 to 1241).
//!
//! Every `drop_every` beats the fighter drops a basket where it stands (at most `max_baskets`; the oldest is taken
//! back for a new one). A cobra rises from each basket over `rise` beats. On the strike grid every risen cobra with
//! the enemy in reach lunges at where the enemy is; the lunge is out after `t_strike` seconds and bites if the enemy
//! then touches the fang line. A landed bite grows the reach and shortens the lunge. During SNAKE DANCE the cobras
//! leave their baskets and slither after the enemy, then slither home.
//!
//! Python hangs this state on the ball (`B.baskets`, `B.next_bk`, `B.dance_until`, `B.t_strike`); here it is
//! [`Striker`], in [`Fight::striker`] under the fighter's index. The reach itself is the ball's `blade_len`. None of
//! this code draws from the generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::roster::{StrikerCfg, WeaponCfg};

/// `getattr(self.R, 'swat_beats', 2)`: beats a swatted cobra stays flat. No episode sets the rule, so it is the
/// Python default and not a field of [`crate::Rules`].
pub const SWAT_BEATS: f64 = 2.0;

/// A cobra's lunge (`bk['strike']`): out along `aim` to `len` by `t_hit`, when it bites or misses; over at `t_end`.
#[derive(Clone, Debug, PartialEq)]
pub struct Strike {
    /// When it started.
    pub t0: f64,
    /// When the head is fully out and the bite is tested.
    pub t_hit: f64,
    /// When the cobra is back and may lunge again.
    pub t_end: f64,
    /// The direction of the lunge, in radians: at the enemy as it was at `t0`.
    pub aim: f64,
    /// The length of the lunge.
    pub len: f64,
    /// Whether the bite was tested (or the lunge was swatted): a lunge bites at most once.
    pub done: bool,
}

/// A basket and its cobra (an entry of `B.baskets`).
#[derive(Clone, Debug, PartialEq)]
pub struct Basket {
    /// Its number, counting up from 0 per fighter.
    pub id: i64,
    /// Where the basket stands.
    pub home: V2,
    /// Where the cobra is: on its basket, except during SNAKE DANCE and on its way back.
    pub cp: V2,
    /// When the basket was dropped.
    pub t_drop: f64,
    /// The lunge in progress, if any.
    pub strike: Option<Strike>,
    /// Swatted flat until this time (`bk['down']`; `None` while Python's dict has no such key).
    pub down: Option<f64>,
}

/// One striker's state: what `striker_setup` and `run()` put on the ball.
#[derive(Clone, Debug, PartialEq)]
pub struct Striker {
    /// The baskets on the floor, oldest first.
    pub baskets: Vec<Basket>,
    /// The next basket's number.
    pub next_bk: i64,
    /// SNAKE DANCE runs while `t < dance_until` (-1.0 before the first dance).
    pub dance_until: f64,
    /// Seconds a lunge takes now: `w['t_strike']` at the start, less `speed_grow` per landed bite, down to `t_min`.
    pub t_strike: f64,
}

/// `B.w['kind'] == 'striker'`.
pub fn is_striker(fight: &Fight, k: usize) -> bool {
    matches!(fight.balls[k].cfg.weapon, WeaponCfg::Striker(_))
}

/// The striker's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k` is not a striker.
fn weapon<'a>(fight: &Fight<'a>, k: usize) -> &'a StrikerCfg {
    let cfg = fight.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Striker(w) => w,
        other => panic!("fighter {k} ({}) is not a striker: {other:?}", cfg.name),
    }
}

/// `int(round(4 * w['every']))`: the striker's own strike grid in 16ths (4 for `every = 1` beat). The fight loop
/// divides it by the haste factor (`fight.py` 5991 to 5992).
///
/// # Panics
/// When fighter `k` is not a striker.
pub fn main16(fight: &Fight, k: usize) -> i64 {
    py::round_half_even(4.0 * weapon(fight, k).every) as i64
}

/// Fighter `k`'s striker state.
///
/// # Panics
/// When [`setup`] has not run for fighter `k` (Python raises `AttributeError`: the ball has no `baskets`).
fn state<'f>(fight: &'f Fight, k: usize) -> &'f Striker {
    fight.striker[k]
        .as_ref()
        .unwrap_or_else(|| panic!("fighter {k} was not set up as a striker"))
}

/// Fighter `k`'s striker state, to change. Panics as [`state`] does.
fn state_mut<'f>(fight: &'f mut Fight, k: usize) -> &'f mut Striker {
    fight.striker[k]
        .as_mut()
        .unwrap_or_else(|| panic!("fighter {k} was not set up as a striker"))
}

/// `striker_setup(k)`, with the line of `run()` that follows it (`B.t_strike = B.w['t_strike']`, 5770): no baskets,
/// the next one is number 0, no dance, and a lunge takes `t_strike` seconds.
///
/// # Panics
/// When fighter `k` is not a striker.
pub fn setup(fight: &mut Fight, k: usize) {
    let w = weapon(fight, k);
    fight.striker[k] = Some(Striker {
        // dict(id, home (2,), cp (2,), t_drop, strike or None)
        baskets: Vec::new(),
        next_bk: 0,
        dance_until: -1.0,
        t_strike: w.t_strike,
    });
}

/// `striker_beat(k, kb)`: on beat `kb` of the fight, every `drop_every` beats, the fighter drops a basket where it
/// stands; with `max_baskets` on the floor the oldest is taken back first. Event `basket`: the new id, where it
/// stands, and the id taken back (-1 for none).
///
/// # Panics
/// When `int(drop_every)` or `int(max_baskets)` is below 1 (Python raises there: a remainder by zero, a pop from
/// an empty list). The roster refuses such an entry when it is read (`roster.rs`), so only settings built by hand
/// can get here; debug builds name the setting.
pub fn beat(fight: &mut Fight, k: usize, kb: i64) {
    let w = weapon(fight, k);
    // int(w['drop_every']), int(w['max_baskets']): truncated
    let (drop_every, max_baskets) = (w.drop_every as i64, w.max_baskets as i64);
    debug_assert!(
        drop_every >= 1 && max_baskets >= 1,
        "a striker needs drop_every and max_baskets of at least 1: {} and {}",
        w.drop_every,
        w.max_baskets
    );
    if kb % drop_every == 0 {
        let (p, t) = (fight.balls[k].p, fight.t);
        let st = state_mut(fight, k);
        let mut gone = -1;
        if st.baskets.len() as i64 >= max_baskets {
            gone = st.baskets.remove(0).id;
        }
        let id = st.next_bk;
        st.next_bk += 1;
        st.baskets.push(Basket {
            id,
            home: p,
            cp: p,
            t_drop: t,
            strike: None,
            down: None,
        });
        let data = vec![Value::Int(id), Value::Float(p[0]), Value::Float(p[1]), Value::Int(gone)];
        fight.ev("basket", k as i64, data);
    }
}

/// `striker_fire(k)`: on the strike grid, every risen cobra that is neither lunging nor swatted flat and has the
/// enemy in reach (`blade_len + slack` from the cobra to the enemy's surface) lunges at it. Event `strike`: the
/// basket id, the aim, the length of the lunge and the seconds until it bites (haste divides them).
pub fn fire(fight: &mut Fight, k: usize) {
    let w = weapon(fight, k);
    let r = fight.rules.r;
    for i in 0..state(fight, k).baskets.len() {
        let bk = &state(fight, k).baskets[i];
        if fight.t < bk.t_drop + w.rise * fight.rules.beat - 1e-9 || bk.strike.is_some() {
            continue;
        }
        if bk.down.is_some_and(|down| fight.t < down) {
            // swatted flat (opt-in rule, #41): still curling back up
            continue;
        }
        let id = bk.id;
        let rel = geom::sub(fight.balls[1 - k].p, bk.cp);
        let dist = geom::hypot(rel);
        let blade_len = fight.balls[k].blade_len;
        if dist - r > blade_len + w.slack {
            continue;
        }
        let a = dmath::atan2(rel[1], rel[0]);
        let ln = geom::py_min(blade_len + r * 0.4, geom::py_max(0.2, dist - r * 0.6));
        let tk = state(fight, k).t_strike / fight.ab_mult(k);
        let t = fight.t;
        state_mut(fight, k).baskets[i].strike = Some(Strike {
            t0: t,
            t_hit: t + tk,
            t_end: t + tk * 2.6,
            aim: a,
            len: ln,
            done: false,
        });
        let data = vec![Value::Int(id), Value::Float(a), Value::Float(ln), Value::Float(tk)];
        fight.ev("strike", k as i64, data);
    }
}

/// `striker_step(k, dt)`: the cobras slither (after the enemy during SNAKE DANCE, else home), a lunge that is out
/// bites if the enemy touches the fang line (a capsule of radius `head_r` around it; `dmg`, `knock`, and a landed
/// bite grows the reach by `grow` up to `max_reach` and shortens the lunge by `speed_grow` down to `t_min`), and a
/// lunge that is over is cleared.
pub fn step(fight: &mut Fight, k: usize, dt: f64) {
    let w = weapon(fight, k);
    let r = fight.rules.r;
    let t = fight.t;
    let dancing = t < state(fight, k).dance_until;
    for i in 0..state(fight, k).baskets.len() {
        let o_p = fight.balls[1 - k].p;
        let bk = &mut state_mut(fight, k).baskets[i];
        let tgt = if dancing { o_p } else { bk.home };
        let d = geom::sub(tgt, bk.cp);
        let dist = geom::hypot(d);
        let stop = if dancing { 2.0 * r + 0.1 } else { 0.0 };
        if dist > stop + 1e-6 && bk.strike.is_none() {
            let sp = w.slither * (if dancing { 1.0 } else { 1.6 }) * dt;
            // bk['cp'] + d / dist * min(sp, dist - stop)
            let m = geom::py_min(sp, dist - stop);
            bk.cp = [bk.cp[0] + d[0] / dist * m, bk.cp[1] + d[1] / dist * m];
        }
        let (id, a0) = (bk.id, bk.cp);
        let Some(s) = &mut bk.strike else {
            continue;
        };
        let t_end = s.t_end;
        if !s.done && t + 1e-9 >= s.t_hit {
            s.done = true;
            let (aim, len) = (s.aim, s.len);
            let u = [dmath::cos(aim), dmath::sin(aim)];
            let q = geom::sub(o_p, a0);
            // c = a0 + u * min(max(float(q @ u), 0.0), S['len'])
            let along = geom::py_min(geom::py_max(geom::dot(q, u), 0.0), len);
            let c = [a0[0] + u[0] * along, a0[1] + u[1] * along];
            let dd = geom::hypot(geom::sub(o_p, c));
            if dd < r + w.head_r {
                // hx, hy = O.p + (c - O.p) / max(dd, 1e-6) * R
                let m = geom::py_max(dd, 1e-6);
                let h = [o_p[0] + (c[0] - o_p[0]) / m * r, o_p[1] + (c[1] - o_p[1]) / m * r];
                if fight.balls[1 - k].shield > 0.0 {
                    let data = vec![
                        Value::Int(k as i64),
                        Value::Float(h[0]),
                        Value::Float(h[1]),
                        Value::Str("fang".to_string()),
                    ];
                    fight.ev("block", (1 - k) as i64, data);
                } else {
                    let landed = fight.damage(k, 1 - k, w.dmg, "fang", h, id);
                    fight.knock(1 - k, u, w.knock);
                    if landed {
                        let b = &mut fight.balls[k];
                        b.blade_len = geom::py_min(w.max_reach, b.blade_len + w.grow);
                        let st = state_mut(fight, k);
                        st.t_strike = geom::py_max(w.t_min, st.t_strike - w.speed_grow);
                    }
                }
            }
        }
        if t >= t_end {
            state_mut(fight, k).baskets[i].strike = None;
        }
    }
}

/// `swat_cobras(k, a, d, L_)`, the opt-in rule `Rules(swat_cobras=True)` (#41): fighter `k`'s live lance (base `a`,
/// direction `d`, length `l`) knocks flat every risen enemy cobra it passes through: the cobra's body over its
/// basket, or its head while a lunge is still on its way out. A lunge in the air then misses, and the cobra cannot
/// lunge for [`SWAT_BEATS`] beats. No damage, no growth. Event `swat`: the cobra's owner, the basket id and where
/// the lance touched.
///
/// The thrust step calls this while the lance is going out or held, when the rule is on (`fight.py` 753 to 754).
/// It does nothing when the enemy has no baskets on the floor.
pub fn swat_cobras(fight: &mut Fight, k: usize, a: V2, d: V2, l: f64) {
    let o = 1 - k;
    // bks = getattr(O, 'baskets', None); if not bks: return
    let n = match &fight.striker[o] {
        Some(st) if !st.baskets.is_empty() => st.baskets.len(),
        _ => return,
    };
    let w = weapon(fight, o);
    let (t, beat) = (fight.t, fight.rules.beat);
    let (q0, q1) = (a, [a[0] + d[0] * l, a[1] + d[1] * l]);
    for i in 0..n {
        let bk = &mut state_mut(fight, o).baskets[i];
        if t < bk.t_drop + w.rise * beat - 1e-9 || t < bk.down.unwrap_or(-1.0) {
            // still rising out of its basket, or already knocked flat
            continue;
        }
        let p0 = bk.cp;
        let mut p1 = p0;
        if let Some(s) = bk.strike.as_ref().filter(|s| !s.done) {
            // the head lunging out along the strike
            let u = geom::py_min(1.0, geom::py_max(0.0, (t - s.t0) / geom::py_max(1e-6, s.t_hit - s.t0)));
            // p0 + np.array([math.cos(S['aim']), math.sin(S['aim'])]) * S['len'] * u
            p1 = [
                p0[0] + dmath::cos(s.aim) * s.len * u,
                p0[1] + dmath::sin(s.aim) * s.len * u,
            ];
        }
        let (dist, c) = geom::seg_seg(p0, p1, q0, q1);
        if dist < w.head_r + 0.05 {
            bk.down = Some(t + SWAT_BEATS * beat);
            if let Some(s) = &mut bk.strike {
                // the strike in the air misses
                s.done = true;
            }
            let data = vec![
                Value::Int(o as i64),
                Value::Int(bk.id),
                Value::Float(c[0]),
                Value::Float(c[1]),
            ];
            fight.ev("swat", k as i64, data);
        }
    }
}

#[cfg(test)]
mod tests;
