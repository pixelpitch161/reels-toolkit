//! Weapon kind `rests`, VIOLA's COUNTING RESTS: `fight.py` 5021 to 5107 (`rests_setup`, `fire_note`, `rests_tick`,
//! `_crush`), 5137 to 5181 (`notes_step`), and the abilities `double_stop` (5108 to 5117) and `melody` (`trigger` 2923
//! to 2927). The ball's `resting` and `guard` are what `damage` reads (342 to 343); `cur_kb`, `melody_until`,
//! `melody_k0` and `melody_rest` are on the ball too.
//!
//! A stub until VIOLA's port (milestone 4). Every function here is already called where `Fight.run()` calls its Python
//! namesake (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`] (or a test of this kind's own
//! lists), which is false until the roster reads the kind (`roster::kinds::rests::PORTED`). So a stub body is never
//! reached in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies
//! with the Python, every name and argument as `fight.py` has it, every draw from the fight's generator in Python's
//! order and number, and touches no other engine file.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::{AbilityCfg, WeaponCfg};

/// One entry of `self.notes`: its fields are filled in by VIOLA's port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Note {}

/// This kind's per-fight state (`Fight::rests`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.notes`: the whole notes rolling or dropping (`fire_note`'s dicts).
    pub notes: Vec<Note>,
}

/// `B.w['kind'] == 'rests'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Rests(_))
}

/// Whether `self.notes` holds anything (Python's truth test of the list).
pub fn live(f: &Fight) -> bool {
    !f.rests.notes.is_empty()
}

/// `rests_setup(k)` (5021 to 5033), from `run()` 5858 to 5860.
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::rests::setup");
}

/// `rests_tick(k, k16)` (5064 to 5086), from `run()` 6045 to 6049 (its own counter, `n16v`).
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::rests::tick");
}

/// `notes_step(dt)` (5137 to 5181), from `run()` 6191 to 6192 (`if getattr(self, 'notes', None)`).
pub fn notes_step(f: &mut Fight, _dt: f64) {
    f.socket("stub:kinds::rests::notes_step");
}

/// `trigger`, kind `double_stop` (2928 to 2929): `start_double_stop` (5108 to 5117).
pub fn start_double_stop(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::rests::start_double_stop");
}

/// `trigger`, kind `melody` (2923 to 2927): the ball's `melody_until`, `melody_k0`, `melody_rest` and the `melody`
/// event.
pub fn start_melody(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::rests::start_melody");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5862 to 5863 (with `NOTES`): `REST`.
/// None until the port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `REST` (5616 to 5620), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::rests::snap");
}
