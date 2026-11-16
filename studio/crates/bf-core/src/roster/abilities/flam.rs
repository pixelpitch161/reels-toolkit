//! Ability kind `flam`, SNARE DRUM's FLAM: for `dur_beats` beats every landed jab lands again as a hit of kind `flam` worth `mult` of it (a Studio kind; the engine's `bf_sim::kinds::sticks` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `flam`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["dur_beats"], &["mult"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dur_beats", "mult"];

/// What an ability of kind `flam` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlamCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<FlamCfg, RosterError> {
    Ok(FlamCfg {})
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
