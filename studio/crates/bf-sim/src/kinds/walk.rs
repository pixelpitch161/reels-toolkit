//! Weapon kind `walk`, CONTRABASS's WALKING BASS: `fight.py` 3965 to 4040 (`walk_setup`, `walk_blocked`, `walk_tick`,
//! `walk_step`, `_step_land`), and the walker's side of the clash in `move` (3190 to 3208, 3227 to 3231).
//!
//! A stub until CONTRABASS's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::walk::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::WeaponCfg;

/// Weapon kind `walk`'s per-fight state (`Fight::walk`): what `fight.py` hangs on the ball or the fight for this kind,
/// indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'walk'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Walk(_))
}

/// `walk_setup(k)` (3977 to 3983), from `run()` 5876 to 5878
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::walk::setup");
}

/// `walk_tick(k, k16)` (3990 to 4017), from `run()` 6065 to 6069
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::walk::tick");
}

/// `walk_step(k, dt)` (4018 to 4028), from `run()` 6173 to 6174
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::walk::step");
}

/// `move` 3203 to 3205: a walker pushed back off the ropes in a clash has its step (`wstep`, if any) restarted where it
/// now is: `S_['p0'], S_['p1'] = W_.p.copy(), W_.p.copy()`. A ball without a step (`wstep` None) is left as it is: a
/// no-op.
pub fn clash_pinned(f: &mut Fight, _w: usize) {
    f.socket("stub:kinds::walk::clash_pinned");
}

/// `effects_step` 3078 to 3082, a ring with `grow` (the walk's step ring, 4036 to 4038) that landed: `Bw.blade_len =
/// min(Bw.w['max_reach'], Bw.blade_len + Bw.w['grow'])` for the ring's owner (and its `pull` when its weapon has
/// `pull_grow`)
pub fn ring_grow(f: &mut Fight, _owner: usize) {
    f.socket("stub:kinds::walk::ring_grow");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5880 to 5881: `WALK`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `WALK` (5634 to 5642), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::walk::snap");
}
