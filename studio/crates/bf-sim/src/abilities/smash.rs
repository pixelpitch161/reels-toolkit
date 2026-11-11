//! Ability kind `smash` (ELECTRIC's the smash): `fight.py` `trigger` 2939 to 2940; 3923 to 3964 (`start_smash`,
//! `splinters_step`); the smashed timer and its `rebuild` event (`run()` 6104 to 6107).
//!
//! A stub until ELECTRIC's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::smash::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::smash`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `smash`: `trigger` 2939 to 2940.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::smash::start");
}

/// Whether `self.splinters` holds anything: the test before `splinters_step` (`run()` 6193)
/// A stub: false (nothing) until the port.
pub fn live(_f: &Fight) -> bool {
    false
}

/// `splinters_step(dt)` (3948 to 3964), from `run()` 6193 to 6194 (`if getattr(self, 'splinters', None)`: the stub
/// tests that itself). The caller tests [`live`]; the stub walks its own list.
pub fn splinters_step(f: &mut Fight, _dt: f64) {
    f.socket("stub:abilities::smash::splinters_step");
}
