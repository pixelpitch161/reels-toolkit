//! Ability kind `beam` (TRUMPET's HIGH C): `fight.py` `trigger` 2870 to 2871; `ab_beam_charge` 2990 to 2999,
//! `ab_beam_fire` 3001 to 3010, `abilities_step` 3013 to 3016, the ability beams of `beams_step` 857 to 897.
//!
//! A stub until TRUMPET's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::beam::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::beam`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `beam`: `trigger` 2870 to 2871.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::beam::start");
}

/// Whether `self.beams` holds anything: half of the test before `beams_step` (`run()` 6139; the other half, a `beam`
/// weapon, is an older kind)
/// A stub: false (nothing) until the port.
pub fn live(_f: &Fight) -> bool {
    false
}

/// `abilities_step` 3013 to 3016: a charged ability beam fires when its beat comes (`ab_beam_fire(k)`, 3001 to 3010),
/// before any fighter's levels
pub fn check_charge(f: &mut Fight, _k: usize) {
    f.socket("stub:abilities::beam::check_charge");
}

/// `beams_step(dt)` (857 to 897), from `run()` 6139 to 6140 (`if self.beams or any(B.w['kind'] == 'beam' ...)`: the
/// stub tests that itself; the `beam` weapon is an older kind)
pub fn beams_step(f: &mut Fight, _dt: f64) {
    f.socket("stub:abilities::beam::beams_step");
}
