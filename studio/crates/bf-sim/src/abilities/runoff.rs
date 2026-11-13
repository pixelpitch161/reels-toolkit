//! Ability kind `runoff` (PIANO's 88 KEYS): `fight.py` `trigger` 2944 to 2945; `start_runoff` 5245 to 5261.
//!
//! The roll runs off its spool: every lane fires at once, then no row fires while a new roll is threaded. The work is
//! the roll's (`kinds::roll::start_runoff`). Event `runoff`: the fighter, the volley, how many shots, their pitches.

use crate::abilities::param;
use crate::fight::Fight;
use crate::kinds::roll::{self, Runoff};
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::runoff`): none of its own. `B.reload_until` is with the roll's state
/// (`Fight::roll`), which reads it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `runoff`: `trigger` 2944 to 2945.
///
/// # Panics
/// For a fighter whose weapon is not a roll, as Python raises (`start_runoff` reads the ball's `roll_ax`).
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let numbers = Runoff {
        n: param(ab, "n"),
        speed: param(ab, "speed"),
        dmg: param(ab, "dmg"),
        margin: ab.get("margin").unwrap_or(0.25),
        reload_beats: ab.get("reload_beats").unwrap_or(0.0),
    };
    roll::start_runoff(f, k, numbers);
}
