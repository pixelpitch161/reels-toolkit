//! Ability kind `seven_levers`, OCTOBASS's SEVEN LEVERS: seven lever punches `gap` seconds apart, fanned over `fan` degrees, each reaching `reach` ball radii and stopping on the enemy for `dmg`; the seventh adds a knock of `knock_last` (a Studio kind; the engine's `bf_sim::kinds::levers` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `seven_levers`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["gap"], &["fan"], &["reach"], &["dmg"], &["knock_last"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "gap", "fan", "reach", "dmg", "knock_last"];

/// What an ability of kind `seven_levers` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SevenLeversCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SevenLeversCfg, RosterError> {
    Ok(SevenLeversCfg {})
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
