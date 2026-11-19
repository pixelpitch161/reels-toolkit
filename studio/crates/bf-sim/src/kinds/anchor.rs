//! Weapon kind `anchor`, BASS CLARINET's ENDPIN ANCHOR: `fight.py` 2568 to 2658 (`anchor_setup`, `anchor_plant`,
//! `anchor_release`, `anchor_tick`, `anchor_step`).
//!
//! Every `every` beats (on the 16th grid; haste divides it) the fighter drives its floor peg into the canvas an
//! orbit ahead of it, toward the enemy's spot `lead` x the whirl's first half ahead (the peg clipped so the circle
//! stays inside the ropes), and whirls round it like a hammer-thrower: `turns` turns in `whirl_beats` beats, the
//! radius easing from where it was to the orbit over `settle` s. The arm from the peg to the far side of the ball
//! sweeps the whole circle: an enemy it crosses within the reach is smashed (`dmg`, knocked along the swing and
//! outward by `knock`; then `cool` s of rest for that arm). Then it pulls the peg and is flung off along the tangent
//! at the shared speed with a `release` boost. A landed smash of the fighter's own whirl adds `grow` to the orbit
//! (`blade_len`, up to `max_orbit`). While anchored the ball skips the normal motion (`move`). With a twin
//! (`abilities::twin`) a ghost arm sweeps the far side of the peg.
//!
//! Events: `anchor` (owner, the peg, the orbit, the whirl's end), `pull` (owner, where it is, its velocity); a smash
//! is a `hit` of kind `anchor` whose sub-number is 0 for the arm and 1 for the twin's. Recorded as `ANC` when a
//! fighter has the kind.
//!
//! A whirl belongs to "ball `k`" and is run with the weapon settings it was planted with ([`Anc::w`]): IDENTITY
//! CRISIS plants one on the copier's ball with the enemy's settings and orbit ([`anchor_plant`]), which whirls,
//! smashes and is pulled like any other but grows no orbit ([`Anc::own`]). None of this code draws from the fight's
//! generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::{Fight, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AnchorCfg, WeaponCfg};

/// Weapon kind `anchor`'s per-fight state (`Fight::anchor`): nothing (the orbit is the ball's `blade_len`, the whirl
/// the ball's [`Anc`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.anc` while the ball whirls round a peg (`anchor_plant` 2599): Python's `dict(c=c, th=th, r0=r0, orbit=orbit,
/// om=om, t0=t, t_end=t + tw, cool=[0.0, 0.0], w=w)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Anc {
    /// The peg.
    pub c: V2,
    /// The angle of the ball round the peg (radians; it is never wrapped).
    pub th: f64,
    /// The radius the whirl started at.
    pub r0: f64,
    /// The radius it eases to.
    pub orbit: f64,
    /// Radians a second, signed.
    pub om: f64,
    /// When the peg went in.
    pub t0: f64,
    /// When it is pulled.
    pub t_end: f64,
    /// Seconds of rest left for the arm (0) and the twin's arm (1).
    pub cool: [f64; 2],
    /// The weapon settings the whirl runs on (the fighter's own, or the ones a copy took from the enemy).
    pub w: AnchorCfg,
    /// `w is B.w`: the whirl runs on the ball's own weapon (a landed smash then grows the orbit).
    pub own: bool,
}

/// `B.w['kind'] == 'anchor'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Anchor(_))
}

/// Fighter `k`'s anchor settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not an anchor (Python raises `KeyError` on the first setting it reads).
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a AnchorCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Anchor(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not an anchor", other.kind()),
    }
}

/// `anchor_setup(k)` (2579 to 2583), from `run()` 5818 to 5820 (5821 to 5823, `B.anc = None` on both balls, is
/// `Ball::anc`'s default): the orbit starts at `base`, no whirl.
pub fn setup(f: &mut Fight, k: usize) {
    let base = weapon(f, k).base;
    f.balls[k].blade_len = base;
    f.balls[k].anc = None;
}

/// `anchor_tick(k, k16)` (2611 to 2619), from `run()` 6030 to 6034: on its grid a fighter that is neither stunned
/// (or sunk in shadow) nor whirling drives the peg in.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    if stunned(f, k) || f.balls[k].anc.is_some() {
        return;
    }
    let w = weapon(f, k);
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    let ev16 = (py::round_half_even(main16 as f64 / f.ab_mult(k)) as i64).max(1);
    if k16 > 0 && py::imod_py(k16, ev16) == 0 {
        let orbit = f.balls[k].blade_len;
        anchor_plant(f, k, w, orbit);
    }
}

/// `anchor_step(k, dt)` (2620 to 2658), from `run()` 6159 to 6162: for either ball whose `anc` is set (a copied whirl
/// too), while a fighter is an anchorer: the ball goes round the peg, the arm (and a twin's) smashes an enemy it
/// sweeps over, and at the whirl's end the peg is pulled.
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let o = 1 - k;
    let (r_ball, a) = (f.rules.r, f.rules.a);
    let t = f.t;
    let Some(an) = f.balls[k].anc.as_mut() else {
        return;
    };
    let th0 = an.th;
    let dth = an.om * dt;
    an.th = th0 + dth;
    let th = an.th;
    // u_ = min(1.0, (self.t - An['t0']) / w.get('settle', 0.12))
    let u = py::py_min(1.0, (t - an.t0) / an.w.settle);
    let r = an.r0 + (an.orbit - an.r0) * (u * u * (3.0 - 2.0 * u));
    let (c, om) = (an.c, an.om);
    // An['cool'] = [max(0.0, c_ - dt) for c_ in An['cool']]
    an.cool = an.cool.map(|c_| py::py_max(0.0, c_ - dt));
    let cool = an.cool;
    let (dmg0, twin_dmg, knock, rest, own) = (an.w.dmg, an.w.twin_dmg, an.w.knock, an.w.cool, an.own);
    let (grow, max_orbit) = (an.w.grow, an.w.max_orbit);
    let t_end = an.t_end;
    let (cs, sn) = (dmath::cos(th), dmath::sin(th));
    let lim = a - r_ball;
    let b = &mut f.balls[k];
    // B.p = np.clip(An['c'] + r * np.array([cos, sin]), -(A - R), A - R)
    b.p = [
        geom::np_clip(c[0] + r * cs, -lim, lim),
        geom::np_clip(c[1] + r * sn, -lim, lim),
    ];
    // B.v = np.array([-sin, cos]) * An['om'] * max(r, 0.05)
    let far = py::py_max(r, 0.05);
    b.v = [-sn * om * far, cs * om * far];
    if py::py_min(cool[0], cool[1]) <= 0.0 {
        let op = f.balls[o].p;
        let rel = geom::sub(op, c);
        let dist = geom::hypot(rel);
        if dist <= r + 2.0 * r_ball && f.balls[o].shadow <= 0.0 {
            let phi = dmath::atan2(rel[1], rel[0]);
            let half = dmath::asin(py::py_min(1.0, r_ball / py::py_max(dist, 1e-6)));
            // MULTIPHONICS: the ghost twin's arm, on the far side of the peg
            let twin = f.twin.until[k] > t;
            let arms = [(th0, 0usize), (th0 + dmath::PI, 1usize)];
            for &(a0, ghost) in &arms[..if twin { 2 } else { 1 }] {
                if cool[ghost] <= 0.0 && geom::swept(py::fmod_py(a0, 2.0 * dmath::PI), dth, phi, half) {
                    let n = if dist > 1e-6 {
                        geom::div(rel, dist)
                    } else {
                        [dmath::cos(a0), dmath::sin(a0)]
                    };
                    // hx, hy = (O.p - n * R).tolist()
                    let h = geom::sub(op, geom::scale(n, r_ball));
                    let dmg = dmg0 * if ghost == 1 { twin_dmg } else { 1.0 };
                    let landed = f.damage(k, o, dmg, "anchor", h, ghost as i64);
                    // sw = np.array([-sin(a0 + dth), cos(a0 + dth)]) * (1.0 if dth >= 0 else -1.0)
                    let dir = if dth >= 0.0 { 1.0 } else { -1.0 };
                    let sw = [-dmath::sin(a0 + dth) * dir, dmath::cos(a0 + dth) * dir];
                    f.knock(o, geom::add(sw, geom::scale(n, 0.7)), knock);
                    // each arm rests on its own
                    if let Some(an) = f.balls[k].anc.as_mut() {
                        an.cool[ghost] = rest;
                    }
                    if landed && own {
                        let b = &mut f.balls[k];
                        b.blade_len = py::py_min(max_orbit, b.blade_len + grow);
                    }
                    break;
                }
            }
        }
    }
    if t + 1e-9 >= t_end {
        anchor_release(f, k);
    }
}

/// `anchor_plant(k, w, orbit)` (2584 to 2601): ball `k` drives the peg in and starts a whirl with the weapon settings
/// `w` at the radius `orbit`. IDENTITY CRISIS's signature copy calls it with the enemy's weapon and orbit
/// (`start_copy` 2818 to 2819).
pub fn anchor_plant(f: &mut Fight, k: usize, w: &AnchorCfg, orbit: f64) {
    let (b, o) = (&f.balls[k], &f.balls[1 - k]);
    let (r, a) = (f.rules.r, f.rules.a);
    let tw = w.whirl_beats * f.rules.beat;
    // tgt = O.p + O.v * tw * 0.5 * w.get('lead', 0.0)
    let tgt = geom::add(o.p, geom::scale(geom::scale(geom::scale(o.v, tw), 0.5), w.lead));
    let d = geom::sub(tgt, b.p);
    let dist = geom::hypot(d);
    // u = d / dist if dist > 1e-6 else B.v / (np.linalg.norm(B.v) + 1e-12)
    let u = if dist > 1e-6 {
        geom::div(d, dist)
    } else {
        geom::div(b.v, geom::norm(b.v) + 1e-12)
    };
    let lim = py::py_max(0.15, a - r - orbit - 0.02);
    // c = np.clip(B.p + u * orbit, -lim, lim)
    let c = [
        geom::np_clip(b.p[0] + u[0] * orbit, -lim, lim),
        geom::np_clip(b.p[1] + u[1] * orbit, -lim, lim),
    ];
    let rel = geom::sub(b.p, c);
    let r0 = geom::hypot(rel);
    let th = if r0 > 1e-6 {
        dmath::atan2(rel[1], rel[0])
    } else {
        dmath::atan2(-u[1], -u[0])
    };
    // om = B.spin_dir * w['turns'] * 2 * math.pi / tw
    let om = b.spin_dir * w.turns * 2.0 * dmath::PI / tw;
    let t_end = f.t + tw;
    // `w is B.w`: the settings are the ball's own weapon's (the same object, as Python asks)
    let own = matches!(&b.cfg.weapon, WeaponCfg::Anchor(mine) if std::ptr::eq(mine, w));
    let t0 = f.t;
    f.balls[k].anc = Some(Anc {
        c,
        th,
        r0,
        orbit,
        om,
        t0,
        t_end,
        cool: [0.0, 0.0],
        w: w.clone(),
        own,
    });
    let data = vec![
        Value::Float(c[0]),
        Value::Float(c[1]),
        Value::Float(orbit),
        Value::Float(t_end),
    ];
    f.ev("anchor", k as i64, data);
}

/// `anchor_release(k)` (2602 to 2609): the peg is pulled: the ball flies off along the tangent of its whirl at the
/// shared speed, with at least the weapon's `release` boost. LIGHTS OUT calls it too, to pull a whirl in progress
/// (`start_lights_out` 2770 to 2771).
///
/// # Panics
/// When ball `k` is not whirling (Python raises `TypeError` on `An['th']`); every caller asks first.
pub fn anchor_release(f: &mut Fight, k: usize) {
    let v_shared = f.rules.v;
    let b = &mut f.balls[k];
    let an = b.anc.take().expect("a whirl to pull");
    let dir = if an.om >= 0.0 { 1.0 } else { -1.0 };
    // tang = np.array([-sin(th), cos(th)]) * (1.0 if om >= 0 else -1.0); B.v = tang * self.R.V
    b.v = [-dmath::sin(an.th) * dir * v_shared, dmath::cos(an.th) * dir * v_shared];
    b.boost = py::py_max(b.boost, an.w.release);
    let data = vec![
        Value::Float(b.p[0]),
        Value::Float(b.p[1]),
        Value::Float(b.v[0]),
        Value::Float(b.v[1]),
    ];
    f.ev("pull", k as i64, data);
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5825 to 5826: `ANC` when a fighter's
/// weapon is an anchor (a copied whirl alone adds no key).
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) { &["ANC"] } else { &[] }
}

/// This kind's rows of one frame: `snap`, `ANC` (5569 to 5576), into `out` (only the keys [`record_keys`] gave): each
/// ball's (1, the peg's x and y, the angle, the ball's distance from the peg, 1 with a twin's arm else 0), NaN for a
/// ball that does not whirl.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut row = [f64::NAN; 12];
    for (j, b) in f.balls.iter().enumerate() {
        if let Some(an) = &b.anc {
            let twin = if f.twin.until[j] > f.t { 1.0 } else { 0.0 };
            // float(np.hypot(*(Bj.p - An['c'])))
            let r = geom::hypot(geom::sub(b.p, an.c));
            row[6 * j..6 * j + 6].copy_from_slice(&[1.0, an.c[0], an.c[1], an.th, r, twin]);
        }
    }
    out.push_f64("ANC", &[2, 6], &row);
}

#[cfg(test)]
mod tests;
