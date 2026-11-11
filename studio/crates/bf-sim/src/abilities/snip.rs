//! Ability kind `snip` (OBOE's DOUBLE REED): `fight.py` `trigger` 2882 to 2883; 2526 to 2567 (`snip_ready`,
//! `start_snip`, `snip_step`); the gate in `abilities_step` 3037 to 3039.
//!
//! A stub until OBOE's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::snip::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::snip`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `snip`: `trigger` 2882 to 2883.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::snip::start");
}

/// `snip_ready(k, ab)` (2532 to 2535): an unlock of this kind waits in `abilities_step` (3037 to 3039, and 3046 to 3049
/// for a copied snip) until this is true
/// A stub: true (no wait) until the port.
pub fn ready(f: &Fight, _k: usize, _ab: &AbilityCfg) -> bool {
    f.socket("stub:abilities::snip::ready");
    true
}

/// `snip_step(k, dt)` (2543 to 2567), from `run()` 6152 to 6156 (the snippers, then each fighter in `extra_snip` that
/// is not one)
pub fn snip_step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:abilities::snip::snip_step");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5803 to 5805: `SNIP` when a fighter has
/// a snip. None until the port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `SNIP` (5555 to 5561), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:abilities::snip::snap");
}
