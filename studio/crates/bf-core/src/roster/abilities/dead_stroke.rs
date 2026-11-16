//! Ability kind `dead_stroke`, MARIMBA's DEAD STROKE (a Studio kind; the engine's `bf_sim::kinds::mallets` runs it): arms the marimba for `dur_beats` beats; the next mallet to land pins the enemy for `pin_beats` beats.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `dead_stroke`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["dur_beats"], &["pin_beats"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dur_beats", "pin_beats"];

/// What an ability of kind `dead_stroke` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeadStrokeCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<DeadStrokeCfg, RosterError> {
    Ok(DeadStrokeCfg {})
}

/// Its numbers must be finite and not negative.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(x) = ab.get("dur_beats")
        && !(x.is_finite() && x >= 0.0)
    {
        return Err(e.err(&format!("{prefix}.dur_beats"), "must be a finite number, not negative"));
    }
    if let Some(x) = ab.get("pin_beats")
        && !(x.is_finite() && x >= 0.0)
    {
        return Err(e.err(&format!("{prefix}.pin_beats"), "must be a finite number, not negative"));
    }
    Ok(())
}
