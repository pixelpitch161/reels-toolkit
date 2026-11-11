//! Weapon kind `line`, ORGAN's the pipes: `fight.py` 1323 to 1485 (`hazard_beat`, `_to_ropes`, `fire_line`,
//! `hazards_step`; `lay_string` and `strings` belong to the older `trap` kind).
//!
//! A stub until ORGAN's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::line::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::roster::WeaponCfg;

/// Weapon kind `line`'s per-fight state (`Fight::line`): what `fight.py` hangs on the ball or the fight for this kind,
/// indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'line'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Line(_))
}

/// `hazard_beat(k, kb)` (1324 to 1332), from `run()` 5940 to 5941
pub fn beat(f: &mut Fight, _k: usize, _kb: i64) {
    f.socket("stub:kinds::line::beat");
}

/// `hazards_step(dt)` (1393 to 1434), from `run()` 6141 to 6142 (`if self.lines or self.strings`: the stub tests that
/// itself)
pub fn hazards_step(f: &mut Fight, _dt: f64) {
    f.socket("stub:kinds::line::hazards_step");
}
