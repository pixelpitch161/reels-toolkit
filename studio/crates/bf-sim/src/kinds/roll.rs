//! Weapon kind `roll`, PIANO's PIANO ROLL: `fight.py` 5164 to 5283 (`ROLL_DEFAULT`, `roll_setup`, `roll_lane`,
//! `roll_tick`, `fire_row`, `start_runoff`, `hold_step`), the projectiles (`spawn_key` 370 to 378, their flight and
//! hits in `move` 3235 to 3278) and their `K` rows in `snap` (5509 to 5514).
//!
//! A stub until PIANO's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::roll::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use bf_core::fightfile::FightFile;

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::{FighterCfg, WeaponCfg};
use crate::rules::{Rules, SimError};

/// Weapon kind `roll`'s per-fight state (`Fight::roll`): what `fight.py` hangs on the ball or the fight for this kind,
/// indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.w['kind'] == 'roll'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Roll(_))
}

/// `roll_setup(k)` (5182 to 5198), from `run()` 5882 to 5884
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::roll::setup");
}

/// `roll_tick(k, n16)` (5213 to 5225), from `run()` 6070 to 6075: `n16` is the ABSOLUTE 16th (`n16o`, not counted from
/// the fight's first), and a stunned roller is skipped by the caller
pub fn tick(f: &mut Fight, _k: usize, _n16: i64) {
    f.socket("stub:kinds::roll::tick");
}

/// `hold_step()` (5262 to 5293), from `run()` 6113 to 6114 (`if rollers`), before `move`
pub fn hold_step(f: &mut Fight) {
    f.socket("stub:kinds::roll::hold_step");
}

/// `move`, the projectiles (3235 to 3278): `steer` (curving keys), the flight, `ricochet`, `kill_keys(out, 'wall')`,
/// the hits on each ball (`damage(..., 'key', ...)`, the volley / roll growth 3263 to 3268, the nudge), after the
/// balls' clash
pub fn keys_move(f: &mut Fight, _dt: f64) {
    f.socket("stub:kinds::roll::keys_move");
}

/// `snap`, `K` (5509 to 5514): the first `min(KMAX, len(kp))` rows of this frame's `K` (72 x 7, all NaN on entry) from
/// `fight.keys`: `[x, y, atan2(vy, vx), owner, id, idx, vol]`
pub(crate) fn k_rows(f: &Fight, _k: &mut [f64]) {
    f.socket("stub:kinds::roll::k_rows");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5886 to 5887: `ROLL`, `HOLD` when a
/// fighter is a roller. None until the port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `ROLL` and `HOLD` (5621 to 5624), into `out` (only the keys [`record_keys`]
/// gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::roll::snap");
}

/// `kit/story.py` 33 to 35 (`rules()`): a roll fighter's rows are its line, so the fight's rules carry `roll_rows`
/// (`roll_rows(name)`, 41 to 75, by fighter index) and `roll_start8 = 8 * LOOP_START` (`Rules::roll_rows`,
/// `Rules::roll_start8`). Called by `setup_from` after the pair rules; nothing to do for a fight without a roller.
pub fn feed_rules(_ff: &FightFile, fighters: [&FighterCfg; 2], _rules: &mut Rules) -> Result<(), SimError> {
    if fighters.iter().any(|f| matches!(f.weapon, WeaponCfg::Roll(_))) {
        // a stub: PIANO's port fills it (no roller can be in a fight before then: the roster reads none)
        return Err(SimError::new(
            "fighters",
            "the engine has not ported the roll's rows (weapon kind \"roll\") yet",
        ));
    }
    Ok(())
}
