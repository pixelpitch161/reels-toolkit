//! Ability kind `stun` (CLARINET's SQUEAK): `fight.py` `trigger` 2889 to 2892; the stun hit of `effects_step` 3072
//! to 3074.
//!
//! A shrill burst ring from the fighter, like a wave's (`speed`, `radius`, `dmg`, `knock`; a hit by it is of kind
//! `stun`, and the record's `WAVES` rows carry that kind). The fighter it lands on is stunned for `stun` seconds: it
//! holds still, heading kept (`move` 3135), and starts no attack (`_stunned`, [`crate::fight::stunned`], asked by
//! every grid). The timer is the ball's (`Ball::stun`); the fight loop runs it down (6100 to 6101).
//!
//! Event: `stun`, the stunned fighter, the ring's owner, the seconds.
//!
//! A copied ability (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here assumes which side has
//! the kind.

use bf_core::event::Value;

use crate::abilities::param;
use crate::fight::{Fight, Wave};
use crate::geom;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::stun`): nothing of its own (the ring is in `Fight::waves`, the timer on
/// the ball).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `stun` (2889 to 2892): a ring of kind `stun` from where fighter `k` is, as wide as the ball, with
/// the ability's `speed`, `radius`, `dmg`, `knock` and `stun`, and no slow.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let wave = Wave {
        owner: k,
        c: f.balls[k].p,
        r: f.rules.r,
        speed: param(ab, "speed"),
        rmax: param(ab, "radius"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        kind: "stun".to_string(),
        hit: false,
        name: ab.name.clone(),
        t0: f.t,
        slow: None,
        stun: Some(param(ab, "stun")),
        detune: None,
        grow: false,
        clears: true,
    };
    f.waves.push(wave);
}

/// `effects_step` 3072 to 3074, a ring with `stun` that landed: `O.stun = max(getattr(O, 'stun', 0.0), W['stun'])`,
/// event `stun`.
pub fn hit(f: &mut Fight, o: usize, owner: usize, stun: f64) {
    let b = &mut f.balls[o];
    b.stun = geom::py_max(b.stun, stun);
    f.ev("stun", o as i64, vec![Value::Int(owner as i64), Value::Float(stun)]);
}
