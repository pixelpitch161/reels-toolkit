//! Ability kind `drum_roll`, BASS DRUM's DRUM ROLL: its roster settings. `fight.py` `trigger` 2985 to 2986;
//! `_next_leg`, `start_drum_roll` 5451 to 5476.
//!
//! For `beats` beats the drum rolls nonstop: legs of `leg16` 16ths (`int(ab.get('leg16', 3))`) with no tipping, each
//! aimed at the enemy turned `zig` degrees to alternate sides (`ab.get('zig', 30.0)`), at `speed_k` x the roll's
//! speed (`ab.get('speed_k', 1.0)`), a contact every `cool` s at most (`ab.get('cool', 0.25)`) for `dmg` and `knock`
//! (`fwd` of it on ahead: `ab.get('fwd', w.get('fwd', 0.4))`). The entry holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `drum_roll`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (`start_drum_roll`, `rollc_step`): `beats`, `dmg`,
/// `knock`.
pub const REQUIRED: &[&[&str]] = &[&["beats"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("drum_roll+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "beats",
    "leg16",
    "zig",
    "speed_k",
    "cool",
    "dmg",
    "knock",
    "fwd",
];

/// What an ability of kind `drum_roll` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrumRollCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<DrumRollCfg, RosterError> {
    Ok(DrumRollCfg {})
}

/// The kind's own checks: `int(ab.get('leg16', 3))` raises for a number that is not finite, and a `speed_k` of 0
/// raises a division by zero at the first leg (`0.5 * span / (V * self.R.V)`).
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(leg) = ab.get("leg16")
        && !leg.is_finite()
    {
        return Err(e.err(&join(prefix, "leg16"), format!("{leg:e} is not a finite number")));
    }
    if ab.get("speed_k") == Some(0.0) {
        return Err(e.err(
            &join(prefix, "speed_k"),
            "0 is no speed (a leg's length in time divides by it)",
        ));
    }
    Ok(())
}
