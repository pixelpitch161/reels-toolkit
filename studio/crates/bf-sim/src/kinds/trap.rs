//! Weapon kind `trap`, HARP's the strings: `fight.py` 1324 to 1332 (`hazard_beat`, shared with `line`), 1363 to 1392
//! (`lay_string`) and the strings' half of `hazards_step` (1417 to 1434). A string's id comes from the hazards' shared
//! counter (`self.next_hz`, `Fight::line`'s `next_hz`).
//!
//! A stub until HARP's port (milestone 4). Every function here is already called where `Fight.run()` calls its Python
//! namesake (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`] (or a test of this kind's own
//! lists), which is false until the roster reads the kind (`roster::kinds::trap::PORTED`). So a stub body is never
//! reached in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies
//! with the Python, every name and argument as `fight.py` has it, every draw from the fight's generator in Python's
//! order and number, and touches no other engine file.

use crate::fight::Fight;
use crate::roster::WeaponCfg;

/// One entry of `self.strings`: its fields are filled in by HARP's port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Strand {}

/// This kind's per-fight state (`Fight::trap`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.strings`: the strings across the ring (`lay_string`'s dicts).
    pub strings: Vec<Strand>,
}

/// `B.w['kind'] == 'trap'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Trap(_))
}

/// Whether `self.strings` holds anything (Python's truth test of the list).
pub fn live(f: &Fight) -> bool {
    !f.trap.strings.is_empty()
}

/// `hazard_beat(k, kb)` for a `trap` (1324 to 1332: on its grid, `lay_string` 1363 to 1392), from `run()` 5940 to 5941.
pub fn beat(f: &mut Fight, _k: usize, _kb: i64) {
    f.socket("stub:kinds::trap::beat");
}

/// The strings' half of `hazards_step(dt)` (1417 to 1434), from `run()` 6141 to 6142 (`if self.lines or self.strings`),
/// after the pipes' half (`kinds::line::hazards_step`).
pub fn strings_step(f: &mut Fight, _dt: f64) {
    f.socket("stub:kinds::trap::strings_step");
}
