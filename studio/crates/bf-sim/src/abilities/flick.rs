//! Ability kind `flick` (HORN's FINAL FLICK): `fight.py` `trigger` 2880 to 2881; 2282 to 2337 (`FLICK_STATES`,
//! `start_flick`, `flick_step`).
//!
//! A stub until HORN's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::flick::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::flick`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `flick`: `trigger` 2880 to 2881.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::flick::start");
}

/// `flick_step(k, dt)` (2297 to 2337), from `run()` 6148 to 6149
pub fn flick_step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:abilities::flick::flick_step");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5827 to 5829: `FLICK`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `FLICK` (5705 to 5711), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:abilities::flick::snap");
}
