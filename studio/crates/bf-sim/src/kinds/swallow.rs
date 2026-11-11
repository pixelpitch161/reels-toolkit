//! Weapon kind `swallow`, TUBA's the bell: `fight.py` 1045 to 1130 (`swallow_setup`, `swallow_beat`, `swallow_step`
//! with the `swallow_rollers` rule at 1100).
//!
//! A stub until TUBA's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::swallow::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::roster::WeaponCfg;

/// Weapon kind `swallow`'s per-fight state (`Fight::swallow`): what `fight.py` hangs on the ball or the fight for this
/// kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'swallow'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Swallow(_))
}

/// `swallow_setup(k)` (1046 to 1052), from `run()` 5765 to 5767 (the same loop as the striker's setup, before it)
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::swallow::setup");
}

/// `swallow_beat(k, kb)` (1053 to 1063), from `run()` 5936 to 5937
pub fn beat(f: &mut Fight, _k: usize, _kb: i64) {
    f.socket("stub:kinds::swallow::beat");
}

/// `swallow_step(k, dt)` (1064 to 1130), from `run()` 6133 to 6134
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::swallow::step");
}
