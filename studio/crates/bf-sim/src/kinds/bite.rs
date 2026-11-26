//! Weapon kind `bite`, ACCORDION's BELLOWS BITE: `fight.py` 1868 to 1904 (`bite_fire`, `bite_snap`) and its branch of
//! `grid16_fire` (1487 to 1507).
//!
//! On its own 16th grid (a haste divides it) the two halves fly apart with the bellows stretched between them like
//! jaws, aimed (and locked) at where the enemy will be `open16` 16ths later; then they snap shut on a 16th. An enemy
//! inside the mouth (within the reach and the jaws' half-angle of the locked aim) is bitten; landed main bites grow
//! the reach (`blade_len`). Python hangs the open bite on the ball (`B.bite`); here it is [`State`], indexed by
//! fighter. It draws nothing from the generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::Fight;
use crate::geom;
use crate::roster::{BiteCfg, WeaponCfg};

/// An open bite (`B.bite`, the dict `bite_fire` makes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bite {
    /// When the jaws opened.
    pub t0: f64,
    /// When they snap shut.
    pub t_snap: f64,
    /// The locked aim (radians).
    pub aim: f64,
    /// The reach at the opening.
    pub reach: f64,
    /// Whether it is on the weapon's own grid (a main bite).
    pub main: bool,
}

/// This kind's per-fight state (`Fight::bite`): each fighter's open bite.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.bite`, per fighter.
    pub open: [Option<Bite>; 2],
}

/// `B.w['kind'] == 'bite'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Bite(_))
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a BiteCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Bite(w) => w,
        _ => unreachable!("not a biter"),
    }
}

/// Its branch of `grid16_fire(k, k16)` (1487 to 1507): the jaws due to close snap on this 16th, then the grid test
/// and a new bite when none is open.
pub fn grid16_fire(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    if let Some(b) = f.bite.open[k]
        && f.t + f.rules.dt >= b.t_snap
    {
        snap(f, k);
    }
    let main = (py::round_half_even(4.0 * w.every) as i64).max(1);
    let step16 = (py::round_half_even(main as f64 / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(k16, step16) != 0 {
        return;
    }
    if f.bite.open[k].is_none() {
        fire(f, k, py::imod_py(k16, main) == 0);
    }
}

/// `bite_fire(k, main)` (1868 to 1879).
fn fire(f: &mut Fight, k: usize, main: bool) {
    let w = weapon(f, k);
    let t_open = w.open16 * f.rules.beat / 4.0;
    let (b, o) = (&f.balls[k], &f.balls[1 - k]);
    let tgt = geom::add(o.p, geom::scale(o.v, t_open * w.lead));
    let d = geom::sub(tgt, b.p);
    let aim = dmath::atan2(d[1], d[0]);
    let reach = b.blade_len;
    f.bite.open[k] = Some(Bite {
        t0: f.t,
        t_snap: f.t + t_open,
        aim,
        reach,
        main,
    });
    f.ev(
        "bite",
        k as i64,
        vec![
            Value::Int(i64::from(main)),
            Value::Float(aim),
            Value::Float(reach),
            Value::Float(t_open),
        ],
    );
}

/// `bite_snap(k)` (1880 to 1904): the jaws snap shut.
fn snap(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let Some(bt) = f.bite.open[k].take() else {
        return;
    };
    let r = f.rules.r;
    let ro = crate::fight::radius(f, 1 - k);
    let bp = f.balls[k].p;
    let rel = geom::sub(f.balls[1 - k].p, bp);
    let dist = geom::hypot(rel);
    let phi = dmath::atan2(rel[1], rel[0]);
    let half = geom::radians(w.half) + dmath::asin(py::py_min(1.0, ro / py::py_max(dist, 1e-6)));
    let inside = dist - ro <= r + bt.reach && geom::wrap(phi - bt.aim).abs() <= half;
    let mut landed = false;
    if inside {
        let at = geom::add(bp, geom::scale(geom::div(rel, py::py_max(dist, 1e-6)), dist - ro));
        let dmg = w.dmg * if bt.main { 1.0 } else { w.sub_dmg };
        landed = f.damage(k, 1 - k, dmg, "bite", at, i64::from(bt.main));
        f.knock(1 - k, rel, w.knock * if bt.main { 1.0 } else { 0.6 });
        if landed && bt.main {
            let b = &mut f.balls[k];
            b.blade_len = py::py_min(w.max_reach, b.blade_len + w.grow);
        }
    }
    f.ev(
        "snap",
        k as i64,
        vec![
            Value::Int(i64::from(inside)),
            Value::Int(i64::from(landed)),
            Value::Float(bt.aim),
            Value::Float(bt.reach),
        ],
    );
}
