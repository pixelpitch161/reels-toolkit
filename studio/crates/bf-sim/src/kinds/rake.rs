//! Weapon kind `rake`, ACOUSTIC's SIX-STRING RAKE: `fight.py` 497 to 700: `fire_rake` (528 to 545), `aim_blend`,
//! `rake_lateral`, `sweep_front`, the rake branches of `dash_cone_step` (457 to 501) and `surf_rake_step` (585 to 700);
//! the `dash`, `cone`, `surf` and `spout` branches belong to older kinds.
//!
//! A stub until ACOUSTIC's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::rake::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::roster::WeaponCfg;

/// Weapon kind `rake`'s per-fight state (`Fight::rake`): what `fight.py` hangs on the ball or the fight for this kind,
/// indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'rake'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Rake(_))
}

/// The `dc` 16th block of `run()` (5964 to 5974) for one fighter, after the caller's stun test: `dc_main =
/// int(round(4 * every))`, `dc_every = max(1, int(round(dc_main / ab_mult(k))))`, and on `dc_k % dc_every == 0`
/// `fire_rake(k, dc_k % dc_main == 0)`
pub fn tick(f: &mut Fight, _k: usize, _dc_k: i64) {
    f.socket("stub:kinds::rake::tick");
}

/// `dash_cone_step(dt)` (457 to 501), from `run()` 6195 to 6196 (`if dc`)
pub fn dash_cone_step(f: &mut Fight, _dt: f64) {
    f.socket("stub:kinds::rake::dash_cone_step");
}

/// `surf_rake_step(dt)` (585 to 700), from `run()` 6197 to 6198 (`if dc or self.whips or self.sweeps or self.spouts`)
pub fn surf_rake_step(f: &mut Fight, _dt: f64) {
    f.socket("stub:kinds::rake::surf_rake_step");
}
