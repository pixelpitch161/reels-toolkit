//! Weapon kind `surf`, UKULELE's SURF'S UP: `fight.py` 502 to 527 (`surf_dist`, `fire_surf`) and the surfs of
//! `surf_rake_step` (588 to 609), on the `dc` 16th counter of `run()` (5964 to 5974, shared with the rake: one counter,
//! `Schedule::dc_next`).
//!
//! A stub until UKULELE's port (milestone 4). Every function here is already called where `Fight.run()` calls its
//! Python namesake (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`] (or a test of this
//! kind's own lists), which is false until the roster reads the kind (`roster::kinds::surf::PORTED`). So a stub body is
//! never reached in a fight; with `Fight::trace_sockets` on, one that is reached logs its name. The port fills the
//! bodies with the Python, every name and argument as `fight.py` has it, every draw from the fight's generator in
//! Python's order and number, and touches no other engine file.

use crate::fight::Fight;
use crate::roster::WeaponCfg;

/// One entry of `self.surfs`: its fields are filled in by UKULELE's port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wave {}

/// This kind's per-fight state (`Fight::surf`): what `fight.py` hangs on the ball or the fight for it, indexed by
/// fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.surfs`: the waves ridden at the enemy (`fire_surf`'s dicts).
    pub surfs: Vec<Wave>,
}

/// `B.w['kind'] == 'surf'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Surf(_))
}

/// Whether `self.surfs` holds anything (Python's truth test of the list).
pub fn live(f: &Fight) -> bool {
    !f.surf.surfs.is_empty()
}

/// The `dc` block of `run()` for a surf (5970 to 5974), after the caller's stun test: `dc_main = int(round(4 *
/// every))`, `dc_every = max(1, int(round(dc_main / ab_mult(k))))`, and on `dc_k % dc_every == 0` `fire_surf(k, dc_k %
/// dc_main == 0)` (506 to 527).
pub fn tick(f: &mut Fight, _k: usize, _dc_k: i64) {
    f.socket("stub:kinds::surf::tick");
}

/// The surfs of `surf_rake_step(dt)` (588 to 609), before its strokes; `kinds::rake::surf_rake_step` calls it.
pub fn surfs_step(f: &mut Fight) {
    f.socket("stub:kinds::surf::surfs_step");
}
