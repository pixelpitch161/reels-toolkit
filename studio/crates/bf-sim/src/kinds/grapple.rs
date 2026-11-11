//! Weapon kind `grapple`, CELLO's THE EMBRACE: `fight.py` 4041 to 4201 (`GRAB_STATES`, `grapple_setup`, `grapple_busy`,
//! `grapple_tick`, `_end_rush`, `_nearest_rope_angle`, `grapple_release`, `grapple_step`).
//!
//! A stub until CELLO's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::grapple::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::WeaponCfg;

/// Weapon kind `grapple`'s per-fight state (`Fight::grapple`): what `fight.py` hangs on the ball or the fight for this
/// kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'grapple'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Grapple(_))
}

/// `grapple_setup(k)` (4058 to 4064), from `run()` 5864 to 5866
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::grapple::setup");
}

/// `grapple_tick(k, k16)` (4070 to 4090), from `run()` 6060 to 6064
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::grapple::tick");
}

/// `grapple_step(k, dt)` (4113 to 4201), from `run()` 6175 to 6176
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::grapple::step");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5868 to 5869: `GRAB`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `GRAB` (5625 to 5633), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::grapple::snap");
}
