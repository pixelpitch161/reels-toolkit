//! Weapon kind `spotlight`, ALTO SAX's SPOTLIGHT SOLO: `fight.py` 4369 to 4512 (`spot_setup`, `_spot_line`,
//! `spot_tick`, `spot_pos`, `spot_step`, `start_encore`), and the abilities `octave`, `altissimo` and `encore`
//! (`trigger` 2958 to 2965).
//!
//! A stub until ALTO SAX's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::spotlight::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python, every name
//! and argument as `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches
//! no other engine file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::{AbilityCfg, WeaponCfg};

/// Weapon kind `spotlight`'s per-fight state (`Fight::spotlight`): what `fight.py` hangs on the ball or the fight for
/// this kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'spotlight'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Spotlight(_))
}

/// `spot_setup(k)` (4387 to 4393), from `run()` 5888 to 5890
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::spotlight::setup");
}

/// `spot_tick(k, k16)` (4399 to 4460), from `run()` 6076 to 6080
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::spotlight::tick");
}

/// `spot_step(k, dt)` (4471 to 4500), from `run()` 6183 to 6184
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::spotlight::step");
}

/// `trigger`, kind `octave` (2958 to 2960)
pub fn start_octave(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::spotlight::start_octave");
}

/// `trigger`, kind `altissimo` (2961 to 2963)
pub fn start_altissimo(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::spotlight::start_altissimo");
}

/// `trigger`, kind `encore` (2964 to 2965): `start_encore(k, ab)` (4501 to 4537)
pub fn start_encore(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::spotlight::start_encore");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5892 to 5893: `SPOT`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `SPOT` (5671 to 5681), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::spotlight::snap");
}
