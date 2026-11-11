//! Ability kind `arc` (PIANO's 88 KEYS ARC): `fight.py` `trigger` 2852 to 2858; the arcs of `effects_step` 3099
//! to 3113.
//!
//! A stub until PIANO's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::arc::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::arc`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `arc`: `trigger` 2852 to 2858.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::arc::start");
}

/// Whether `self.arcs` holds anything: `effects_step` walks the arcs (3099 to 3113), and only then is [`step`] called
/// A stub: false (nothing) until the port.
pub fn live(_f: &Fight) -> bool {
    false
}

/// The arcs of `effects_step` (3099 to 3113): each arc emits its projectiles on time, after the slams; spent arcs are
/// dropped
pub fn step(f: &mut Fight) {
    f.socket("stub:abilities::arc::step");
}
