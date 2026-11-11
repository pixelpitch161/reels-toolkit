//! Ability kind `prelude` (CELLO's PRELUDE): `fight.py` `trigger` 2948 to 2950; `prelude_step` 4220 to 4243.
//!
//! A stub until CELLO's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::prelude::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::prelude`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `prelude`: `trigger` 2948 to 2950.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::prelude::start");
}

/// `prelude_step(k)` (4226 to 4243), from `run()` 6177 to 6178
pub fn prelude_step(f: &mut Fight, _k: usize) {
    f.socket("stub:abilities::prelude::prelude_step");
}
