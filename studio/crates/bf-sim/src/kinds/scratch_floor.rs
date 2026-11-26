//! Weapon kind `scratch_floor`, TURNTABLES's THE RING IS THE RECORD: `fight.py` 1713 to 1762 (`scratch_beat`,
//! `scratch_step`).
//!
//! Every `every` beats (a haste divides it) the DJ scratches the record the ring sits on: over `dur_beats` beats the
//! floor jerks forward, then back half way through (directions alternate scratch to scratch). The DJ's own ball is
//! anchored by its needle; each jerk flings the opponent round the ring centre (the tangent in the jerk's direction +
//! `out` x outward) with a knock of the current strength (`blade_len`, recorded as `BL`). The first rope the opponent
//! hits within `window` s of a jerk is a rope slam: a `scratch` hit of `dmg` x the impact speed / the shared speed
//! (capped at `cap`); a landed slam adds `grow` strength up to `max`. A rewinding opponent is intangible: its jerks
//! pass it by. Events `scratch` (length, strength, direction) and `jerk` (direction, the opponent's x, y, strength).
//! None of this code draws from the fight's generator.

use bf_core::event::Value;
use bf_core::py;

use crate::fight::{Fight, rewinding};
use crate::geom;
use crate::roster::{ScratchFloorCfg, WeaponCfg};

/// `B.sf`: `dict(n=0, pending=[], live=-1.0, hit=True, seen=len(self.events))`, made at the first scratch beat.
#[derive(Clone, Debug, PartialEq)]
pub struct Sf {
    /// Scratches so far (the next one's direction: forward on an even count).
    pub n: i64,
    /// The jerks still to come: (time, direction).
    pub pending: Vec<(f64, f64)>,
    /// A rope hit up to this time is a slam.
    pub live: f64,
    /// Whether the last jerk's slam is spent (landed, or none).
    pub hit: bool,
    /// The events already looked at for a rope hit.
    pub seen: usize,
}

/// This kind's per-fight state (`Fight::scratch_floor`): each ball's `B.sf` (`None` before its first scratch beat).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// By ball.
    pub balls: [Option<Sf>; 2],
}

/// `B.w['kind'] == 'scratch_floor'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::ScratchFloor(_))
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a ScratchFloorCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::ScratchFloor(w) => w,
        w => panic!("fighter {k}'s weapon is not a scratch_floor: {w:?}"),
    }
}

/// `scratch_beat(k, kb)` (1713 to 1730), from `run()` 5942 to 5943.
pub fn beat(f: &mut Fight, k: usize, kb: i64) {
    let w = weapon(f, k);
    if f.scratch_floor.balls[k].is_none() {
        f.scratch_floor.balls[k] = Some(Sf {
            n: 0,
            pending: Vec::new(),
            live: -1.0,
            hit: true,
            seen: f.events.len(),
        });
        f.balls[k].blade_len = w.base;
    }
    // ev = max(1, int(round(w['every'] / self.ab_mult(k))))
    let ev = (py::round_half_even(w.every / f.ab_mult(k)) as i64).max(1);
    if kb.rem_euclid(ev) != 0 {
        return;
    }
    let d = w.dur_beats * f.rules.beat;
    let t = f.t;
    let sf = f.scratch_floor.balls[k].as_mut().expect("made above");
    let sgn = if sf.n.rem_euclid(2) == 0 { 1.0 } else { -1.0 };
    sf.n += 1;
    sf.pending = vec![(t, sgn), (t + d / 2.0, -sgn)];
    let strength = f.balls[k].blade_len;
    f.ev(
        "scratch",
        k as i64,
        vec![Value::Float(d), Value::Float(strength), Value::Float(sgn)],
    );
}

/// `scratch_step(k, dt)` (1731 to 1762), from `run()` 6127 to 6128.
pub fn step(f: &mut Fight, k: usize, _dt: f64) {
    let w = weapon(f, k);
    let o = 1 - k;
    if f.scratch_floor.balls[k].is_none() {
        return;
    }
    loop {
        let t = f.t;
        let next = {
            let sf = f.scratch_floor.balls[k].as_mut().expect("tested above");
            match sf.pending.first() {
                Some(&(t0, sgn)) if t + 1e-9 >= t0 => {
                    sf.pending.remove(0);
                    Some(sgn)
                }
                _ => None,
            }
        };
        let Some(sgn) = next else { break };
        if rewinding(f, o) {
            // a rewinding opponent is intangible
            continue;
        }
        let op = f.balls[o].p;
        let rn = geom::hypot(op);
        let outw = if rn > 1e-6 {
            geom::div(op, rn)
        } else {
            let ov = f.balls[o].v;
            geom::div(ov, geom::norm(ov) + 1e-12)
        };
        let tang = geom::scale([-outw[1], outw[0]], sgn);
        let strength = f.balls[k].blade_len;
        f.knock(o, geom::add(tang, geom::scale(outw, w.out)), strength);
        let live = f.t + w.window;
        let sf = f.scratch_floor.balls[k].as_mut().expect("tested above");
        sf.live = live;
        sf.hit = false;
        let op = f.balls[o].p;
        let data = vec![
            Value::Float(sgn),
            Value::Float(op[0]),
            Value::Float(op[1]),
            Value::Float(strength),
        ];
        f.ev("jerk", k as i64, data);
    }
    let sf = f.scratch_floor.balls[k].as_ref().expect("tested above");
    if !sf.hit && f.t <= sf.live {
        let seen = sf.seen;
        let rope = f.events[seen..]
            .iter()
            .find(|e| e.kind == "rope" && e.who() == Some(o))
            .map(|e| (e.f(5), e.f(6), e.f(7)));
        if let Some((x, y, speed)) = rope {
            f.scratch_floor.balls[k].as_mut().expect("tested above").hit = true;
            let dmg = w.dmg * py::py_min(w.cap, speed / f.rules.v);
            if f.damage(k, o, dmg, "scratch", [x, y], 0) {
                f.balls[k].blade_len = py::py_min(w.max, f.balls[k].blade_len + w.grow);
            }
        }
    }
    let n = f.events.len();
    f.scratch_floor.balls[k].as_mut().expect("tested above").seen = n;
}
