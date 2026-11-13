//! Ability kind `lash` (SYNTH's SAWTOOTH): `fight.py` `trigger` 2868 to 2869; 1661 to 1712 (`start_lash`, `lash_angle`,
//! `lash_step`).
//!
//! A stub until SYNTH's port (milestone 4). [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`),
//! and every other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the
//! roster reads the kind (`roster::abilities::lash::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// One entry of `self.lashes`: its fields are filled in by SYNTH's port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lash {}

/// This kind's per-fight state (`Fight::lash`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.lashes`: the lashes in progress (`start_lash`'s dicts).
    pub lashes: Vec<Lash>,
}

/// Whether `self.lashes` holds anything (Python's truth test of the list).
pub fn live(f: &Fight) -> bool {
    !f.lash.lashes.is_empty()
}

/// `trigger`, kind `lash` (2868 to 2869): `start_lash(k, ab)` (1661 to 1681).
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::lash::start");
}

/// `lash_step(dt)` (1687 to 1712), from `run()` 6137 to 6138 (`if getattr(self, 'lashes', None)`).
pub fn lash_step(f: &mut Fight, _dt: f64) {
    f.socket("stub:abilities::lash::lash_step");
}
