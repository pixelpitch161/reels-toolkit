//! Ability kind `full_set`, TIMPANI's FULL SET (a Studio kind; the engine's `bf_sim::kinds::kettles` runs it): two more kettles of radius `r_mid` (the weapon's) join the set; the hops go round all four and on every beat one kettle the timpani is not on booms by itself, in turn.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `full_set`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "beat_every"];

/// What an ability of kind `full_set` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FullSetCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<FullSetCfg, RosterError> {
    Ok(FullSetCfg {})
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
