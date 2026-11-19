//! Weapon kind `combo`, TRUMPET's valve combo: `fight.py` 898 to 966 (`combo_setup`, `combo_beat`, `combo_step`).
//!
//! Every `every` beats (haste divides it) `punches` pistons punch out of the ball one after another, `gap` beats
//! apart (haste divides the gap too). Between punches the valves (`th`) turn toward the enemy, `aim_rate` at most. A
//! piston aims where the enemy is when it starts, shoots out to the reach in `t_out` seconds, holds for `t_hold` and
//! pulls back over `t_back` (haste runs the whole punch faster). It hits once, the enemy against the piston (head
//! radius `head_r`): `dmg` and `knock`, the last punch of a combo `dmg3` and `knock3`; a landed punch lengthens the
//! reach by `grow` up to `max_len`. A punch that meets a shield is blocked and pulls straight back.
//!
//! The aim is on the ball (`th`) and the reach in `blade_len`, where `Ball.__init__` puts them; the punch queue, the
//! live punches and the combo count are in [`State`], per fighter. None of this code draws from the generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::Fight;
use crate::geom;
use crate::roster::{ComboCfg, WeaponCfg};

/// A piston on its way (one of `B.punches`).
#[derive(Clone, Debug, PartialEq)]
pub struct Punch {
    /// Its place in its combo, from 0 (`i`).
    pub i: i64,
    /// When it started (`t0`; a blocked punch is moved to the start of its pull-back).
    pub t0: f64,
    /// Its aim, where the enemy was when it started (`th`).
    pub th: f64,
    /// Its place across the aim (`off`).
    pub off: f64,
    /// Whether it has hit (or was blocked).
    pub hit: bool,
    /// Its time scale, 1 over the haste factor when it started (`k`).
    pub k: f64,
}

/// One fighter's combo state: what `combo_setup` hangs on the ball.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BallState {
    /// The times the punches still to come start (`punch_q`).
    pub punch_q: Vec<f64>,
    /// The next punch's place in its combo (`punch_i`).
    pub punch_i: i64,
    /// The pistons out of the ball now (`punches`).
    pub punches: Vec<Punch>,
    /// Combos started so far (`combo_n`).
    pub combo_n: i64,
}

/// Weapon kind `combo`'s per-fight state (`Fight::combo`), indexed by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// Each fighter's queue, pistons and count.
    pub balls: [BallState; 2],
}

/// `B.w['kind'] == 'combo'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Combo(_))
}

/// The combo's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k` has no combo.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a ComboCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Combo(w) => w,
        other => panic!("fighter {k} ({}) has no combo: {other:?}", cfg.name),
    }
}

/// `combo_setup(k)` (899 to 904), from `run()` 5756 to 5758: an empty queue, and the valves start facing the enemy.
pub fn setup(f: &mut Fight, k: usize) {
    f.combo.balls[k] = BallState::default();
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    f.balls[k].th = py::fmod_py(dmath::atan2(d[1], d[0]), 2.0 * dmath::PI);
}

/// `combo_beat(k, kb)` (905 to 917), from `run()` 5934 to 5935: on its beat a combo is queued, the first punch now.
/// Event `combo`: the combo's number, its punches, the seconds between two.
pub fn beat(f: &mut Fight, k: usize, kb: i64) {
    let w = weapon(f, k);
    // kb % max(1, int(round(w['every'] / self.ab_mult(k)))) == 0
    let every = (py::round_half_even(w.every / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(kb, every) != 0 {
        return;
    }
    // g = w.get('gap', 0.5) * self.R.beat / self.ab_mult(k)
    let g = w.gap * f.rules.beat / f.ab_mult(k);
    let n = w.punches;
    let t = f.t;
    let b = &mut f.combo.balls[k];
    // [self.t + j * g for j in range(n)]
    b.punch_q = (0..n).map(|j| t + j as f64 * g).collect();
    b.punch_i = 0;
    b.combo_n += 1;
    let data = vec![Value::Int(b.combo_n), Value::Int(n), Value::Float(g)];
    f.ev("combo", k as i64, data);
}

/// `combo_step(k, dt)` (918 to 966), from `run()` 6123 to 6124. Event `punch` when a piston starts: its place in the
/// combo, its aim, the reach then, the combo's number. A hit is `piston` with the punch's place as its sub-number;
/// a shielded enemy gives `block` with `piston`.
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let t = f.t;
    let o_p = f.balls[1 - k].p;
    let rel = geom::sub(o_p, f.balls[k].p);
    let want = dmath::atan2(rel[1], rel[0]);
    let lim = w.aim_rate * dt;
    {
        let b = &mut f.balls[k];
        // B.th = (B.th + max(-lim, min(lim, _wrap(want - B.th)))) % (2 * math.pi)
        b.th = py::fmod_py(
            b.th + geom::py_max(-lim, geom::py_min(lim, geom::wrap(want - b.th))),
            2.0 * dmath::PI,
        );
    }
    let n = w.punches;
    // while B.punch_q and self.t + 1e-9 >= B.punch_q[0]
    while f.combo.balls[k].punch_q.first().is_some_and(|&t0| t + 1e-9 >= t0) {
        let mult = f.ab_mult(k);
        let reach = f.balls[k].blade_len;
        let s = &mut f.combo.balls[k];
        s.punch_q.remove(0);
        let i = s.punch_i;
        s.punch_i += 1;
        s.punches.push(Punch {
            i,
            t0: t,
            th: want,
            // (i - (n - 1) / 2) * w.get('spread', 0.15)
            off: (i as f64 - (n - 1) as f64 / 2.0) * w.spread,
            hit: false,
            k: 1.0 / mult,
        });
        let data = vec![
            Value::Int(i),
            Value::Float(want),
            Value::Float(reach),
            Value::Int(s.combo_n),
        ];
        f.ev("punch", k as i64, data);
    }
    let (t_out, t_hold, t_back) = (w.t_out, w.t_hold, w.t_back);
    // Python walks `B.punches` and keeps the live ones in a new list; nothing in the loop adds a punch.
    let mut punches = std::mem::take(&mut f.combo.balls[k].punches);
    punches.retain_mut(|p| {
        let u = (t - p.t0) / p.k;
        if u >= t_out + t_hold + t_back {
            return false;
        }
        if p.hit || u >= t_out + t_hold {
            return true;
        }
        let e = geom::py_min(1.0, u / t_out);
        let b_p = f.balls[k].p;
        // ext = B.blade_len * (1 - (1 - e) ** 2): a Python float squared with `**` is the C runtime's pow(x, 2.0),
        // one step from the product now and then
        let ext = f.balls[k].blade_len * (1.0 - dmath::pow(1.0 - e, 2.0));
        let d = [dmath::cos(p.th), dmath::sin(p.th)];
        // a0 = B.p + d * R * 0.8 + np.array([-d[1], d[0]]) * P['off']
        let a0 = [
            b_p[0] + d[0] * r * 0.8 + -d[1] * p.off,
            b_p[1] + d[1] * r * 0.8 + d[0] * p.off,
        ];
        let relp = geom::sub(o_p, a0);
        // c = a0 + d * min(max(float(relp @ d), 0.0), R * 0.2 + ext)
        let s = geom::py_min(geom::py_max(geom::dot(relp, d), 0.0), r * 0.2 + ext);
        let c = [a0[0] + d[0] * s, a0[1] + d[1] * s];
        let dist = geom::hypot(geom::sub(o_p, c));
        if dist < r + w.head_r {
            p.hit = true;
            // hx, hy = (O.p + (c - O.p) / max(dist, 1e-6) * R)
            let m = geom::py_max(dist, 1e-6);
            let h = [o_p[0] + (c[0] - o_p[0]) / m * r, o_p[1] + (c[1] - o_p[1]) / m * r];
            if f.balls[1 - k].shield > 0.0 {
                let data = vec![
                    Value::Int(k as i64),
                    Value::Float(h[0]),
                    Value::Float(h[1]),
                    Value::Str("piston".to_string()),
                ];
                f.ev("block", (1 - k) as i64, data);
                // blocked: it pulls straight back
                p.t0 = t - (t_out + t_hold) * p.k;
            } else {
                let last = p.i == n - 1;
                let landed = f.damage(k, 1 - k, if last { w.dmg3 } else { w.dmg }, "piston", h, p.i);
                f.knock(1 - k, d, if last { w.knock3 } else { w.knock });
                if landed {
                    let b = &mut f.balls[k];
                    b.blade_len = geom::py_min(w.max_len, b.blade_len + w.grow);
                }
            }
        }
        true
    });
    f.combo.balls[k].punches = punches;
}

#[cfg(test)]
mod tests;
