//! Weapon kind `jump`, CLARINET's TWELFTH JUMP: `fight.py` 2338 to 2424 (`jump_setup`, `_jump`, `jump_tick`), and the
//! ability `rhapsody` (`trigger` 2930 to 2932; its chain runs in `jump_tick`).
//!
//! The register key makes the clarinet jump a twelfth, so the ball jumps too: every `every` beats (on the 16th grid,
//! from the second beat of the fight; haste divides it) it vanishes and reappears past the enemy. The target locks
//! `lock16` 16ths before the jump (the enemy's spot then, plus `lead` x its motion until the jump); the ball lands
//! `land` beyond it, turned a seeded angle of at most `jitter` degrees off the line, at most its reach (`blade_len`:
//! `base` to `max_reach`, `grow` more per landed cut) from where it was. The zip line from where it was to where it
//! lands cuts the enemy if it passes within a ball radius plus `cut` of it (`dmg`, `knock`): an enemy that moved, a
//! short jump or a wide landing misses. The ball keeps its speed, heading along the jump; it never lands inside the
//! enemy.
//!
//! RHAPSODY is `n` jumps `step16` 16ths apart zig-zagging `zig` degrees either side through the enemy as it is (no
//! lock), `land` past it, at most `reach` long, each a cut; regular jumps pause meanwhile, and it grows no reach.
//!
//! Event: `jump`, owner, `x0`, `y0`, `x1`, `y1`, hit (1 or 0), 0 for a jump or 1 for a jump of the RHAPSODY chain.
//! A landed cut is a `hit` of kind `cut` with the same 0 or 1 as its sub-number.

use bf_core::event::Value;
use bf_core::{dmath, py};

use crate::abilities::param;
use crate::fight::{Fight, stunned};
use crate::geom::{self, V2};
use crate::roster::{AbilityCfg, JumpCfg, WeaponCfg};

/// A running RHAPSODY chain (`B.rhap`): Python's `dict(ab=ab, n=ab['n'], side=1.0, next=None)`, with the numbers the
/// chain reads from `ab` held by value.
#[derive(Clone, Debug, PartialEq)]
pub struct Rhap {
    /// Jumps left (`ab['n']` at the start; a number as the roster gives it).
    pub n: f64,
    /// Which side of the enemy the next jump turns to: 1.0 or -1.0.
    pub side: f64,
    /// The 16th the next jump of the chain comes at (`None` before the first: it jumps on the next 16th).
    pub next: Option<f64>,
    /// `ab['step16']`: 16ths between two jumps of the chain.
    pub step16: f64,
    /// `ab['zig']`: degrees each jump is turned off the line through the enemy.
    pub zig: f64,
    /// `ab['land']`.
    pub land: f64,
    /// `ab['reach']`.
    pub reach: f64,
    /// `ab['dmg']`.
    pub dmg: f64,
    /// `ab['knock']`.
    pub knock: f64,
    /// `ab.get('cut')`: `None` takes the weapon's `cut` when the jump is made (`ab.get('cut', w['cut'])`).
    pub cut: Option<f64>,
}

/// Weapon kind `jump`'s per-fight state (`Fight::jump`): what `fight.py` hangs on each ball for this kind, by
/// fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.jump_tgt`: the locked target of the next jump (`None` until it locks; the enemy's spot is used then).
    pub jump_tgt: [Option<V2>; 2],
    /// `B.rhap`: the running RHAPSODY chain.
    pub rhap: [Option<Rhap>; 2],
}

/// `B.w['kind'] == 'jump'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Jump(_))
}

/// Fighter `k`'s jump settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a jump (Python raises `KeyError` on the first setting it reads).
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a JumpCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Jump(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not a jump", other.kind()),
    }
}

/// `jump_setup(k)` (2349 to 2354), from `run()` 5793 to 5795: the reach starts at `base`, no target, no chain.
pub fn setup(f: &mut Fight, k: usize) {
    let base = weapon(f, k).base;
    f.balls[k].blade_len = base;
    f.jump.jump_tgt[k] = None;
    f.jump.rhap[k] = None;
}

/// What one jump is made with: `_jump`'s arguments after `k`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Jump {
    /// The spot it jumps past.
    pub tgt: V2,
    /// Degrees the landing line is turned off the line to the target.
    pub theta: f64,
    /// How far beyond the target it lands.
    pub land: f64,
    /// The longest the jump may be.
    pub reach: f64,
    /// Damage of a cut.
    pub dmg: f64,
    /// Knockback of a cut.
    pub knock: f64,
    /// The zip line cuts an enemy within a ball radius plus this of it.
    pub cut: f64,
    /// 0 for a jump, 1 for a jump of the RHAPSODY chain (the last field of the event, the cut's sub-number).
    pub kind: i64,
    /// Whether a landed cut grows the reach.
    pub grow: bool,
}

/// `_jump(k, tgt, theta, land, reach, dmg, knock, cut, kind, grow)` (2359 to 2394): the ball jumps `land` past `tgt`
/// along the line to it turned by `theta` degrees, at most `reach` far, inside the ropes and never inside the enemy;
/// the segment between its two spots cuts the enemy within `R + cut` of it.
pub fn jump(f: &mut Fight, k: usize, j: Jump) {
    let o = 1 - k;
    let r = f.rules.r;
    let op = f.balls[o].p;
    let p0 = f.balls[k].p;
    let bv = f.balls[k].v;
    let d = geom::sub(j.tgt, p0);
    let dist = geom::hypot(d);
    // u = d / dist if dist > 1e-6 else B.v / (np.linalg.norm(B.v) + 1e-12)
    let u = if dist > 1e-6 {
        geom::div(d, dist)
    } else {
        geom::div(bv, geom::norm(bv) + 1e-12)
    };
    let a = geom::radians(j.theta);
    let (ca, sa) = (dmath::cos(a), dmath::sin(a));
    let ur = [u[0] * ca - u[1] * sa, u[0] * sa + u[1] * ca];
    // J = tgt + ur * land - p0
    let mut jv = geom::sub(geom::add(j.tgt, geom::scale(ur, j.land)), p0);
    let lj = geom::hypot(jv);
    if lj > j.reach {
        jv = geom::scale(jv, j.reach / lj);
    }
    let lim = f.rules.a - r;
    let clip = |p: V2| [geom::np_clip(p[0], -lim, lim), geom::np_clip(p[1], -lim, lim)];
    let mut p1 = clip(geom::add(p0, jv));
    let rel = geom::sub(p1, op);
    let dr = geom::hypot(rel);
    if dr < 2.0 * r + 0.05 {
        // never land inside the enemy: step out past it
        let out = if dr > 1e-6 { geom::div(rel, dr) } else { ur };
        p1 = clip(geom::add(op, geom::scale(out, 2.0 * r + 0.05)));
    }
    let seg = geom::sub(p1, p0);
    let l2 = geom::dot(seg, seg);
    // sc = min(1.0, max(0.0, float((O.p - p0) @ seg) / L2)) if L2 > 1e-12 else 0.0
    let sc = if l2 > 1e-12 {
        geom::py_min(1.0, geom::py_max(0.0, geom::dot(geom::sub(op, p0), seg) / l2))
    } else {
        0.0
    };
    let off = geom::sub(op, geom::add(p0, geom::scale(seg, sc)));
    let dd = geom::hypot(off);
    let hit = dd < r + j.cut;
    f.balls[k].p = p1;
    if l2 > 1e-12 {
        // B.v = seg / math.sqrt(L2) * float(np.linalg.norm(B.v))
        f.balls[k].v = geom::scale(geom::div(seg, l2.sqrt()), geom::norm(bv));
    }
    if hit {
        let n = if dd > 1e-6 {
            geom::div(off, dd)
        } else {
            geom::div([-seg[1], seg[0]], l2.sqrt() + 1e-12)
        };
        let h = geom::sub(op, geom::scale(n, r));
        let landed = f.damage(k, o, j.dmg, "cut", h, j.kind);
        // n + 0.5 * seg / (math.sqrt(L2) + 1e-12)
        let away = geom::add(n, geom::div(geom::scale(seg, 0.5), l2.sqrt() + 1e-12));
        f.knock(o, away, j.knock);
        if landed && j.grow {
            let w = weapon(f, k);
            let b = &mut f.balls[k];
            b.blade_len = geom::py_min(w.max_reach, b.blade_len + w.grow);
        }
    }
    let data = vec![
        Value::Float(p0[0]),
        Value::Float(p0[1]),
        Value::Float(p1[0]),
        Value::Float(p1[1]),
        Value::Int(i64::from(hit)),
        Value::Int(j.kind),
    ];
    f.ev("jump", k as i64, data);
}

/// `jump_tick(k, k16)` (2396 to 2424), from `run()` 6015 to 6019 (no stun test in the caller): on 16th `k16` of the
/// fight a stunned jumper loses its lock; a RHAPSODY chain makes its next jump when its 16th has come; otherwise the
/// target locks `lock16` 16ths before a jump and the jump comes on the weapon's grid, from the second beat on, with
/// one draw from the fight's generator for its jitter.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let o = 1 - k;
    let w = weapon(f, k);
    if stunned(f, k) {
        f.jump.jump_tgt[k] = None;
        return;
    }
    if let Some(rh) = f.jump.rhap[k].as_mut() {
        // RHAPSODY: the chain of zig-zag cuts
        if rh.next.is_none_or(|next| k16 as f64 >= next) {
            rh.next = Some(k16 as f64 + rh.step16);
            let j = Jump {
                tgt: f.balls[o].p,
                theta: rh.side * rh.zig,
                land: rh.land,
                reach: rh.reach,
                dmg: rh.dmg,
                knock: rh.knock,
                cut: rh.cut.unwrap_or(w.cut),
                kind: 1,
                grow: false,
            };
            jump(f, k, j);
            // nothing in `_jump` touches the chain, so it is still there
            let rh = f.jump.rhap[k].as_mut().expect("the chain that just jumped");
            rh.side = -rh.side;
            rh.n -= 1.0;
            if rh.n <= 0.0 {
                f.jump.rhap[k] = None;
            }
        }
        return;
    }
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    let ev16 = (py::round_half_even(main16 as f64 / f.ab_mult(k)) as i64).max(1);
    // lock = min(int(w.get('lock16', 1)), ev16 - 1)
    let lock = w.lock16.min(ev16 - 1);
    if lock > 0 && py::imod_py(k16 + lock, ev16) == 0 {
        // B.jump_tgt = O.p + O.v * (lock * self.R.beat / 4.0) * w.get('lead', 0.0)
        let ahead = lock as f64 * f.rules.beat / 4.0;
        let b = &f.balls[o];
        f.jump.jump_tgt[k] = Some(geom::add(b.p, geom::scale(geom::scale(b.v, ahead), w.lead)));
    }
    if k16 >= main16 && py::imod_py(k16, ev16) == 0 {
        let tgt = f.jump.jump_tgt[k].unwrap_or(f.balls[o].p);
        let th = f.rng.uniform(-1.0, 1.0) * w.jitter;
        let j = Jump {
            tgt,
            theta: th,
            land: w.land,
            reach: f.balls[k].blade_len,
            dmg: w.dmg,
            knock: w.knock,
            cut: w.cut,
            kind: 0,
            grow: true,
        };
        jump(f, k, j);
        f.jump.jump_tgt[k] = None;
    }
}

/// `trigger`, kind `rhapsody` (2930 to 2932): `B.jump_tgt = None`, `B.rhap = dict(ab=ab, n=ab['n'], side=1.0,
/// next=None)`. On a ball whose weapon is no jump the chain is never run (only `jump_tick` reads it), as in Python.
pub fn start_rhapsody(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    f.jump.jump_tgt[k] = None;
    f.jump.rhap[k] = Some(Rhap {
        n: param(ab, "n"),
        side: 1.0,
        next: None,
        step16: param(ab, "step16"),
        zig: param(ab, "zig"),
        land: param(ab, "land"),
        reach: param(ab, "reach"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        cut: ab.get("cut"),
    });
}

#[cfg(test)]
mod tests;
