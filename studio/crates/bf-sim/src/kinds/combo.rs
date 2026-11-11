//! Weapon kind `combo`, TRUMPET's the valve combo: `fight.py` 898 to 966 (`combo_setup`, `combo_beat`, `combo_step`).
//!
//! A stub until TRUMPET's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::combo::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::roster::WeaponCfg;

/// Weapon kind `combo`'s per-fight state (`Fight::combo`): what `fight.py` hangs on the ball or the fight for this
/// kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'combo'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Combo(_))
}

/// `combo_setup(k)` (899 to 904), from `run()` 5756 to 5758
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::combo::setup");
}

/// `combo_beat(k, kb)` (905 to 917), from `run()` 5934 to 5935
pub fn beat(f: &mut Fight, _k: usize, _kb: i64) {
    f.socket("stub:kinds::combo::beat");
}

/// `combo_step(k, dt)` (918 to 966), from `run()` 6123 to 6124
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::combo::step");
}
