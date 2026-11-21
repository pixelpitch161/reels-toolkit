//! Ability kind `fret_slide`, BASS GUITAR's FRET SLIDE: holds still for `wind_beats` beats, then dashes at the enemy at `speed` x the shared speed up to `span` ball radii and stops dead on contact (`dmg`, knock `knock`), staying put `settle_beats` beats (a Studio kind; the engine's `bf_sim::kinds::pick` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `fret_slide`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[
    &["wind_beats"],
    &["speed"],
    &["span"],
    &["dmg"],
    &["knock"],
    &["settle_beats"],
];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "wind_beats",
    "speed",
    "span",
    "dmg",
    "knock",
    "settle_beats",
];

/// What an ability of kind `fret_slide` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FretSlideCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<FretSlideCfg, RosterError> {
    Ok(FretSlideCfg {})
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
