//! Weapon kind `anchor`, BASS CLARINET's ENDPIN ANCHOR: `fight.py` 2568 to 2658 (`anchor_setup`, `anchor_plant`,
//! `anchor_release`, `anchor_tick`, `anchor_step`).
//!
//! A stub until BASS CLARINET's port. Every function here is already called where `Fight.run()` calls its Python
//! namesake (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the
//! roster reads the kind (`roster::kinds::anchor::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python, every name
//! and argument as `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches
//! no other engine file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::{AnchorCfg, WeaponCfg};

/// Weapon kind `anchor`'s per-fight state (`Fight::anchor`): what `fight.py` hangs on the ball or the fight for this
/// kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.anc` while the ball whirls round a peg (`anchor_plant` 2584 to 2601): `Ball::anc`. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Anc {}

/// `B.w['kind'] == 'anchor'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Anchor(_))
}

/// `anchor_setup(k)` (2579 to 2583), from `run()` 5818 to 5820 (5821 to 5823, `B.anc = None` on both balls, is
/// `Ball::anc`'s default)
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::anchor::setup");
}

/// `anchor_tick(k, k16)` (2611 to 2619), from `run()` 6030 to 6034
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::anchor::tick");
}

/// `anchor_step(k, dt)` (2620 to 2658), from `run()` 6159 to 6162: for either ball whose `anc` is set (a copied whirl
/// too), while a fighter is an anchorer
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::anchor::step");
}

/// `anchor_plant(k, w, orbit)` (2584 to 2601); IDENTITY CRISIS's signature copy calls it with the enemy's weapon
/// (`start_copy` 2818 to 2819)
pub fn anchor_plant(f: &mut Fight, _k: usize, _w: &AnchorCfg, _orbit: f64) {
    f.socket("stub:kinds::anchor::anchor_plant");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5825 to 5826: `ANC`. None until the
/// port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `ANC` (5569 to 5576), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::anchor::snap");
}
