//! Weapon kind `crowd`, TENOR SAX's CROWD PLEASER: `fight.py` 4513 to 4757 (`crowd_setup`, `_crowd_volley`,
//! `_launch_throw`, `crowd_tick`, `crowd_step`, `start_runway`, `strut_step`, `start_dive`, `dive_step`), and the
//! abilities `runway`, `subtone` and `stagedive` (`trigger` 2966 to 2976).
//!
//! A stub until TENOR SAX's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::crowd::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::{AbilityCfg, WeaponCfg};

/// Weapon kind `crowd`'s per-fight state (`Fight::crowd`): what `fight.py` hangs on the ball or the fight for this
/// kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.strut` while WALK THE BAR runs (`start_runway` 4648 to 4664): `Ball::strut`. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Strut {}

/// `B.dive` while STAGE DIVE runs (`start_dive` 4692 to 4709): `Ball::dive`. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dive {}

/// `B.w['kind'] == 'crowd'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Crowd(_))
}

/// `crowd_setup(k)` (4538 to 4551), from `run()` 5894 to 5896
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::crowd::setup");
}

/// `crowd_tick(k, k16)` (4588 to 4609), from `run()` 6081 to 6085
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::crowd::tick");
}

/// `crowd_step(k, dt)` (4610 to 4647), from `run()` 6185 to 6186
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::crowd::step");
}

/// `trigger`, kind `runway` (2966 to 2968): `if B.dive is None: start_runway(k, ab)` (4648 to 4664). Python tests
/// `B.dive is None` at the call site (2967): the stub must too.
pub fn start_runway(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::crowd::start_runway");
}

/// `trigger`, kind `subtone` (2969 to 2972): `hush`, the pending throws of `k` dropped, event `hush`
pub fn start_subtone(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::crowd::start_subtone");
}

/// `trigger`, kind `stagedive` (2973 to 2976): `B.strut = None`, `B.hush = None`, then `start_dive(k, ab)` (4692
/// to 4709). Python clears `B.strut` and `B.hush` before it (2974 to 2975): the stub must too.
pub fn start_dive(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::crowd::start_dive");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5898 to 5899: `CROWD`, `THROWS`. None
/// until the port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `CROWD` and `THROWS` (5682 to 5704), into `out` (only the keys
/// [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::crowd::snap");
}
