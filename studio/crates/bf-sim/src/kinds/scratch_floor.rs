//! Weapon kind `scratch_floor`, TURNTABLES's THE RING IS THE RECORD: `fight.py` 1713 to 1762 (`scratch_beat`,
//! `scratch_step`).
//!
//! A stub until TURNTABLES's port (milestone 4). Every function here is already called where `Fight.run()` calls its
//! Python namesake (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`] (or a test of this
//! kind's own lists), which is false until the roster reads the kind (`roster::kinds::scratch_floor::PORTED`). So a
//! stub body is never reached in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port
//! fills the bodies with the Python, every name and argument as `fight.py` has it, every draw from the fight's
//! generator in Python's order and number, and touches no other engine file.

use crate::fight::Fight;
use crate::roster::WeaponCfg;

/// This kind's per-fight state (`Fight::scratch_floor`): what `fight.py` hangs on the ball or the fight for it, indexed
/// by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'scratch_floor'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads
/// the kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::ScratchFloor(_))
}

/// `scratch_beat(k, kb)` (1713 to 1730), from `run()` 5942 to 5943.
pub fn beat(f: &mut Fight, _k: usize, _kb: i64) {
    f.socket("stub:kinds::scratch_floor::beat");
}

/// `scratch_step(k, dt)` (1731 to 1762), from `run()` 6127 to 6128.
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::scratch_floor::step");
}
