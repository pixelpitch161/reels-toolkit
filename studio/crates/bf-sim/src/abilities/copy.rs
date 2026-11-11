//! Ability kind `copy` (EUPHONIUM's IDENTITY CRISIS): `fight.py` `trigger` 2893 to 2894; 2801 to 2822 (`COPYABLE`,
//! `copy_source`, `start_copy`); the gate in `abilities_step` 3046 to 3049; `trigger`'s quiet path 2830 to 2840.
//!
//! A stub until EUPHONIUM's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::copy::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::copy`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `copy`: `trigger` 2893 to 2894.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::copy::start");
}

/// `copy_source(k)` (2804 to 2806): the enemy's last ability if its kind is one of `COPYABLE` (2802), else none
/// A stub: none until the port.
pub fn source<'a>(f: &Fight<'a>, _k: usize) -> Option<&'a AbilityCfg> {
    f.socket("stub:abilities::copy::source");
    None
}
