//! Weapon kind `inflate`, BAGPIPES's BAG INFLATE: `fight.py` 1763 to 1867 (`inflate_setup`, `inflate_start`,
//! `inflate_step`, `inflate_clash`) and its branch of `grid16_fire` (1487 to 1507). The per-ball radius, mass and speed
//! it sets are the ball's `rad`, `mass` and `vmul`, which `move` reads through `fight::radius` and `fight::mass`.
//!
//! A stub until BAGPIPES's port (milestone 4). Every function here is already called where `Fight.run()` calls its
//! Python namesake (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`] (or a test of this
//! kind's own lists), which is false until the roster reads the kind (`roster::kinds::inflate::PORTED`). So a stub body
//! is never reached in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills the
//! bodies with the Python, every name and argument as `fight.py` has it, every draw from the fight's generator in
//! Python's order and number, and touches no other engine file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::WeaponCfg;

/// This kind's per-fight state (`Fight::inflate`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'inflate'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Inflate(_))
}

/// `inflate_setup(k)` (1763 to 1776), from `run()` 5756 to 5762 (the `combo` / `flail` / `inflate` loop): the ball's
/// `rad`, `mass`, `vmul`, and its contact hook (`clash_hooks`, which `move` runs as [`clash`]).
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::inflate::setup");
}

/// Its branch of `grid16_fire(k, k16)` (1487 to 1507: the grid test, then `inflate_start` 1777 to 1791), from `run()`
/// 5975 to 5980 after the caller's stun test.
pub fn grid16_fire(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::inflate::grid16_fire");
}

/// `inflate_step(k, dt)` (1792 to 1838), from `run()` 6131 to 6132.
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::inflate::step");
}

/// `inflate_clash(k)` (1839 to 1867): the contact hook `move` runs after a clash (`clash_hooks`, 3233 to 3234), once
/// per inflater in the order `inflate_setup` added them.
pub fn clash(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::inflate::clash");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5763 to 5764: `RAD`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `RAD` (5520 to 5521), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::inflate::snap");
}
