//! Weapon kind `pulse`, DRUMS's the drumhead: `fight.py` 411 to 424 (`fire_pulse`) and the pulse counter of `run()`
//! (5946 to 5963); its rings are `Fight::waves` of kind `pulse` (with `grow` and `clears`), which `effects_step`
//! already runs.
//!
//! On every 16th of its own grid (`4 * every`, a float as in Python; a haste divides it) a ring blasts out from the
//! ball to its reach (`blade_len` beyond the surface). A pulse on the main grid hits for `dmg`, knocks `knock` and
//! grows the reach when it lands (by `grow`, up to `max_reach`); an extra pulse from haste hits for `sub_dmg` x and
//! knocks half. None of this code draws from the generator.

use bf_core::event::Value;
use bf_core::py;

use crate::fight::{Fight, Wave};
use crate::roster::{PulseCfg, WeaponCfg};

/// This kind's per-fight state (`Fight::pulse`): the kind keeps none of its own (its rings are `Fight::waves`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'pulse'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Pulse(_))
}

/// The pulse's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a pulse.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a PulseCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Pulse(w) => w,
        other => panic!("fighter {k} ({}) has no pulse weapon: {other:?}", cfg.name),
    }
}

/// The pulse block of `run()` for one fighter, after the caller's stun test (5951 to 5957): `main = 4 * every` (a
/// float), `every16 = max(1, int(round(main / ab_mult(k))))`, and on `k16 % every16 == 0` `fire_pulse(k, k16 % main ==
/// 0)` (411 to 424). From `run()` 5946 to 5963.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let main = 4.0 * weapon(f, k).every;
    let every16 = (py::round_half_even(main / f.ab_mult(k)) as i64).max(1);
    if k16 % every16 == 0 {
        fire_pulse(f, k, (k16 as f64).rem_euclid(main) == 0.0);
    }
}

/// `fire_pulse(k, main)` (411 to 424): a ring from the ball's centre, as wide as the ball, out to the reach. Event
/// `pulse`: whether it is a main pulse and the reach.
pub fn fire_pulse(f: &mut Fight, k: usize, main: bool) {
    let w = weapon(f, k);
    let b = &f.balls[k];
    let r = f.rules.r;
    let wave = Wave {
        owner: k,
        c: b.p,
        r,
        speed: w.speed,
        rmax: r + b.blade_len,
        dmg: w.dmg * if main { 1.0 } else { w.sub_dmg },
        knock: w.knock * if main { 1.0 } else { 0.5 },
        kind: "pulse".to_string(),
        hit: false,
        name: "pulse".to_string(),
        t0: f.t,
        slow: None,
        stun: None,
        detune: None,
        grow: main,
        clears: w.clears,
    };
    let reach = b.blade_len;
    f.waves.push(wave);
    f.ev(
        "pulse",
        k as i64,
        vec![Value::Int(i64::from(main)), Value::Float(reach)],
    );
}

/// `effects_step` 3078 to 3082 for a landed main pulse: the reach grows by `grow`, up to `max_reach` (DRUMS has no
/// `pull_grow`, which the roster refuses).
pub fn ring_grow(f: &mut Fight, owner: usize) {
    let w = weapon(f, owner);
    let b = &mut f.balls[owner];
    b.blade_len = py::py_min(w.max_reach, b.blade_len + w.grow);
}
