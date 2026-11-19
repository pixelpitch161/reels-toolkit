//! Weapon kind `surf`, UKULELE's SURF'S UP: `fight.py` 502 to 527 (`surf_dist`, `fire_surf`) and the surfs of
//! `surf_rake_step` (588 to 609), on the `dc` 16th counter of `run()` (5964 to 5974, shared with the rake: one counter,
//! `Schedule::dc_next`).
//!
//! The strum raises a wave the ball rides at the enemy: a speed burst (its power grows with the wave's height
//! `blade_len`) and a front `hw` wide that crashes into the enemy once, pushing it along the wave; a landed wave on the
//! weapon's own grid grows the height. The generator is never drawn.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;
pub use bf_core::roster::kinds::surf::surf_dist;

use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::kinds::rake::aim_blend;
use crate::roster::{SurfCfg, WeaponCfg};

/// One entry of `self.surfs` (`fire_surf`'s dict).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wave {
    /// Who rides it.
    pub owner: usize,
    /// Where its front started.
    pub o: V2,
    /// Where it rolls (unit).
    pub u: V2,
    /// Square to `u`.
    pub n: V2,
    /// When it was strummed.
    pub t0: f64,
    /// The ride's speed burst.
    pub power: f64,
    /// The front's half width.
    pub hw: f64,
    /// Seconds it lives at most.
    pub life: f64,
    /// Its damage.
    pub dmg: f64,
    /// Its knockback.
    pub knock: f64,
    /// Whether it has hit.
    pub hit: bool,
    /// Whether a landed hit grows the wave (on the weapon's own grid).
    pub grow: bool,
}

/// This kind's per-fight state (`Fight::surf`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.surfs`: the waves ridden at the enemy (`fire_surf`'s dicts).
    pub surfs: Vec<Wave>,
}

/// `B.w['kind'] == 'surf'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Surf(_))
}

/// Whether `self.surfs` holds anything (Python's truth test of the list).
pub fn live(f: &Fight) -> bool {
    !f.surf.surfs.is_empty()
}

/// The surf's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a surf.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a SurfCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Surf(w) => w,
        other => panic!("fighter {k} ({}) has no surf weapon: {other:?}", cfg.name),
    }
}

/// The `dc` block of `run()` for a surf (5970 to 5974), after the caller's stun test: `dc_main = int(round(4 *
/// every))`, `dc_every = max(1, int(round(dc_main / ab_mult(k))))`, and on `dc_k % dc_every == 0` `fire_surf(k, dc_k %
/// dc_main == 0)`.
pub fn tick(f: &mut Fight, k: usize, dc_k: i64) {
    let w = weapon(f, k);
    let dc_main = py::round_half_even(4.0 * w.every) as i64;
    let dc_every = (py::round_half_even(dc_main as f64 / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(dc_k, dc_every) == 0 {
        fire_surf(f, k, py::imod_py(dc_k, dc_main) == 0);
    }
}

/// `fire_surf(k, main)` (506 to 527): the ball rides a new wave at the enemy (leading it by half the way). Event
/// `surf`: `int(main)`, the height, the front's start x, y, the aim, the power, the half width.
pub fn fire_surf(f: &mut Fight, k: usize, main: bool) {
    let w = weapon(f, k);
    let (rv, rr) = (f.rules.v, f.rules.r);
    let h = f.balls[k].blade_len;
    let power = (w.p0 + w.p_h * h) * if main { 1.0 } else { w.sub_power };
    let (bp, bv) = (f.balls[k].p, f.balls[k].v);
    let (op, ov) = (f.balls[1 - k].p, f.balls[1 - k].v);
    let dist = geom::hypot(geom::sub(op, bp));
    let lead = 0.5 * dist / (rv * (1.0 + power));
    let dd = geom::sub(geom::add(op, geom::scale(ov, lead)), bp);
    let ang = aim_blend(bv, dd, w.aim_k);
    let u = [dmath::cos(ang), dmath::sin(ang)];
    let b = &mut f.balls[k];
    b.boost = py::py_max(b.boost, power);
    b.v = geom::scale(u, rv * (1.0 + b.boost));
    let hw = w.w0 + w.w_h * h;
    let o = geom::add(bp, geom::scale(u, rr * 0.3));
    f.surf.surfs.push(Wave {
        owner: k,
        o,
        u,
        n: [-u[1], u[0]],
        t0: f.t,
        power,
        hw,
        life: w.life,
        dmg: w.dmg * if main { 1.0 } else { w.sub_dmg },
        knock: w.knock * (0.8 + 0.5 * h),
        hit: false,
        grow: main,
    });
    let data = vec![
        Value::Int(i64::from(main)),
        Value::Float(h),
        Value::Float(o[0]),
        Value::Float(o[1]),
        Value::Float(ang),
        Value::Float(power),
        Value::Float(hw),
    ];
    f.ev("surf", k as i64, data);
}

/// The surfs of `surf_rake_step(dt)` (588 to 609), before its strokes; `kinds::rake::surf_rake_step` calls it. A
/// wave ends when its front reaches the ropes or its life runs out (event `surf_end`: x, y, whether it crashed); the
/// first step the enemy is in front of the front (`-0.3 R` to `R` along it, within `hw + 0.6 R` across) it hits once.
pub fn surfs_step(f: &mut Fight) {
    let (r, v, a_) = (f.rules.r, f.rules.v, f.rules.a);
    let kt = f.rules.knock_t;
    let mut done = vec![false; f.surf.surfs.len()];
    for (i, gone) in done.iter_mut().enumerate() {
        let sf = f.surf.surfs[i].clone();
        let tau = f.t - sf.t0;
        let c = geom::add(sf.o, geom::scale(sf.u, surf_dist(tau, v, sf.power, kt)));
        if c[0].abs() >= a_ - 0.05 || c[1].abs() >= a_ - 0.05 || tau >= sf.life {
            *gone = true;
            let data = vec![
                Value::Float(c[0]),
                Value::Float(c[1]),
                Value::Int(i64::from(tau < sf.life)),
            ];
            f.ev("surf_end", sf.owner as i64, data);
            continue;
        }
        if sf.hit {
            continue;
        }
        let o = 1 - sf.owner;
        let op = f.balls[o].p;
        let rel = geom::sub(op, c);
        let (a, b) = (geom::dot(rel, sf.u), geom::dot(rel, sf.n));
        if -r * 0.3 <= a && a <= r && b.abs() <= sf.hw + r * 0.6 {
            f.surf.surfs[i].hit = true;
            let h = geom::sub(op, geom::scale(sf.u, r));
            let landed = f.damage(sf.owner, o, sf.dmg, "surf", h, i64::from(sf.grow));
            f.knock(o, sf.u, sf.knock);
            if landed && sf.grow {
                let w = weapon(f, sf.owner);
                let bw = &mut f.balls[sf.owner];
                bw.blade_len = py::py_min(w.max_h, bw.blade_len + w.grow);
            }
        }
    }
    let mut it = done.into_iter();
    f.surf.surfs.retain(|_| !it.next().unwrap_or(false));
}
