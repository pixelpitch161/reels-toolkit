//! Ability kind `clamp` (ACCORDION's FULL BELLOWS): `fight.py` `trigger` 2876 to 2877; 1905 to 1926 (`clamp_start`,
//! `clamps_step`).
//!
//! A stub until ACCORDION's port (milestone 4). [`start`] is already called from `trigger`'s dispatch
//! (`abilities/mod.rs`), and every other function here where `Fight.run()` calls its Python namesake, behind a test
//! that is false until the roster reads the kind (`roster::abilities::clamp::PORTED`). So a stub body is never reached
//! in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the
//! Python (every draw from the fight's generator in Python's order and number) and touches no other engine file. A
//! copied ability (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the
//! kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// One entry of `self.clamps`: its fields are filled in by ACCORDION's port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clamp {}

/// This kind's per-fight state (`Fight::clamp`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.clamps`: the clamps called and not yet snapped (`clamp_start`'s dicts).
    pub clamps: Vec<Clamp>,
}

/// Whether `self.clamps` holds anything (Python's truth test of the list).
pub fn live(f: &Fight) -> bool {
    !f.clamp.clamps.is_empty()
}

/// `trigger`, kind `clamp` (2876 to 2877): `clamp_start(k, ab)` (1905 to 1914).
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::clamp::start");
}

/// `clamps_step()` (1915 to 1926), from `run()` 6201 to 6202 (`if self.clamps`).
pub fn clamps_step(f: &mut Fight) {
    f.socket("stub:abilities::clamp::clamps_step");
}
