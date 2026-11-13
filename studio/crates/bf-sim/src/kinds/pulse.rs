//! Weapon kind `pulse`, DRUMS's the drumhead: `fight.py` 411 to 424 (`fire_pulse`) and the pulse counter of `run()`
//! (5946 to 5963); its rings are `Fight::waves` of kind `pulse` (with `grow` and `clears`), which `effects_step`
//! already runs.
//!
//! A stub until DRUMS's port (milestone 4). Every function here is already called where `Fight.run()` calls its Python
//! namesake (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`] (or a test of this kind's own
//! lists), which is false until the roster reads the kind (`roster::kinds::pulse::PORTED`). So a stub body is never
//! reached in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies
//! with the Python, every name and argument as `fight.py` has it, every draw from the fight's generator in Python's
//! order and number, and touches no other engine file.

use crate::fight::Fight;
use crate::roster::WeaponCfg;

/// This kind's per-fight state (`Fight::pulse`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'pulse'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Pulse(_))
}

/// The pulse block of `run()` for one fighter, after the caller's stun test (5951 to 5957): `main = 4 * every` (a
/// float), `every16 = max(1, int(round(main / ab_mult(k))))`, and on `k16 % every16 == 0` `fire_pulse(k, k16 % main ==
/// 0)` (411 to 424). From `run()` 5946 to 5963.
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::pulse::tick");
}
