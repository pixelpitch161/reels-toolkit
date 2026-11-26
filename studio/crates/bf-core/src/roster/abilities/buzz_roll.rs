//! Ability kind `buzz_roll`, SNARE DRUM's BUZZ ROLL: for `beats` beats a buzzing disc of `radius` ball radii hits the enemy touching it every `tick16` sixteenths for `dmg` and a knock of `knock` (a Studio kind; the engine's `bf_sim::kinds::sticks` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `buzz_roll`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["beats"], &["radius"], &["tick16"], &["dmg"], &["knock"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "beats",
    "radius",
    "tick16",
    "dmg",
    "knock",
];

/// What an ability of kind `buzz_roll` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuzzRollCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<BuzzRollCfg, RosterError> {
    Ok(BuzzRollCfg {})
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
