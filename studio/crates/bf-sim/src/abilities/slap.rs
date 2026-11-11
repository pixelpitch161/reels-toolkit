//! Ability kind `slap` (CONTRABASS's SLAP BASS): `fight.py` `trigger` 2946 to 2947; 4202 to 4219 (`slap_ready`,
//! `start_slap`); the gate in `abilities_step` 3043 to 3045.
//!
//! A stub until CONTRABASS's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and
//! every other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the
//! roster reads the kind (`roster::abilities::slap::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::slap`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `slap`: `trigger` 2946 to 2947.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::slap::start");
}

/// `slap_ready(k, ab)` (4207 to 4210): the gate in `abilities_step` 3043 to 3045
/// A stub: true (no wait) until the port.
pub fn ready(f: &Fight, _k: usize, _ab: &AbilityCfg) -> bool {
    f.socket("stub:abilities::slap::ready");
    true
}
