//! Weapon kind `tiptoe`, BASSOON's TIPTOE: `fight.py` 3280 to 3429 (`tiptoe_setup`, `_behind`, `_hop_launch`,
//! `_hop_land`, `tiptoe_tick`, `tiptoe_step`, `start_mountain`), and the ability `mountain` (`trigger` 2917 to 2918).
//!
//! A stub until BASSOON's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::tiptoe::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::{AbilityCfg, WeaponCfg};

/// Weapon kind `tiptoe`'s per-fight state (`Fight::tiptoe`): what `fight.py` hangs on the ball or the fight for this
/// kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'tiptoe'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Tiptoe(_))
}

/// `tiptoe_setup(k)` (3294 to 3303), from `run()` 5830 to 5832
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::tiptoe::setup");
}

/// `tiptoe_tick(k, k16)` (3365 to 3411), from `run()` 6055 to 6059
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::tiptoe::tick");
}

/// `tiptoe_step(k, dt)` (3412 to 3422), from `run()` 6171 to 6172
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::tiptoe::step");
}

/// `trigger`, kind `mountain` (2917 to 2918): `start_mountain(k, ab)` (3423 to 3436)
pub fn start_mountain(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::tiptoe::start_mountain");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5834 to 5835: `HOP`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `HOP` (5609 to 5615), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::tiptoe::snap");
}
