//! Weapon kind `echo`, HORN's the echo call: `fight.py` 2196 to 2281 (`echo_setup`, `echo_pos`, `_echo_blast`,
//! `echo_tick`, `echo_step`).
//!
//! A stub until HORN's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::echo::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::WeaponCfg;

/// Weapon kind `echo`'s per-fight state (`Fight::echo`): what `fight.py` hangs on the ball or the fight for this kind,
/// indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'echo'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Echo(_))
}

/// `echo_setup(k)` (2205 to 2214), from `run()` 5787 to 5789
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::echo::setup");
}

/// `echo_tick(k, k16)` (2236 to 2252), from `run()` 6009 to 6014 (a stunned fighter is skipped by the caller)
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::echo::tick");
}

/// `echo_step(k, dt)` (2253 to 2281), from `run()` 6146 to 6147
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::echo::step");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5791 to 5792: `ECHO_G`, `ECHO_N`,
/// `ECHO_R`. None until the port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `ECHO_G`, `ECHO_N`, `ECHO_R` (5539 to 5549), into `out` (only the keys
/// [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::echo::snap");
}
