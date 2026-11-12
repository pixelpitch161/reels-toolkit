//! Ability kind `sustain` (PIANO's SUSTAIN): `fight.py` `trigger` 2941 to 2943.
//!
//! For the ability's time the roll's rows hang like mines: `kinds::roll::fire_row` reads `sustain_until` and gives
//! each shot of a sustained row its stop and release times; `kinds::roll::hold_step` stops and releases them. Event
//! `sustain`: the fighter and when the sustain ends.

use bf_core::event::Value;

use crate::abilities::ab_dur;
use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::sustain`): none of its own. `B.sustain_until` is with the roll's state
/// (`Fight::roll`), which reads it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `sustain`: `trigger` 2941 to 2943.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + ab_dur(f, ab);
    f.roll.sustain_until[k] = until;
    f.ev("sustain", k as i64, vec![Value::Float(until)]);
}
