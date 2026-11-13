//! Ability kind `joke` (VIOLA's VIOLA JOKE): `fight.py` `trigger` 2921 to 2922; 5118 to 5136 (`start_joke`: the stun is
//! the ball's `stun`, and the note it drops is the `rests` kind's `fire_note`).
//!
//! A stub until VIOLA's port (milestone 4). [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`),
//! and every other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the
//! roster reads the kind (`roster::abilities::joke::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This kind's per-fight state (`Fight::joke`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `joke` (2921 to 2922): `start_joke(k, ab)` (5118 to 5136).
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::joke::start");
}
