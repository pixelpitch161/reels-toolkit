//! Weapon kind `tune`, OBOE's TUNING A: `fight.py` 2425 to 2525 (`tune_setup`, `_detune`, `tune_tick`, `tune_step`),
//! and the ability `endless` (`trigger` 2884 to 2888).
//!
//! A stub until OBOE's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::tune::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::abilities::detune::DetuneCfg;
use crate::roster::{AbilityCfg, WeaponCfg};

/// Weapon kind `tune`'s per-fight state (`Fight::tune`): what `fight.py` hangs on the ball or the fight for this kind,
/// indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'tune'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Tune(_))
}

/// `tune_setup(k)` (2436 to 2446), from `run()` 5797 to 5799
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::tune::setup");
}

/// `tune_tick(k, k16)` (2452 to 2495), from `run()` 6020 to 6024 (no stun test in the caller)
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::tune::tick");
}

/// `tune_step(k, dt)` (2496 to 2525), from `run()` 6150 to 6151
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::tune::step");
}

/// `_detune(vic, att, depth, dur, weak)` (2447 to 2451), from `effects_step` 3069 to 3071 (a ring with `detune` that
/// landed: `dur` is `detune['dur_beats'] * beat`)
pub fn detune(f: &mut Fight, _vic: usize, _att: usize, _depth: f64, _dur: f64, _weak: f64) {
    f.socket("stub:kinds::tune::detune");
}

/// `effects_step` 3069 to 3071, a ring with `detune` that landed on `vic`: `dd_ = W['detune']; self._detune(o,
/// W['owner'], dd_['depth'], dd_['dur_beats'] * self.R.beat, dd_['weak'])` (it reads the option, then calls
/// [`detune`]). The stub computes `dur = dur_beats * beat` here, as `effects_step` does.
pub fn wave_detune(f: &mut Fight, _vic: usize, _att: usize, _detune: &DetuneCfg) {
    f.socket("stub:kinds::tune::wave_detune");
}

/// `trigger`, kind `endless` (2884 to 2888): `tune_lock`, `tbeams`, `endless`, event `endless`
pub fn start_endless(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::tune::start_endless");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5801 to 5802: `DET`, `TUNE`. None until
/// the port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `DET` and `TUNE` (5550 to 5554), into `out` (only the keys [`record_keys`]
/// gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::tune::snap");
}
