//! Ability kind `below_hearing`, OCTOBASS's BELOW HEARING: the music drops out while one slow ring grows from the ball at `speed` ring units a second; when it reaches the enemy (or after `max_s` seconds) it hits for `dmg`, a knock of `knock` and a stun of `stun_beats` beats (a Studio kind; the engine's `bf_sim::kinds::levers` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `below_hearing`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["speed"], &["max_s"], &["dmg"], &["knock"], &["stun_beats"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "speed",
    "max_s",
    "dmg",
    "knock",
    "stun_beats",
];

/// What an ability of kind `below_hearing` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BelowHearingCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<BelowHearingCfg, RosterError> {
    Ok(BelowHearingCfg {})
}

/// Its numbers must be finite and not negative.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    for key in ALLOWED.iter().skip(3) {
        if let Some(x) = ab.get(key)
            && !(x.is_finite() && x >= 0.0)
        {
            return Err(e.err(&format!("{prefix}.{key}"), "must be a finite number, not negative"));
        }
    }
    Ok(())
}
