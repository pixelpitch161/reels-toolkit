//! Weapon kind `grapple`, CELLO's THE EMBRACE: `fight.py` 4041 to 4201 (`GRAB_STATES`, `grapple_setup`, `grapple_busy`,
//! `grapple_tick`, `_end_rush`, `_nearest_rope_angle`, `grapple_release`, `grapple_step`).
//!
//! Every `every` beats (`off16` 16ths after the bar line; a haste divides it) the fighter RUSHES: for `rush16` 16ths
//! it charges at `rush_speed` x the shared speed (`vmul`), steering at `track` rad/s for the enemy's spot `lead` x its
//! motion ahead. An enemy within its reach (`blade_len` beyond contact) is GRABBED (`dmg`; a shield blocks it, a
//! walking enemy slips it): the fighter plants itself (`pinned`) and holds the enemy (`grabbed`) at arm's length for
//! `hold_beats`, swinging it round (at least `turns` of a turn, accelerating, shuffling in off the ropes over the
//! first third) to face the rope nearest it (+-`jitter` degrees, seeded), then THROWS it that way (`dmg_throw`, a
//! knock boost of `power`). A rope hit of the thrown enemy within `rope_t` s is the rope slam (`dmg_rope`). A landed
//! throw lengthens the reach and raises the power.
//!
//! Python hangs this state on the ball (`B.gr`, `B.toss`, `B.power`); here it is [`State`], `Fight::grapple`, indexed
//! by fighter. The reach is the ball's `blade_len`. The generator is drawn once per grab (the throw's jitter) and
//! never otherwise. VIBRATO's shakes (`abilities::vibrato`) are dealt from [`tick`], as Python deals them.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::{self, Fight};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{GrappleCfg, WeaponCfg};

/// `GRAB_STATES`: what a grapple is doing, as the record numbers it (0 is idle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// `rush` (1): charging at the enemy.
    Rush,
    /// `hold` (2): holding and swinging it.
    Hold,
}

impl Phase {
    /// `GRAB_STATES[state]`.
    pub fn code(self) -> f64 {
        match self {
            Phase::Rush => 1.0,
            Phase::Hold => 2.0,
        }
    }
}

/// `B.gr`: the rush or the hold under way.
#[derive(Clone, Debug, PartialEq)]
pub struct Gr {
    /// `state`.
    pub state: Phase,
    /// When it started.
    pub t0: f64,
    /// When it ends (the rush gives up; the hold throws).
    pub t1: f64,
    /// The arm's angle now (the rush's heading, the swing's angle).
    pub a: f64,
    /// The hold's starting angle (the enemy's side when grabbed).
    pub a0: f64,
    /// The hold's throw angle (toward the nearest rope).
    pub a1: f64,
    /// The swing's signed sweep in radians.
    pub sweep: f64,
    /// Where the holder stood when it grabbed.
    pub c0: V2,
    /// Where it shuffles in to (at least 3 R off every rope).
    pub c1: V2,
}

/// `B.toss`: the thrown enemy still flying.
#[derive(Clone, Debug, PartialEq)]
pub struct Toss {
    /// When a rope hit is no longer a rope slam.
    pub t_end: f64,
    /// The first event not yet looked at.
    pub ev_i: usize,
}

/// Weapon kind `grapple`'s per-fight state (`Fight::grapple`): what `fight.py` hangs on the grappler's ball, indexed
/// by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.gr`.
    pub gr: [Option<Gr>; 2],
    /// `B.toss`.
    pub toss: [Option<Toss>; 2],
    /// `B.power`: the throw's knock boost (set by `grapple_setup`; only a grappler has it).
    pub power: [f64; 2],
}

/// `B.w['kind'] == 'grapple'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Grapple(_))
}

/// The grappler's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a grapple.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a GrappleCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Grapple(w) => w,
        other => panic!("fighter {k} ({}) has no grapple weapon: {other:?}", cfg.name),
    }
}

/// `grapple_setup(k)` (4058 to 4064), from `run()` 5864 to 5866: no rush, no toss, the reach and the power at the
/// roster's.
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    f.grapple.gr[k] = None;
    f.grapple.toss[k] = None;
    f.balls[k].blade_len = w.base;
    f.grapple.power[k] = w.power;
}

/// `grapple_busy(k)` (4065 to 4068): it cannot start a rush now (stunned, smashed or airborne).
fn busy(f: &Fight, k: usize) -> bool {
    let b = &f.balls[k];
    fight::stunned(f, k) || b.smashed > 0.0 || b.swan.is_some()
}

/// `grapple_tick(k, k16)` (4070 to 4090), from `run()` 6060 to 6064: on 16th `k16` of the fight, VIBRATO's shake of a
/// held enemy; then, unless a rush or hold is under way or it is busy, a rush on the weapon's grid. Event `rush`: the
/// aim, when the rush ends.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    let holding = matches!(&f.grapple.gr[k], Some(g) if g.state == Phase::Hold);
    if holding {
        // VIBRATO: every 16th of the hold while it lasts (`self.t < getattr(B, 'vib_until', -1.0)`)
        if let Some((until, dmg)) = f.vibrato.on[k] {
            if f.t < until {
                let at = f.balls[1 - k].p;
                f.damage(k, 1 - k, dmg, "vibrato", at, 0);
            }
        }
    }
    if f.grapple.gr[k].is_some() || busy(f, k) {
        return;
    }
    let m = f.ab_mult(k);
    // main16 = int(round(4 * w['every'])); (k16 - int(w.get('off16', 0))) % max(1, int(round(main16 / m)))
    let main16 = py::round_half_even(4.0 * w.every);
    let n = (py::round_half_even(main16 / m) as i64).max(1);
    if py::imod_py(k16 - w.off16.trunc() as i64, n) != 0 {
        return;
    }
    let rush = w.rush16 * f.rules.beat / 4.0;
    let (o, b) = (&f.balls[1 - k], &f.balls[k]);
    // d = O.p + O.v * rush * w.get('lead', 0.0) - B.p
    let d = geom::sub(geom::add(o.p, geom::scale(geom::scale(o.v, rush), w.lead)), b.p);
    let aim = dmath::atan2(d[1], d[0]);
    let v_shared = f.rules.v;
    let b = &mut f.balls[k];
    b.v = geom::scale([dmath::cos(aim), dmath::sin(aim)], v_shared);
    b.vmul = Some(w.rush_speed);
    let (t0, t1) = (f.t, f.t + rush);
    f.grapple.gr[k] = Some(Gr {
        state: Phase::Rush,
        t0,
        t1,
        a: aim,
        a0: 0.0,
        a1: 0.0,
        sweep: 0.0,
        c0: [0.0; 2],
        c1: [0.0; 2],
    });
    f.ev("rush", k as i64, vec![Value::Float(aim), Value::Float(t1)]);
}

/// `_end_rush(k, how, x, y)` (4091 to 4095): the rush is over. Event `grab`: how (1 grabbed, 0 missed, 2 slipped off
/// a walking enemy, 3 blocked), where, the reach.
fn end_rush(f: &mut Fight, k: usize, how: i64, x: f64, y: f64) {
    f.balls[k].vmul = None;
    f.grapple.gr[k] = None;
    let reach = f.balls[k].blade_len;
    let data = vec![Value::Int(how), Value::Float(x), Value::Float(y), Value::Float(reach)];
    f.ev("grab", k as i64, data);
}

/// `_nearest_rope_angle(p)` (4097 to 4100): the direction of the rope nearest `p`. Python takes the `min` of the
/// four `(distance, angle)` tuples: the nearest, and at a tie the smaller angle (the first of equals stays).
fn nearest_rope_angle(a: f64, p: V2) -> f64 {
    let sides = [
        (a - p[0], 0.0),
        (a + p[0], dmath::PI),
        (a - p[1], dmath::PI / 2.0),
        (a + p[1], -dmath::PI / 2.0),
    ];
    let mut best = sides[0];
    for s in &sides[1..] {
        // tuple `<`: the first items differ (`!=`), so they decide; else the second ones do
        let less = if s.0 != best.0 { s.0 < best.0 } else { s.1 < best.1 };
        if less {
            best = *s;
        }
    }
    best.1
}

/// `grapple_release(k, why)` (4102 to 4111): fighter `k` lets go without a throw (SWAN SONG taking off; SLAP BASS or
/// THE ELEPHANT breaking the grab call it for the holder): a hold frees both balls (event `release`, why), a rush only
/// drops its speed factor; nothing is under way afterwards. A fighter that is no grappler holds nothing, so nothing
/// happens to it but the (empty) state's reset.
pub fn release(f: &mut Fight, k: usize, why: &str) {
    match &f.grapple.gr[k] {
        Some(g) if g.state == Phase::Hold => {
            f.balls[1 - k].grabbed = false;
            f.balls[k].pinned = false;
            f.ev("release", k as i64, vec![Value::Str(why.to_string())]);
        }
        Some(_) => f.balls[k].vmul = None,
        None => {}
    }
    f.grapple.gr[k] = None;
}

/// `np.clip(p, lo, hi)` on a 2-vector.
fn clip(p: V2, lo: f64, hi: f64) -> V2 {
    [geom::np_clip(p[0], lo, hi), geom::np_clip(p[1], lo, hi)]
}

/// A float field of an event (a rope event's `x` and `y`).
///
/// # Panics
/// When the field is not a float.
fn float(v: &Value) -> f64 {
    match v {
        Value::Float(x) => *x,
        other => panic!("a rope event's position is a float, not {other:?}"),
    }
}

/// `grapple_step(k, dt)` (4113 to 4201), from `run()` 6175 to 6176: the rope slam of a thrown enemy, the rush's
/// steering and its grab, the hold's swing and the throw.
///
/// Events: `ropeslam` (x, y), `block` (the shielded enemy, the grappler, x, y, `grab`), `grab`, `hold` (a0, a1, when
/// it throws, the sweep), `throw` (the angle, the power, x, y).
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    if let Some(toss) = f.grapple.toss[k].clone() {
        // the thrown enemy: a rope hit while it flies is the slam
        let rope = f.events[toss.ev_i..]
            .iter()
            .find(|e| e.kind == "rope" && e.fields[0] == Value::Int(1 - k as i64))
            .map(|e| (e.fields[3].clone(), e.fields[4].clone()));
        if let Some((x, y)) = rope {
            f.damage(k, 1 - k, w.dmg_rope, "ropeslam", [float(&x), float(&y)], 0);
            f.ev("ropeslam", k as i64, vec![x, y]);
            f.grapple.toss[k] = None;
        } else {
            let ev_i = f.events.len();
            f.grapple.toss[k] = if f.t > toss.t_end {
                None
            } else {
                Some(Toss {
                    t_end: toss.t_end,
                    ev_i,
                })
            };
        }
    }
    let Some(mut g) = f.grapple.gr[k].clone() else {
        return;
    };
    let (bp, op) = (f.balls[k].p, f.balls[1 - k].p);
    let rel = geom::sub(op, bp);
    let dist = geom::hypot(rel);
    if g.state == Phase::Rush {
        let ov = f.balls[1 - k].v;
        let tl = w.lead * py::py_max(0.0, g.t1 - f.t);
        // dd = O.p + O.v * tl - B.p
        let dd = geom::sub(geom::add(op, geom::scale(ov, tl)), bp);
        let want = dmath::atan2(dd[1], dd[0]);
        let bv = f.balls[k].v;
        let cur = dmath::atan2(bv[1], bv[0]);
        let st = w.track * dt;
        let a = cur + py::py_max(-st, py::py_min(st, geom::wrap(want - cur)));
        f.balls[k].v = geom::scale([dmath::cos(a), dmath::sin(a)], geom::hypot(bv));
        g.a = a;
        f.grapple.gr[k] = Some(g.clone());
        let o = &f.balls[1 - k];
        // 4142 to 4143 (`rw_left`: the kind `rewind`, older than the kit's; never set)
        let free = o.shadow <= 0.0 && o.smashed <= 0.0 && !o.air && o.swan.is_none();
        if free && dist - 2.0 * r <= f.balls[k].blade_len {
            let n = if dist > 1e-6 {
                geom::div(rel, dist)
            } else {
                [dmath::cos(a), dmath::sin(a)]
            };
            let h = geom::add(bp, geom::scale(n, r));
            if f.balls[1 - k].walking {
                // a walking bass slips the grab
                end_rush(f, k, 2, h[0], h[1]);
                return;
            }
            if f.balls[1 - k].shield > 0.0 {
                let data = vec![
                    Value::Int(k as i64),
                    Value::Float(h[0]),
                    Value::Float(h[1]),
                    Value::Str("grab".to_string()),
                ];
                f.ev("block", 1 - k as i64, data);
                end_rush(f, k, 3, h[0], h[1]);
                return;
            }
            if !f.damage(k, 1 - k, w.dmg, "grab", h, 0) {
                end_rush(f, k, 0, h[0], h[1]);
                return;
            }
            let a0 = dmath::atan2(n[1], n[0]);
            let m_ = f.rules.a - 3.0 * r - 0.05;
            let a1 =
                nearest_rope_angle(f.rules.a, clip(bp, -m_, m_)) + geom::radians(f.rng.uniform(-1.0, 1.0) * w.jitter);
            let sd = f.balls[k].spin_dir;
            let mut delta = py::fmod_py((a1 - a0) * sd, 2.0 * dmath::PI);
            if delta < w.turns * 2.0 * dmath::PI {
                delta += 2.0 * dmath::PI;
            }
            f.balls[k].vmul = None;
            f.balls[k].pinned = true;
            f.balls[1 - k].grabbed = true;
            let t1 = f.t + w.hold_beats * f.rules.beat;
            f.grapple.gr[k] = Some(Gr {
                state: Phase::Hold,
                t0: f.t,
                t1,
                a0,
                a1,
                sweep: delta * sd,
                a: a0,
                c0: bp,
                c1: clip(bp, -m_, m_),
            });
            let reach = f.balls[k].blade_len;
            let data = vec![
                Value::Int(1),
                Value::Float(h[0]),
                Value::Float(h[1]),
                Value::Float(reach),
            ];
            f.ev("grab", k as i64, data);
            let data = vec![
                Value::Float(a0),
                Value::Float(a1),
                Value::Float(t1),
                Value::Float(delta * sd),
            ];
            f.ev("hold", k as i64, data);
        } else if f.t >= g.t1 {
            end_rush(f, k, 0, bp[0], bp[1]);
        }
        return;
    }
    // the hold: swing the enemy round at arm's length, faster and faster, then throw
    let lim = f.rules.a - r;
    let v_shared = f.rules.v;
    if f.t + 1e-9 >= g.t1 {
        let a = g.a1;
        let d = [dmath::cos(a), dmath::sin(a)];
        f.balls[k].p = g.c1;
        f.balls[1 - k].grabbed = false;
        f.balls[k].pinned = false;
        f.grapple.gr[k] = None;
        let power = f.grapple.power[k];
        let o = &mut f.balls[1 - k];
        o.p = clip(geom::add(g.c1, geom::scale(d, 2.0 * r + 0.03)), -lim, lim);
        o.v = geom::scale(d, v_shared);
        o.boost = py::py_max(o.boost, power);
        let h = o.p;
        if f.damage(k, 1 - k, w.dmg_throw, "throw", h, 0) {
            let b = &mut f.balls[k];
            b.blade_len = py::py_min(w.max_reach, b.blade_len + w.grow);
            f.grapple.power[k] = py::py_min(w.power_max, power + w.power_grow);
        }
        let data = vec![
            Value::Float(a),
            Value::Float(f.grapple.power[k]),
            Value::Float(h[0]),
            Value::Float(h[1]),
        ];
        f.ev("throw", k as i64, data);
        f.grapple.toss[k] = Some(Toss {
            t_end: f.t + w.rope_t,
            ev_i: f.events.len(),
        });
        return;
    }
    let u = (f.t - g.t0) / py::py_max(1e-9, g.t1 - g.t0);
    let v_ = py::py_min(1.0, u / 0.35);
    // B.p = G['c0'] + (G['c1'] - G['c0']) * (v_ * v_ * (3 - 2 * v_))
    let s = v_ * v_ * (3.0 - 2.0 * v_);
    let p = geom::add(g.c0, geom::scale(geom::sub(g.c1, g.c0), s));
    f.balls[k].p = p;
    // the swing accelerates into the throw: a = G['a0'] + G['sweep'] * u ** 1.6
    let a = g.a0 + g.sweep * dmath::pow(u, 1.6);
    g.a = a;
    f.grapple.gr[k] = Some(g);
    let out = geom::add(p, geom::scale([dmath::cos(a), dmath::sin(a)], 2.0 * r + 0.02));
    f.balls[1 - k].p = clip(out, -lim, lim);
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5868 to 5869: `GRAB`, when a fighter
/// is a grappler.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) { &["GRAB"] } else { &[] }
}

/// This kind's rows of one frame: `snap`, `GRAB` (5625 to 5633): per ball the state (0 idle, 1 rush, 2 hold), the
/// arm's angle, the state's start and end, then the reach and the power (a grappler's only; zeros else).
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut row = [0.0; 12];
    for j in 0..2 {
        if let Some(g) = &f.grapple.gr[j] {
            row[6 * j..6 * j + 4].copy_from_slice(&[g.state.code(), g.a, g.t0, g.t1]);
        }
        if is(f, j) {
            row[6 * j + 4] = f.balls[j].blade_len;
            row[6 * j + 5] = f.grapple.power[j];
        }
    }
    out.push_f64("GRAB", &[2, 6], &row);
}

#[cfg(test)]
mod tests {
    use super::nearest_rope_angle;
    use bf_core::dmath::PI;

    #[test]
    fn the_nearest_rope_is_pythons_min_of_tuples() {
        assert_eq!(nearest_rope_angle(4.0, [3.0, 0.0]), 0.0);
        assert_eq!(nearest_rope_angle(4.0, [-3.0, 0.5]), PI);
        assert_eq!(nearest_rope_angle(4.0, [0.0, 3.5]), PI / 2.0);
        assert_eq!(nearest_rope_angle(4.0, [0.2, -3.5]), -PI / 2.0);
        // a tie: the smaller angle (-pi/2 before 0, 0 before pi/2)
        assert_eq!(nearest_rope_angle(4.0, [3.0, -3.0]), -PI / 2.0);
        assert_eq!(nearest_rope_angle(4.0, [3.0, 3.0]), 0.0);
        assert_eq!(nearest_rope_angle(4.0, [0.0, 0.0]), -PI / 2.0);
    }
}
