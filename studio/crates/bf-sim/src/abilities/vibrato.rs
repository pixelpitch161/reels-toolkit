//! Ability kind `vibrato` (CELLO's VIBRATO): `fight.py` `trigger` 2955 to 2957; its shakes are dealt by the grapple's
//! `grapple_tick` (4074 to 4076, `kinds::grapple::tick`).
//!
//! For `ab_dur(ab)` an enemy its fighter holds in THE EMBRACE shakes and takes `dmg` on every 16th of the hold. A
//! fighter that is no grappler sets the time and nothing reads it (Python's `B.vib_until` on any ball). A copied
//! ability (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here assumes which side has the kind.

use bf_core::event::Value;

use crate::abilities::{ab_dur, param};
use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::vibrato`): `B.vib_until` and `B.vib_ab['dmg']`, indexed by fighter;
/// `None` before it first fires (Python's `getattr(B, 'vib_until', -1.0)`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// When the shaking ends, and the damage of each shake.
    pub on: [Option<(f64, f64)>; 2],
}

/// `trigger`, kind `vibrato`: `trigger` 2955 to 2957. Event `vibrato`: when it ends.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + ab_dur(f, ab);
    f.vibrato.on[k] = Some((until, param(ab, "dmg")));
    f.ev("vibrato", k as i64, vec![Value::Float(until)]);
}
