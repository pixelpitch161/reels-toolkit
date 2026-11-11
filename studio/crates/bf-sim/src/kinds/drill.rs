//! Weapon kind `drill`, SOUSAPHONE's DRILL SET: `fight.py` 4758 to 5005 (`drill_setup`, `drill_blocked`, `_drill_grow`,
//! `drill_tick`, `_drill_blast`, `_capsule_hit`, `_trample_baskets`, `drill_step`, `start_flash`, `start_front`), and
//! the abilities `flash_spin` and `company_front` (`trigger` 2977 to 2982).
//!
//! A stub until SOUSAPHONE's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::drill::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::{AbilityCfg, WeaponCfg};

/// Weapon kind `drill`'s per-fight state (`Fight::drill`): what `fight.py` hangs on the ball or the fight for this
/// kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.march` while a drill set marches (`drill_tick` 4799 to 4858): `Ball::march`. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct March {}

/// `B.w['kind'] == 'drill'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
/// It is also Python's `hasattr(B, 'march')` (`trigger` 2978, 2981; `snap` 5648): only `drill_setup` gives a ball
/// `march`.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Drill(_))
}

/// `drill_setup(k)` (4781 to 4790), from `run()` 5900 to 5902
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::drill::setup");
}

/// `drill_tick(k, k16)` (4799 to 4858), from `run()` 6086 to 6090
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::drill::tick");
}

/// `drill_step(k, dt)` (4889 to 4984), from `run()` 6187 to 6188
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::drill::step");
}

/// `trigger`, kind `flash_spin` (2977 to 2979): `if hasattr(B, 'march'): start_flash(k, ab)` (4985 to 4994). Python
/// tests `hasattr(B, 'march')` (2978; [`is`]): the stub must too.
pub fn start_flash(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::drill::start_flash");
}

/// `trigger`, kind `company_front` (2980 to 2982): `if hasattr(B, 'march'): start_front(k, ab)` (4995 to 5005). Python
/// tests `hasattr(B, 'march')` (2981; [`is`]): the stub must too.
pub fn start_front(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::drill::start_front");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5904 to 5905: `DRILL`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `DRILL` (5643 to 5670), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::drill::snap");
}
