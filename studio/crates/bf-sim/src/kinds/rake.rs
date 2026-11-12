//! Weapon kind `rake`, ACOUSTIC's SIX-STRING RAKE: `fight.py` 497 to 700: `fire_rake` (528 to 543), `aim_blend`
//! (545 to 554), the rake branch of `surf_rake_step` (610 to 629), and the `dc` 16th block of `run()` (5964 to 5974).
//! `rake_lateral` and `sweep_front` (556 to 564) are `bf_core::roster::kinds::rake`'s (the look calls them too).
//!
//! Every `every` beats on the dash / cone 16th grid (haste divides it) six parallel strings, `blade_len` long and
//! aimed at the enemy blended with the ball's heading (`aim_k`), sweep from `-dir * sweep` to `dir * sweep` across
//! the ring in front of the ball in `stroke` seconds, the direction alternating down (+1) and up (-1), the first a
//! down. One hit per stroke flicks the enemy along it; a landed stroke on the weapon's own grid lengthens the strings.
//!
//! The functions `dash_cone_step` and `surf_rake_step` are shared in `fight.py` with the older kinds `dash`, `cone`,
//! `surf` and the ability `spout`: only the rake's branch is ported here (and the whips and sweeps of
//! `abilities::x33`, which `surf_rake_step` steps); the others are `SLOT` comments at their places. Python hangs
//! `rake_dir` on the ball and the strokes on the fight (`self.rakes`); here both are [`State`], `Fight::rake`. The
//! generator is never drawn.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;
pub use bf_core::roster::kinds::rake::{rake_lateral, sweep_front};

use crate::abilities::x33;
use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::roster::{RakeCfg, WeaponCfg};

/// One stroke in flight (`self.rakes`' dict, made by `fire_rake`).
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    /// Who strums.
    pub owner: usize,
    /// Where the ball was at the strum (`c`).
    pub c: V2,
    /// The aim (radians).
    pub aim: f64,
    /// `[cos aim, sin aim]`.
    pub u: V2,
    /// `[-sin aim, cos aim]`.
    pub n: V2,
    /// +1 down, -1 up.
    pub dir: f64,
    /// The strings' length (`blade_len` at the strum).
    pub l: f64,
    /// The sweep to either side (`w['sweep']`).
    pub s: f64,
    /// The hit band's half width beyond the bundle's middle (`2.5 * gap`).
    pub hb: f64,
    /// When it was strummed.
    pub t0: f64,
    /// Its length in seconds (`w['stroke']`).
    pub dur: f64,
    /// Its damage.
    pub dmg: f64,
    /// Its knockback.
    pub knock: f64,
    /// Whether it has hit.
    pub hit: bool,
    /// Whether a landed hit lengthens the strings (a stroke on the weapon's own grid).
    pub grow: bool,
    /// The bundle's lateral place at the previous step.
    pub s_prev: f64,
}

/// Weapon kind `rake`'s per-fight state (`Fight::rake`).
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// `B.rake_dir` by fighter: `Ball.__init__` sets -1.0, so the first stroke is a down (+1).
    pub rake_dir: [f64; 2],
    /// `self.rakes`: the strokes in flight, in the order they were strummed.
    pub rakes: Vec<Stroke>,
}

impl Default for State {
    fn default() -> Self {
        State {
            rake_dir: [-1.0; 2],
            rakes: Vec::new(),
        }
    }
}

/// `B.w['kind'] == 'rake'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Rake(_))
}

/// The rake's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a rake.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a RakeCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Rake(w) => w,
        other => panic!("fighter {k} ({}) has no rake weapon: {other:?}", cfg.name),
    }
}

/// The `dc` 16th block of `run()` (5970 to 5974) for one fighter, after the caller's stun test: `dc_main =
/// int(round(4 * every))`, `dc_every = max(1, int(round(dc_main / ab_mult(k))))`, and on `dc_k % dc_every == 0`
/// `fire_rake(k, dc_k % dc_main == 0)`. (5974 calls `fire_<kind>`: the `dash`, `cone` and `surf` of older kinds. SLOT.)
pub fn tick(f: &mut Fight, k: usize, dc_k: i64) {
    let w = weapon(f, k);
    // the roster refuses a grid that rounds to 0 (Python's `dc_k % dc_main` raises)
    let dc_main = py::round_half_even(4.0 * w.every) as i64;
    let dc_every = (py::round_half_even(dc_main as f64 / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(dc_k, dc_every) == 0 {
        fire_rake(f, k, py::imod_py(dc_k, dc_main) == 0);
    }
}

/// `aim_blend(B, d, k_aim)` (545 to 554): toward `d` (the enemy) when `k_aim >= 1`, else the unit vectors toward
/// `d` and along the ball's velocity `v`, blended `k_aim` to `1 - k_aim`.
pub fn aim_blend(v: V2, d: V2, k_aim: f64) -> f64 {
    if k_aim >= 1.0 {
        return dmath::atan2(d[1], d[0]);
    }
    // dh = d / (np.hypot(*d) + 1e-9); vh = B.v / (np.hypot(*B.v) + 1e-9)
    let dh = geom::div(d, geom::hypot(d) + 1e-9);
    let vh = geom::div(v, geom::hypot(v) + 1e-9);
    // m = k_aim * dh + (1 - k_aim) * vh
    let m = geom::add(geom::scale(dh, k_aim), geom::scale(vh, 1.0 - k_aim));
    dmath::atan2(m[1], m[0])
}

/// `fire_rake(k, main)` (528 to 543): a stroke from where the ball is, aimed by [`aim_blend`], the direction
/// flipped. Event `rake`: `int(main)`, the length, x, y, the aim, the direction, the sweep, the stroke's seconds.
pub fn fire_rake(f: &mut Fight, k: usize, main: bool) {
    let w = weapon(f, k);
    let (bp, bv, op) = (f.balls[k].p, f.balls[k].v, f.balls[1 - k].p);
    let aim = aim_blend(bv, geom::sub(op, bp), w.aim_k);
    let dir = -f.rake.rake_dir[k];
    f.rake.rake_dir[k] = dir;
    let s = w.sweep;
    let (sa, ca) = (dmath::sin(aim), dmath::cos(aim));
    let l = f.balls[k].blade_len;
    f.rake.rakes.push(Stroke {
        owner: k,
        c: bp,
        aim,
        u: [ca, sa],
        n: [-sa, ca],
        dir,
        l,
        s,
        hb: 2.5 * w.gap,
        t0: f.t,
        dur: w.stroke,
        dmg: w.dmg * if main { 1.0 } else { w.sub_dmg },
        knock: w.knock * if main { 1.0 } else { 0.6 },
        hit: false,
        grow: main,
        s_prev: -dir * s,
    });
    let data = vec![
        Value::Int(i64::from(main)),
        Value::Float(l),
        Value::Float(bp[0]),
        Value::Float(bp[1]),
        Value::Float(aim),
        Value::Float(dir),
        Value::Float(s),
        Value::Float(w.stroke),
    ];
    f.ev("rake", k as i64, data);
}

/// `dash_cone_step(dt)` (457 to 495), from `run()` 6195 to 6196 (`if dc`). It has no rake branch: for a fight whose
/// `dc` fighters are rakes it does nothing.
pub fn dash_cone_step(f: &mut Fight, dt: f64) {
    let _ = (f, dt);
    // 458 to 476: for each ball whose weapon is a `dash` with time left: the dash's contact (an older kind). SLOT.
    // 477 to 495: `self.cones`, filled only by `fire_cone` (an older kind's weapon). SLOT.
}

/// `surf_rake_step(dt)` (585 to 700), from `run()` 6197 to 6198 (`if dc or self.whips or self.sweeps or
/// self.spouts`): the strokes, then the whips and the sweeps ([`x33::step`]), in Python's order.
pub fn surf_rake_step(f: &mut Fight, dt: f64) {
    // 586 to 587: R, V, A (V and A are read only by the surf and spout branches)
    // 588 to 609: `self.surfs`, filled only by `fire_surf` (an older kind's weapon). SLOT.
    rakes_step(f);
    // 630 to 670: the whips and the sweeps
    x33::step(f);
    // 671 to 699: `self.spouts`, filled only by the ability `spout` (an older kind's). SLOT.
    let _ = dt;
}

/// The rake branch of `surf_rake_step` (610 to 629): each stroke moves across; the first step its band (the
/// bundle's sweep since the last step, `hb` wider each way) overlaps the enemy in front of the ball, it hits: `hit`
/// with `e[7] = 'rake'` at the strings' crossing (`e[8]` 1 for a down stroke), a knock along the stroke and a little
/// forward, and a landed stroke on the grid adds `grow` up to `max_len`. A stroke is dropped after its time.
fn rakes_step(f: &mut Fight) {
    let r = f.rules.r;
    for i in 0..f.rake.rakes.len() {
        let rk = f.rake.rakes[i].clone();
        let tau = f.t - rk.t0;
        let s_now = rake_lateral(tau, rk.dur, rk.s, rk.dir);
        let s_prev = rk.s_prev;
        f.rake.rakes[i].s_prev = s_now;
        if rk.hit {
            continue;
        }
        let o = 1 - rk.owner;
        let rel = geom::sub(f.balls[o].p, rk.c);
        let (a, b) = (geom::dot(rel, rk.u), geom::dot(rel, rk.n));
        let lo = py::py_min(s_prev, s_now) - rk.hb;
        let hi = py::py_max(s_prev, s_now) + rk.hb;
        if r * 0.3 <= a && a <= r + rk.l + r * 0.5 && hi >= b - r && lo <= b + r {
            f.rake.rakes[i].hit = true;
            // hx, hy = (Rk['c'] + Rk['u'] * a + Rk['n'] * s_now).tolist()
            let h = geom::add(geom::add(rk.c, geom::scale(rk.u, a)), geom::scale(rk.n, s_now));
            let landed = f.damage(rk.owner, o, rk.dmg, "rake", h, i64::from(rk.dir > 0.0));
            // self.knock(o, Rk['n'] * Rk['dir'] + 0.25 * Rk['u'], Rk['knock'])
            f.knock(
                o,
                geom::add(geom::scale(rk.n, rk.dir), geom::scale(rk.u, 0.25)),
                rk.knock,
            );
            if landed && rk.grow {
                let w = weapon(f, rk.owner);
                let bw = &mut f.balls[rk.owner];
                bw.blade_len = py::py_min(w.max_len, bw.blade_len + w.grow);
            }
        }
    }
    let t = f.t;
    f.rake.rakes.retain(|rk| t - rk.t0 < rk.dur + 1e-9);
}

#[cfg(test)]
mod tests {
    //! The expected numbers are what `fight.py` gave on the reference PC
    //! (`tools/studio_export/unit_checks/rake_unit_vectors.py`); the fights themselves are proven by the harness
    //! (`bf-harness/tests/fighter_acoustic.rs`).
    use super::*;

    #[test]
    fn aim_blend_is_pythons() {
        for (d, v, k, want) in [
            ([1.3, -0.4], [0.2, 3.1], 0.47, 0x3fe5c0be22bc5b7du64),
            ([-2.0, 0.5], [-1.9, -2.2], 0.47, 0xc006623497502a20),
            ([0.3, 0.9], [2.9, 0.8], 1.0, 0x3ff3fc176b7a8560),
            ([-1.1, -1.7], [0.0, 0.0], 0.47, 0xc001292acb9e5e98),
            ([0.0, 0.0], [1.0, 2.0], 0.3, 0x3ff1b6e192ebbe44),
        ] {
            let got = aim_blend(v, d, k);
            assert_eq!(got.to_bits(), want, "aim_blend(v={v:?}, d={d:?}, {k}) = {got}");
        }
    }

    #[test]
    fn the_first_stroke_is_a_down() {
        assert_eq!(State::default().rake_dir, [-1.0, -1.0]);
    }
}
