//! Weapon kind `wrecking`, BARI SAX's WRECKING BALL: `fight.py` 3594 to 3786 (`wreck_setup`, `_wreck_aim`,
//! `wreck_tick`, `_sails_over`, `wreck_step`) and the ability `offbeat` (SKANK, `trigger` 2911 to 2914), whose cycles
//! run in `wreck_tick`.
//!
//! A stub until BARI SAX's port (milestone 4). Every function here is already called where `Fight.run()` calls its
//! Python namesake (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`] (or a test of this
//! kind's own lists), which is false until the roster reads the kind (`roster::kinds::wrecking::PORTED`). So a stub
//! body is never reached in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills
//! the bodies with the Python, every name and argument as `fight.py` has it, every draw from the fight's generator in
//! Python's order and number, and touches no other engine file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::{AbilityCfg, WeaponCfg};

/// This kind's per-fight state (`Fight::wrecking`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'wrecking'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Wrecking(_))
}

/// `wreck_setup(k)` (3594 to 3600), from `run()` 5842 to 5844.
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::wrecking::setup");
}

/// `wreck_tick(k, k16)` (3666 to 3684), from `run()` 6040 to 6044 (its own counter, `n16w`).
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::wrecking::tick");
}

/// `wreck_step(k, dt)` (3697 to 3786), from `run()` 6167 to 6168.
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::wrecking::step");
}

/// `trigger`, kind `offbeat` (2911 to 2914): SKANK.
pub fn start_offbeat(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::wrecking::start_offbeat");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5846 to 5847: `WRECK`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `WRECK` (5577 to 5587), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::wrecking::snap");
}
