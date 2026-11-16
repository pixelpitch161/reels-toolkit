//! Ability kind `tremolo`, MARIMBA's TREMOLO (a Studio kind; the engine's `bf_sim::kinds::mallets` runs it): for `dur_beats` beats every mallet in the air buzzes as a pair and a hit lands twice (a second hit worth `mult` of the first).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `tremolo`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["dur_beats"], &["mult"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dur_beats", "mult"];

/// What an ability of kind `tremolo` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TremoloCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<TremoloCfg, RosterError> {
    Ok(TremoloCfg {})
}

/// Its numbers must be finite and not negative.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(x) = ab.get("dur_beats")
        && !(x.is_finite() && x >= 0.0)
    {
        return Err(e.err(&format!("{prefix}.dur_beats"), "must be a finite number, not negative"));
    }
    if let Some(x) = ab.get("mult")
        && !(x.is_finite() && x >= 0.0)
    {
        return Err(e.err(&format!("{prefix}.mult"), "must be a finite number, not negative"));
    }
    Ok(())
}
