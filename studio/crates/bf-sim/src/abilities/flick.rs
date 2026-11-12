//! Ability kind `flick` (HORN's FINAL FLICK): `fight.py` `trigger` 2880 to 2881; 2282 to 2337 (`FLICK_STATES`,
//! `start_flick`, `flick_step`).
//!
//! A giant glove bursts out of the ball and winds up for `wind_beats`, tracking the enemy; on the beat it flicks out
//! up to `reach` beyond the surface (a glove of radius `glove_r`). If it touches the enemy: `dmg`, and the enemy is
//! flung straight away along the flick at the shared speed (speed boost `boost`, decaying like a knockback); a rope
//! hit within `rope_t` s adds `dmg_rope` (a hit of kind `flick_rope`). Out of reach, it misses. Events `flick_wind`
//! (the time it flicks, the aim) and `flick` (hit 0 / 1, the aim, how far it reached, landed 0 / 1); recorded as
//! `FLICK`.
//!
//! Python hangs `flick` and `flick_rope` on the ball; here they are [`State`], `Fight::flick`, indexed by fighter,
//! with the ability's numbers copied in. A copied ability (IDENTITY CRISIS) runs this code for the copier's ball, so
//! nothing here assumes which side has the kind (as in Python, only a fighter with a flick in its own roster entry is
//! stepped: `run()`'s `flickers`). None of this code draws from the generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::Fight;
use crate::geom;
use crate::record::Frames;
use crate::roster::{AbilityCfg, AbilityKind};

/// The glove's state (`FLICK_STATES`: `wind` 1, `out` 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Winding up, tracking the enemy.
    Wind,
    /// Flicked out (hit or not), shown for `show` s.
    Out,
}

impl Phase {
    /// `FLICK_STATES[state]`.
    pub fn code(self) -> f64 {
        match self {
            Phase::Wind => 1.0,
            Phase::Out => 2.0,
        }
    }
}

/// A glove out (`B.flick`, the dict `start_flick` makes; `ab`'s numbers are copied in).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glove {
    /// Its state.
    pub state: Phase,
    /// When it burst out.
    pub t0: f64,
    /// When it flicks.
    pub t_hit: f64,
    /// The aim, an angle.
    pub aim: f64,
    /// How far beyond contact it reached (0 while it winds up).
    pub ext: f64,
    /// 1 when it touched the enemy.
    pub hit: i64,
    /// When it flicked (`F['t_out']`, set as it goes out).
    pub t_out: f64,
    /// `ab['reach']`.
    pub reach: f64,
    /// `ab.get('glove_r', 0.4)`.
    pub glove_r: f64,
    /// `ab['dmg']`.
    pub dmg: f64,
    /// `ab['boost']`.
    pub boost: f64,
    /// `ab.get('rope_t', 0.9)`.
    pub rope_t: f64,
    /// `ab['dmg_rope']`.
    pub dmg_rope: f64,
    /// `ab.get('show', 0.6)`.
    pub show: f64,
}

/// A flung enemy's pending rope hit (`B.flick_rope`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rope {
    /// The rope hit counts until this time.
    pub t_end: f64,
    /// The first event not looked at yet.
    pub ev_i: usize,
    /// Its damage.
    pub dmg: f64,
}

/// This ability's per-fight state (`Fight::flick`), indexed by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.flick`.
    pub glove: [Option<Glove>; 2],
    /// `B.flick_rope`.
    pub rope: [Option<Rope>; 2],
}

/// `trigger`, kind `flick`: `trigger` 2880 to 2881, `start_flick(k, ab)` (2290 to 2295): the glove bursts out, aimed
/// at the enemy. Event `flick_wind`.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let aim = dmath::atan2(d[1], d[0]);
    let t_hit = f.t + ab.get("wind_beats").unwrap_or(1.0) * f.rules.beat;
    f.flick.glove[k] = Some(Glove {
        state: Phase::Wind,
        t0: f.t,
        t_hit,
        aim,
        ext: 0.0,
        hit: 0,
        t_out: 0.0,
        reach: param(ab, "reach"),
        glove_r: ab.get("glove_r").unwrap_or(0.4),
        dmg: param(ab, "dmg"),
        boost: param(ab, "boost"),
        rope_t: ab.get("rope_t").unwrap_or(0.9),
        dmg_rope: param(ab, "dmg_rope"),
        show: ab.get("show").unwrap_or(0.6),
    });
    f.ev("flick_wind", k as i64, vec![Value::Float(t_hit), Value::Float(aim)]);
}

/// `flick_step(k, dt)` (2297 to 2337), from `run()` 6148 to 6149: a flung enemy's rope hit; the glove tracks the
/// enemy through the wind-up, flicks at its time (a hit, or a miss at full reach), and is put away `show` s later.
pub fn flick_step(f: &mut Fight, k: usize, _dt: f64) {
    let r = f.rules.r;
    if let Some(mut fr) = f.flick.rope[k] {
        // flung: a rope hit while it flies hits again
        let rope = f.events[fr.ev_i..]
            .iter()
            .find(|e| e.kind == "rope" && e.who() == Some(1 - k))
            .map(|e| [e.f(5), e.f(6)]);
        match rope {
            Some(at) => {
                f.damage(k, 1 - k, fr.dmg, "flick_rope", at, 0);
                f.flick.rope[k] = None;
            }
            None => {
                fr.ev_i = f.events.len();
                f.flick.rope[k] = if f.t > fr.t_end { None } else { Some(fr) };
            }
        }
    }
    let Some(mut g) = f.flick.glove[k] else {
        return;
    };
    match g.state {
        Phase::Wind => {
            let (b_p, o_p) = (f.balls[k].p, f.balls[1 - k].p);
            let d = geom::sub(o_p, b_p);
            // the glove tracks the enemy through the wind-up
            g.aim = dmath::atan2(d[1], d[0]);
            if f.t + 1e-9 < g.t_hit {
                f.flick.glove[k] = Some(g);
                return;
            }
            let u = [dmath::cos(g.aim), dmath::sin(g.aim)];
            let dist = geom::hypot(d);
            g.state = Phase::Out;
            g.t_out = f.t;
            if dist - 2.0 * r <= g.reach + g.glove_r {
                g.ext = py::py_max(0.0, dist - 2.0 * r);
                g.hit = 1;
                f.flick.glove[k] = Some(g);
                // hx, hy = (O.p - u * R).tolist()
                let at = geom::sub(o_p, geom::scale(u, r));
                let landed = f.damage(k, 1 - k, g.dmg, "flick", at, 0);
                let v_shared = f.rules.v;
                let o = &mut f.balls[1 - k];
                // flung straight across the ring
                o.v = geom::scale(u, v_shared);
                o.boost = py::py_max(o.boost, g.boost);
                let data = vec![
                    Value::Int(1),
                    Value::Float(g.aim),
                    Value::Float(g.ext),
                    Value::Int(i64::from(landed)),
                ];
                f.ev("flick", k as i64, data);
                f.flick.rope[k] = Some(Rope {
                    t_end: f.t + g.rope_t,
                    ev_i: f.events.len(),
                    dmg: g.dmg_rope,
                });
            } else {
                g.ext = g.reach;
                f.flick.glove[k] = Some(g);
                let data = vec![Value::Int(0), Value::Float(g.aim), Value::Float(g.reach), Value::Int(0)];
                f.ev("flick", k as i64, data);
            }
        }
        Phase::Out => {
            if f.t > g.t_out + g.show {
                f.flick.glove[k] = None;
            }
        }
    }
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5827 to 5829: `FLICK`, when a fighter
/// has a flick among its abilities.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| crate::abilities::has(f, k, &AbilityKind::Flick)) {
        &["FLICK"]
    } else {
        &[]
    }
}

/// This kind's rows of one frame: `snap`, `FLICK` (5705 to 5711): per ball, the glove's state, aim, reach, start,
/// flick time and hit, or six NaN when it has none out.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut row = [f64::NAN; 12];
    for (j, glove) in f.flick.glove.iter().enumerate() {
        if let Some(g) = glove {
            row[6 * j..6 * j + 6].copy_from_slice(&[g.state.code(), g.aim, g.ext, g.t0, g.t_hit, g.hit as f64]);
        }
    }
    out.push_f64("FLICK", &[2, 6], &row);
}
