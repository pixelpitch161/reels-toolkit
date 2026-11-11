//! Ability kind `swan` (CELLO's SWAN SONG): `fight.py` `trigger` 2951 to 2952; 4244 to 4301 (`start_swan`,
//! `swan_step`).
//!
//! A stub until CELLO's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::swan::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::swan`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.swan` while SWAN SONG flies (`start_swan` 4251 to 4273): `Ball::swan`. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Swan {}

/// `trigger`, kind `swan`: `trigger` 2951 to 2952.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::swan::start");
}

/// `swan_step(k)` (4274 to 4301), from `run()` 6179 to 6180
pub fn swan_step(f: &mut Fight, _k: usize) {
    f.socket("stub:abilities::swan::swan_step");
}
