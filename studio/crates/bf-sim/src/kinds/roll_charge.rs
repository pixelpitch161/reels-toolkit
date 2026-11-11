//! Weapon kind `roll_charge`, BASS DRUM's STEAMROLLER: `fight.py` 5284 to 5431 (`rollc_setup`, `rollc_blocked`,
//! `_roll_line`, `_start_roll`, `_end_roll`, `rollc_tick`, `rollc_step`), and the abilities `heartbeat`, `drum_roll`
//! and `cannons` (`trigger` 2983 to 2988; 5432 to 5494).
//!
//! A stub until BASS DRUM's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::roll_charge::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python, every name
//! and argument as `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches
//! no other engine file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::{AbilityCfg, WeaponCfg};

/// Weapon kind `roll_charge`'s per-fight state (`Fight::roll_charge`): what `fight.py` hangs on the ball or the fight
/// for this kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'roll_charge'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads
/// the kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::RollCharge(_))
}

/// `rollc_setup(k)` (5294 to 5305), from `run()` 5906 to 5908
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::roll_charge::setup");
}

/// `rollc_tick(k, k16)` (5360 to 5372), from `run()` 6091 to 6095 (its counter is `n16r`, which the older `rattle`
/// block shares)
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::roll_charge::tick");
}

/// `rollc_step(k, dt)` (5373 to 5431), from `run()` 6189 to 6190
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::roll_charge::step");
}

/// `trigger`, kind `heartbeat` (2983 to 2984): `start_heartbeat(k, ab)` (5446 to 5450)
pub fn start_heartbeat(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::roll_charge::start_heartbeat");
}

/// `trigger`, kind `drum_roll` (2985 to 2986): `start_drum_roll(k, ab)` (5466 to 5476)
pub fn start_drum_roll(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::roll_charge::start_drum_roll");
}

/// `trigger`, kind `cannons` (2987 to 2988): `start_cannons(k, ab)` (5477 to 5494; its slams have `how = 'cannon'`,
/// `Slam::how`)
pub fn start_cannons(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::roll_charge::start_cannons");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5910 to 5911: `RCHG`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `RCHG` (5713 to 5732), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::roll_charge::snap");
}
