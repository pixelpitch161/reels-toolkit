//! Weapon kind `line`, ORGAN's the pipes: `fight.py` 1323 to 1362 (`hazard_beat`, `_to_ropes`, `fire_line`) and the
//! lines' half of `hazards_step` (1393 to 1416).
//!
//! Every `every` beats (a haste divides it) a row of pipes erupts from the floor, from the ball's surface toward the
//! enemy as it stands now. The eruption front runs out at `speed`; each pipe stays up `linger` seconds and then sinks;
//! the row stops at the ropes. A row hits once (a capsule test, half-width `width / 2`); a landed row grows the next
//! rows' length (`blade_len`) by `grow` up to `max_len`.
//!
//! `lay_string` and `strings` (1364 to 1391, 1417 to 1434) belong to the older `trap` kind: `line` never reaches
//! them (`hazard_beat` lays a string only for a weapon that is not a `line`), so they are not ported.
//!
//! Python keeps the rows in `Fight.lines` and their ids in `Fight.next_hz`; here they are [`State`], `Fight::line`.
//! None of this code draws from the generator.

use bf_core::event::Value;
use bf_core::py;

use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::roster::{LineCfg, WeaponCfg};

/// A row of pipes (an entry of `Fight.lines`, the dict `fire_line` makes).
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// The fighter whose row it is.
    pub owner: usize,
    /// Its hazard id.
    pub id: i64,
    /// Where the row starts.
    pub p0: V2,
    /// Its direction, a unit vector.
    pub u: V2,
    /// Its length.
    pub l: f64,
    /// When it was fired.
    pub t0: f64,
    /// Units a second the front runs out.
    pub speed: f64,
    /// Seconds a pipe stays up.
    pub linger: f64,
    /// The row's width.
    pub width: f64,
    /// Its damage.
    pub dmg: f64,
    /// Its knockback.
    pub knock: f64,
    /// Whether it has hit: a row hits once.
    pub hit: bool,
}

/// Weapon kind `line`'s per-fight state (`Fight::line`): `Fight.lines` and `Fight.next_hz`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The rows still on the floor, oldest first.
    pub lines: Vec<Line>,
    /// The next hazard id.
    pub next_hz: i64,
}

/// `B.w['kind'] == 'line'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Line(_))
}

/// The line's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a line.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a LineCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Line(w) => w,
        other => panic!("fighter {k} ({}) has no line weapon: {other:?}", cfg.name),
    }
}

/// `hazard_beat(k, kb)` (1324 to 1332), from `run()` 5940 to 5941: on beat `kb` of the fight, every
/// `max(1, int(round(every / ab_mult(k))))` beats, the fighter fires a row.
pub fn beat(f: &mut Fight, k: usize, kb: i64) {
    let w = weapon(f, k);
    // int(round(x)): half to even; a quotient too large for an i64 saturates, and `kb % n` is `kb` either way
    let n = py::round_half_even(w.every / f.ab_mult(k)) as i64;
    if kb % n.max(1) == 0 {
        fire_line(f, k);
    }
}

/// `_to_ropes(p, u)` (1334 to 1343): the largest `s >= 0` with `p + s u` still inside the ropes (`|x|, |y| <= a`);
/// 0 if `p` is outside.
pub fn to_ropes(a: f64, p: V2, u: V2) -> f64 {
    let mut s = f64::INFINITY;
    for ax in 0..2 {
        if u[ax] > 1e-9 {
            s = py::py_min(s, (a - p[ax]) / u[ax]);
        } else if u[ax] < -1e-9 {
            s = py::py_min(s, (-a - p[ax]) / u[ax]);
        }
    }
    py::py_max(0.0, s)
}

/// `fire_line(k)` (1345 to 1362): a row from the ball's surface toward the enemy where it is now, as long as the
/// fighter's `blade_len` or as far as the ropes. Event `line`: the hazard id, the start, the direction, the length,
/// the speed, the linger and the width.
pub fn fire_line(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let (b, o) = (&f.balls[k], &f.balls[1 - k]);
    let d = geom::sub(o.p, b.p);
    let n = geom::hypot(d);
    // d / n if n > 1e-9 else B.v / (np.linalg.norm(B.v) + 1e-12)
    let u = if n > 1e-9 {
        geom::div(d, n)
    } else {
        geom::div(b.v, geom::norm(b.v) + 1e-12)
    };
    // B.p + u * (self.R.R + w.get('gap', 0.05))
    let p0 = geom::add(b.p, geom::scale(u, f.rules.r + w.gap));
    let l = py::py_min(b.blade_len, to_ropes(f.rules.a, p0, u));
    let hid = f.line.next_hz;
    f.line.next_hz += 1;
    f.line.lines.push(Line {
        owner: k,
        id: hid,
        p0,
        u,
        l,
        t0: f.t,
        speed: w.speed,
        linger: w.linger,
        width: w.width,
        dmg: w.dmg,
        knock: w.knock,
        hit: false,
    });
    let data = vec![
        Value::Int(hid),
        Value::Float(p0[0]),
        Value::Float(p0[1]),
        Value::Float(u[0]),
        Value::Float(u[1]),
        Value::Float(l),
        Value::Float(w.speed),
        Value::Float(w.linger),
        Value::Float(w.width),
    ];
    f.ev("line", k as i64, data);
}

/// Whether `self.lines` holds anything: the test before `hazards_step` (`run()` 6141; `strings` are the older `trap`
/// kind's).
pub fn live(f: &Fight) -> bool {
    !f.line.lines.is_empty()
}

/// `hazards_step(dt)` (1393 to 1416; the strings' half, 1417 to 1434, is the older `trap` kind's), from `run()` 6141
/// to 6142, while [`live`]: each row that has not hit tests the enemy against the part of it that is up (a capsule),
/// hits once (`pipe`, with the row's id; the knock is away from the row and 0.4 along it), and a landed row grows
/// its owner's `blade_len`. Rows whose last pipe has sunk are dropped.
pub fn hazards_step(f: &mut Fight, _dt: f64) {
    let r = f.rules.r;
    for i in 0..f.line.lines.len() {
        let ln = &f.line.lines[i];
        let k = ln.owner;
        let o_p = f.balls[1 - k].p;
        let el = f.t - ln.t0;
        let hi = py::py_min(ln.l, el * ln.speed);
        let lo = py::py_max(0.0, (el - ln.linger) * ln.speed);
        if ln.hit || hi <= lo {
            continue;
        }
        let rel = geom::sub(o_p, ln.p0);
        let s = py::py_min(hi, py::py_max(lo, geom::dot(rel, ln.u)));
        let off = geom::sub(rel, geom::scale(ln.u, s));
        let dist = geom::hypot(off);
        if dist < r + ln.width / 2.0 {
            let (u, dmg, knock, id) = (ln.u, ln.dmg, ln.knock, ln.id);
            f.line.lines[i].hit = true;
            let nv = if dist > 1e-6 {
                geom::div(off, dist)
            } else {
                [-u[1], u[0]]
            };
            // hx, hy = (O.p - nv * R).tolist()
            let h = geom::sub(o_p, geom::scale(nv, r));
            let landed = f.damage(k, 1 - k, dmg, "pipe", h, id);
            f.knock(1 - k, geom::add(nv, geom::scale(u, 0.4)), knock);
            if landed {
                let w = weapon(f, k);
                let b = &mut f.balls[k];
                b.blade_len = py::py_min(w.max_len, b.blade_len + w.grow);
            }
        }
    }
    let t = f.t;
    f.line
        .lines
        .retain(|ln| (t - ln.t0 - ln.linger) * ln.speed < ln.l - 1e-9);
}

#[cfg(test)]
mod tests {
    use super::to_ropes;

    #[test]
    fn to_ropes_is_the_way_to_the_nearest_rope_or_nothing_outside() {
        assert_eq!(to_ropes(3.5, [0.0, 0.0], [1.0, 0.0]), 3.5);
        assert_eq!(to_ropes(3.5, [1.0, -2.0], [0.0, -1.0]), 1.5);
        assert_eq!(to_ropes(3.5, [1.0, 1.5], [0.6, 0.8]), 2.5);
        assert_eq!(to_ropes(3.5, [4.0, 0.0], [1.0, 0.0]), 0.0);
        // a direction with no component past 1e-9 never reaches a rope
        assert_eq!(to_ropes(3.5, [0.0, 0.0], [0.0, 0.0]), f64::INFINITY);
    }
}
