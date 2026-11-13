//! Weapon kind `tether`, SYNTH's the cables: `fight.py` 1572 to 1660 (`_tether`, `tether_beat`, `tether_16`,
//! `tether_step`).
//!
//! A stub until SYNTH's port (milestone 4). Every function here is already called where `Fight.run()` calls its Python
//! namesake (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`] (or a test of this kind's own
//! lists), which is false until the roster reads the kind (`roster::kinds::tether::PORTED`). So a stub body is never
//! reached in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies
//! with the Python, every name and argument as `fight.py` has it, every draw from the fight's generator in Python's
//! order and number, and touches no other engine file.

use crate::fight::Fight;
use crate::roster::WeaponCfg;

/// This kind's per-fight state (`Fight::tether`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'tether'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Tether(_))
}

/// `tether_beat(k, kb)` (1578 to 1593), from `run()` 5944 to 5945.
pub fn beat(f: &mut Fight, _k: usize, _kb: i64) {
    f.socket("stub:kinds::tether::beat");
}

/// `tether_16(k, k16)` (1594 to 1607), from `run()` 5981 to 5986 (its own counter, `next16c`) after the caller's stun
/// test.
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::tether::tick");
}

/// `tether_step(k, dt)` (1608 to 1660), from `run()` 6129 to 6130.
pub fn step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:kinds::tether::step");
}
