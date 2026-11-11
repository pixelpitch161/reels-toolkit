//! Weapon kind `jump`, CLARINET's TWELFTH JUMP: `fight.py` 2338 to 2424 (`jump_setup`, `_jump`, `jump_tick`), and the
//! ability `rhapsody` (`trigger` 2930 to 2932; its chain runs in `jump_tick`).
//!
//! A stub until CLARINET's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::jump::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::roster::{AbilityCfg, WeaponCfg};

/// Weapon kind `jump`'s per-fight state (`Fight::jump`): what `fight.py` hangs on the ball or the fight for this kind,
/// indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'jump'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Jump(_))
}

/// `jump_setup(k)` (2349 to 2354), from `run()` 5793 to 5795
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::jump::setup");
}

/// `jump_tick(k, k16)` (2396 to 2424), from `run()` 6015 to 6019 (no stun test in the caller)
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::jump::tick");
}

/// `trigger`, kind `rhapsody` (2930 to 2932): `B.jump_tgt = None`, `B.rhap = dict(ab=ab, n=ab['n'], side=1.0,
/// next=None)`
pub fn start_rhapsody(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::jump::start_rhapsody");
}
