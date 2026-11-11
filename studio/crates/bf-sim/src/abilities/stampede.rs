//! Ability kind `stampede` (CONTRABASS's THE ELEPHANT): `fight.py` `trigger` 2953 to 2954; 4302 to 4368
//! (`start_stampede`, `stampede_step`).
//!
//! A stub until CONTRABASS's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and
//! every other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the
//! roster reads the kind (`roster::abilities::stampede::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::stampede`): what `fight.py` hangs on the ball or the fight for it, indexed
/// by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `stampede`: `trigger` 2953 to 2954.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::stampede::start");
}

/// `stampede_step(k, dt)` (4323 to 4368), from `run()` 6181 to 6182
pub fn stampede_step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:abilities::stampede::stampede_step");
}
