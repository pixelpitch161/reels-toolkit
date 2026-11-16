//! Ability kind `press_box`, MELLOPHONE's PRESS BOX: for `dur_beats` beats every SLAPBACK echo leaves as two waves, `spread` degrees either side of the enemy (a Studio kind; the engine's `bf_sim::kinds::slapback` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `press_box`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["dur_beats"], &["spread"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dur_beats", "spread"];

/// What an ability of kind `press_box` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PressBoxCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<PressBoxCfg, RosterError> {
    Ok(PressBoxCfg {})
}

/// Its numbers must be finite and not negative.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    for key in ["dur_beats", "spread"] {
        if let Some(x) = ab.get(key)
            && !(x.is_finite() && x >= 0.0)
        {
            return Err(e.err(&format!("{prefix}.{key}"), "must be a finite number, not negative"));
        }
    }
    Ok(())
}
