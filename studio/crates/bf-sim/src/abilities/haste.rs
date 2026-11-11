//! Ability kind `haste` (FLUTTER TONGUE, POWER SLIDE): `fight.py` 2849 to 2851.
//!
//! For `dur` seconds (or `dur_beats` beats) the fighter's weapon grid runs `mult` times as fast: `Fight::ab_mult` is
//! `haste_mult` while `haste` is above 0, and the fight loop runs `haste` down by one step per step.

use crate::abilities::{ab_dur, param};
use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// `trigger`, kind `haste`: `B.haste = ab_dur(ab)`, `B.haste_mult = ab['mult']`. A haste that fires while one still
/// runs starts the time again.
pub fn fire(fight: &mut Fight, k: usize, ab: &AbilityCfg) {
    let dur = ab_dur(fight, ab);
    let b = &mut fight.balls[k];
    b.haste = dur;
    b.haste_mult = param(ab, "mult");
}
