//! Weapon kind `lightning`, ELECTRIC's LIGHTNING RIFF: `fight.py` 3845 to 3922 (`POSTS`, `lightning_setup`,
//! `lightning_tick`, `bolt_source`, `bolt`), and the abilities `fork` and `tapping` (`trigger` 2933 to 2938).
//!
//! A stub until ELECTRIC's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::lightning::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python, every name
//! and argument as `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches
//! no other engine file.

use crate::fight::Fight;
use crate::roster::{AbilityCfg, WeaponCfg};

/// Weapon kind `lightning`'s per-fight state (`Fight::lightning`): what `fight.py` hangs on the ball or the fight for
/// this kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'lightning'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Lightning(_))
}

/// `lightning_setup(k)` (3857 to 3864), from `run()` 5854 to 5856
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::lightning::setup");
}

/// `lightning_tick(k, k16)` (3865 to 3881), from `run()` 6050 to 6054
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::lightning::tick");
}

/// `trigger`, kind `fork` (2933 to 2935): `fork_until`, `fork_ab`, event `fork`
pub fn start_fork(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::lightning::start_fork");
}

/// `trigger`, kind `tapping` (2936 to 2938): `tap_until` (`beats` x beat), `tap_ab`, event `tapping`
pub fn start_tapping(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::lightning::start_tapping");
}
