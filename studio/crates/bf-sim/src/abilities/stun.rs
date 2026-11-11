//! Ability kind `stun` (CLARINET's SQUEAK): `fight.py` `trigger` 2889 to 2892; the stun hit of `effects_step` 3072
//! to 3074.
//!
//! A stub until CLARINET's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::stun::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::stun`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `stun`: `trigger` 2889 to 2892.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::stun::start");
}

/// `effects_step` 3072 to 3074, a ring with `stun` that landed: `O.stun = max(getattr(O, 'stun', 0.0), W['stun'])`,
/// event `stun`
pub fn hit(f: &mut Fight, _o: usize, _owner: usize, _stun: f64) {
    f.socket("stub:abilities::stun::hit");
}
