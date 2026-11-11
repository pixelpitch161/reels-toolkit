//! Ability kind `shockwave` (SAD TROMBONE): `fight.py` 2841 to 2844 (`trigger`), and the slow a ring puts on the
//! enemy it hits, 3075 to 3077 (`effects_step`).
//!
//! A shockwave is the ring of [`crate::abilities::wave`] under its own name. The TROMBONE's carries a `slow`: the
//! enemy it hits moves at `mult` times its speed for `dur` seconds (`Fight::move_step` reads `slow` and
//! `slow_mult`; the fight loop runs `slow` down).

use bf_core::event::Value;

use crate::abilities::wave;
use crate::fight::Fight;
use crate::roster::{AbilityCfg, SlowCfg};

/// `trigger`, kind `shockwave`: a ring of kind `shockwave` from where fighter `k` is.
pub fn fire(fight: &mut Fight, k: usize, ab: &AbilityCfg) {
    wave::ring(fight, k, ab, "shockwave");
}

/// `effects_step`, `if landed and W.get('slow')`: fighter `o`, hit by `owner`'s ring, is slowed: `O.slow = dur`,
/// `O.slow_mult = mult`. Event `slow`: the ring's owner and the seconds it lasts. A slow on a fighter that is
/// already slowed starts the time again.
pub fn slow(fight: &mut Fight, o: usize, owner: usize, slow: SlowCfg) {
    let b = &mut fight.balls[o];
    b.slow = slow.dur;
    b.slow_mult = slow.mult;
    fight.ev("slow", o as i64, vec![Value::Int(owner as i64), Value::Float(slow.dur)]);
}
