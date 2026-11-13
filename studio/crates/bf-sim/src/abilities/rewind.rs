//! Ability kind `rewind` (TURNTABLES's REWIND): `fight.py` `trigger` 2866 to 2867; 1927 to 1998 (`_rewind_ab`,
//! `_rewind_then`, `rewind_ready`, `start_rewind`, `rewind_step`); the gate in `abilities_step` 3035 to 3036. A
//! rewinding ball (the ball's `rw_left` above 0, `fight::rewinding`) is intangible: `damage`, `move` and the kinds that
//! test it skip it.
//!
//! A stub until TURNTABLES's port (milestone 4). [`start`] is already called from `trigger`'s dispatch
//! (`abilities/mod.rs`), and every other function here where `Fight.run()` calls its Python namesake, behind a test
//! that is false until the roster reads the kind (`roster::abilities::rewind::PORTED`). So a stub body is never reached
//! in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the
//! Python (every draw from the fight's generator in Python's order and number) and touches no other engine file. A
//! copied ability (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the
//! kind.

use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// This kind's per-fight state (`Fight::rewind`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `rewind` (2866 to 2867): `start_rewind(k, ab)` (1946 to 1962).
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::rewind::start");
}

/// `rewind_ready(k, ab)` (1940 to 1945, with `_rewind_then` 1930 to 1939): the gate in `abilities_step` 3035 to 3036. A
/// stub: true (no wait) until the port.
pub fn ready(f: &Fight, _k: usize, _ab: &AbilityCfg) -> bool {
    f.socket("stub:abilities::rewind::ready");
    true
}

/// `rewind_step(dt, rewinders)` (1963 to 1998), from `run()` 6116 to 6117, right after `move` (`ks`: the fighters with
/// a `rewind` ability, in ball order).
pub fn rewind_step(f: &mut Fight, _dt: f64, _ks: &[usize]) {
    f.socket("stub:abilities::rewind::rewind_step");
}
