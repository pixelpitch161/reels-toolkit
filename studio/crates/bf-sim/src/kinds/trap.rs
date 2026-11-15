//! Weapon kind `trap`, HARP's the strings: `fight.py` 1324 to 1332 (`hazard_beat`, shared with `line`), 1363 to 1392
//! (`lay_string`) and the strings' half of `hazards_step` (1417 to 1434). A string's id comes from the hazards' shared
//! counter (`self.next_hz`, `Fight::line`'s `next_hz`).
//!
//! Every `every` beats (a haste divides it) a string is stretched rope to rope across the ring, through the point `at`
//! of the way to the enemy and square to the line between them. An enemy crossing it takes `dmg` (once per `cool`
//! seconds per string) and stumbles forward by `knock`; a landed crossing grows the strings kept (`n_vol`) by `grow` up
//! to `n_max`, and laying one more than kept drops the oldest. None of this code draws from the generator.

use bf_core::event::Value;
use bf_core::py;

use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::roster::{TrapCfg, WeaponCfg};

/// One entry of `self.strings` (the dict `lay_string` makes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Strand {
    /// The fighter whose string it is.
    pub owner: usize,
    /// Its hazard id.
    pub id: i64,
    /// Its end on the ropes where it starts.
    pub a: V2,
    /// The unit normal (from the owner toward the enemy when laid).
    pub nrm: V2,
    /// The unit direction along the string.
    pub tdir: V2,
    /// Its length, rope to rope.
    pub len: f64,
    /// When it was laid.
    pub t0: f64,
    /// Seconds before it can hit again.
    pub cool: f64,
    /// The enemy's last signed distance from it (a crossing flips the sign).
    pub last: f64,
    /// Its damage.
    pub dmg: f64,
    /// Its stumble.
    pub knock: f64,
}

/// This kind's per-fight state (`Fight::trap`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.strings`: the strings across the ring (`lay_string`'s dicts).
    pub strings: Vec<Strand>,
}

/// `B.w['kind'] == 'trap'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Trap(_))
}

/// The trap's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a trap.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a TrapCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Trap(w) => w,
        other => panic!("fighter {k} ({}) has no trap weapon: {other:?}", cfg.name),
    }
}

/// Whether `self.strings` holds anything (Python's truth test of the list).
pub fn live(f: &Fight) -> bool {
    !f.trap.strings.is_empty()
}

/// `hazard_beat(k, kb)` for a `trap` (1324 to 1332), from `run()` 5940 to 5941: on beat `kb`, every
/// `max(1, int(round(every / ab_mult(k))))` beats, the fighter lays a string.
pub fn beat(f: &mut Fight, k: usize, kb: i64) {
    let w = weapon(f, k);
    let n = py::round_half_even(w.every / f.ab_mult(k)) as i64;
    if kb % n.max(1) == 0 {
        lay_string(f, k);
    }
}

/// `lay_string(k)` (1363 to 1392). Event `string`: the id and both ends; `string_off` (the id) for each dropped one.
pub fn lay_string(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let a_r = f.rules.a;
    let (b, o) = (&f.balls[k], &f.balls[1 - k]);
    let d = geom::sub(o.p, b.p);
    let n = geom::hypot(d);
    let nrm = if n > 1e-9 {
        geom::div(d, n)
    } else {
        geom::div(b.v, geom::norm(b.v) + 1e-12)
    };
    let tdir = [-nrm[1], nrm[0]];
    let c = geom::add(b.p, geom::scale(d, w.at));
    let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
    for ax in 0..2 {
        if tdir[ax].abs() > 1e-9 {
            let (s1, s2) = ((-a_r - c[ax]) / tdir[ax], (a_r - c[ax]) / tdir[ax]);
            lo = py::py_max(lo, py::py_min(s1, s2));
            hi = py::py_min(hi, py::py_max(s1, s2));
        }
    }
    let a = geom::add(c, geom::scale(tdir, lo));
    let bb = geom::add(c, geom::scale(tdir, hi));
    let last = geom::dot(geom::sub(o.p, a), nrm);
    let n_vol = b.n_vol;
    let hid = f.line.next_hz;
    f.line.next_hz += 1;
    f.trap.strings.push(Strand {
        owner: k,
        id: hid,
        a,
        nrm,
        tdir,
        len: hi - lo,
        t0: f.t,
        cool: 0.0,
        last,
        dmg: w.dmg,
        knock: w.knock,
    });
    let data = vec![
        Value::Int(hid),
        Value::Float(a[0]),
        Value::Float(a[1]),
        Value::Float(bb[0]),
        Value::Float(bb[1]),
    ];
    f.ev("string", k as i64, data);
    // mine[:max(0, len(mine) - cap)]: the oldest of the owner's strings beyond its cap
    let cap = ((n_vol + 1e-9) as i64).max(1) as usize;
    let mine: Vec<i64> = f.trap.strings.iter().filter(|s| s.owner == k).map(|s| s.id).collect();
    let drop = mine.len().saturating_sub(cap);
    for &id in &mine[..drop] {
        f.trap.strings.retain(|s| s.id != id);
        f.ev("string_off", k as i64, vec![Value::Int(id)]);
    }
}

/// The strings' half of `hazards_step(dt)` (1417 to 1434), from `run()` 6141 to 6142, after the pipes' half
/// (`kinds::line::hazards_step`).
pub fn strings_step(f: &mut Fight, dt: f64) {
    for i in 0..f.trap.strings.len() {
        let k = f.trap.strings[i].owner;
        let (o_p, o_v) = (f.balls[1 - k].p, f.balls[1 - k].v);
        let s = &mut f.trap.strings[i];
        s.cool = py::py_max(0.0, s.cool - dt);
        let rel = geom::sub(o_p, s.a);
        let sd = geom::dot(rel, s.nrm);
        let along = geom::dot(rel, s.tdir);
        if sd * s.last < 0.0 && s.cool <= 0.0 && (0.0..=s.len).contains(&along) {
            let (nrm, dmg, knock, id) = (s.nrm, s.dmg, s.knock, s.id);
            let h = geom::sub(o_p, geom::scale(nrm, sd));
            let landed = f.damage(k, 1 - k, dmg, "string", h, id);
            let w = weapon(f, k);
            f.trap.strings[i].cool = w.cool;
            if knock > 0.0 {
                f.knock(1 - k, o_v, knock);
            }
            if landed {
                let b = &mut f.balls[k];
                b.n_vol = py::py_min(w.n_max, b.n_vol + w.grow);
            }
        }
        if sd != 0.0 {
            f.trap.strings[i].last = sd;
        }
    }
}
