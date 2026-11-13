//! Ability kind `snap` (BARI SAX's CABLE SNAP): `fight.py` `trigger` 2919 to 2920; 3787 to 3856 (`start_snap`,
//! `snap_step`).
//!
//! A stub until BARI SAX's port (milestone 4). [`start`] is already called from `trigger`'s dispatch
//! (`abilities/mod.rs`), and every other function here where `Fight.run()` calls its Python namesake, behind a test
//! that is false until the roster reads the kind (`roster::abilities::snap::PORTED`). So a stub body is never reached
//! in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the
//! Python (every draw from the fight's generator in Python's order and number) and touches no other engine file. A
//! copied ability (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the
//! kind.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::AbilityCfg;

/// This kind's per-fight state (`Fight::snap`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `snap` (2919 to 2920): `start_snap(k, ab)` (3787 to 3806).
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::snap::start");
}

/// `snap_step(k, dt)` (3807 to 3856), from `run()` 6169 to 6170.
pub fn snap_step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:abilities::snap::snap_step");
}

/// The record keys this ability adds (Python's `rec` gets them at setup): `run()` 5849 to 5850: `SNAP`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This ability's rows of one frame: `snap`, `SNAP` (5588 to 5594), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:abilities::snap::snap");
}
