//! Ability kind `hook` (BASSOON's BOCAL HOOK): `fight.py` `trigger` 2915 to 2916; 3430 to 3476 (`hook_ready`,
//! `start_hook`, `hook_step`); the gate in `abilities_step` 3040 to 3042.
//!
//! A stub until BASSOON's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::hook::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::hook`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `hook`: `trigger` 2915 to 2916.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::hook::start");
}

/// `hook_ready(k, ab)` (3437 to 3440): the gate in `abilities_step` 3040 to 3042
/// A stub: true (no wait) until the port.
pub fn ready(f: &Fight, _k: usize, _ab: &AbilityCfg) -> bool {
    f.socket("stub:abilities::hook::ready");
    true
}

/// `hook_step(k, dt)` (3448 to 3476), from `run()` 6163 to 6164
pub fn hook_step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:abilities::hook::hook_step");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5851 to 5853: `HOOK`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `HOOK` (5602 to 5608), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:abilities::hook::snap");
}
