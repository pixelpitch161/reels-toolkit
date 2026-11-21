//! Ability kind `headstock`, BASS GUITAR's HEADSTOCK SWING: the neck grows to `len` ball radii and swings back over `wind_beats` beats, then sweeps `sweep` degrees round to the enemy over `swing_beats` beats and stops dead on it (`dmg`, knock `knock`) (a Studio kind; the engine's `bf_sim::kinds::pick` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `headstock`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[
    &["len"],
    &["sweep"],
    &["wind_beats"],
    &["swing_beats"],
    &["dmg"],
    &["knock"],
];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "len",
    "sweep",
    "wind_beats",
    "swing_beats",
    "dmg",
    "knock",
];

/// What an ability of kind `headstock` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeadstockCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<HeadstockCfg, RosterError> {
    Ok(HeadstockCfg {})
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
