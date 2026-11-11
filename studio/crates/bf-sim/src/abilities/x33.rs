//! Ability kinds `whip` (ACOUSTIC's FINGERPICK), `sweep` (ACOUSTIC's BIG STRUM): `fight.py` `trigger` 2872 to 2873;
//! `ab_x33` 566 to 584 and the whips of `surf_rake_step` 585 to 700.
//!
//! A stub until ACOUSTIC's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and every
//! other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the roster
//! reads the kind (`roster::abilities::whip::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::x33`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kinds `whip` and `sweep` (2872 to 2873): `ab_x33(k, ab, kind)` (566 to 584), `kind` being the ability's
/// kind (`spout`, the third, is an older kind).
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg, _kind: &str) {
    f.socket("stub:abilities::x33::start");
}

/// Whether `self.whips` or `self.sweeps` holds anything: half of the test before `surf_rake_step` (`run()` 6197)
/// A stub: false (nothing) until the port.
pub fn live(_f: &Fight) -> bool {
    false
}
