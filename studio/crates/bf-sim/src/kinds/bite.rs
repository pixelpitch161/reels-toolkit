//! Weapon kind `bite`, ACCORDION's BELLOWS BITE: `fight.py` 1868 to 1904 (`bite_fire`, `bite_snap`) and its branch of
//! `grid16_fire` (1487 to 1507).
//!
//! A stub until ACCORDION's port (milestone 4). Every function here is already called where `Fight.run()` calls its
//! Python namesake (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`] (or a test of this
//! kind's own lists), which is false until the roster reads the kind (`roster::kinds::bite::PORTED`). So a stub body is
//! never reached in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills the
//! bodies with the Python, every name and argument as `fight.py` has it, every draw from the fight's generator in
//! Python's order and number, and touches no other engine file.

use crate::fight::Fight;
use crate::roster::WeaponCfg;

/// This kind's per-fight state (`Fight::bite`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'bite'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Bite(_))
}

/// Its branch of `grid16_fire(k, k16)` (1487 to 1507: jaws due to close snap on this 16th, `bite_snap` 1880 to 1904;
/// the grid test; a new bite, `bite_fire` 1868 to 1879, when none is open), from `run()` 5975 to 5980 after the
/// caller's stun test.
pub fn grid16_fire(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::bite::grid16_fire");
}
