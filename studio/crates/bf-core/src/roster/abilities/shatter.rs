//! Ability kind `shatter`, PICCOLO's GLASS SHATTER (a Studio kind; the engine's `bf_sim::kinds::needle` runs it): `n`
//! glass shards break off the ropes one after another over `beats` beats, each flying at the enemy at `speed`, `dmg` a
//! hit (and `knock`).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `shatter`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["n"], &["beats"], &["speed"], &["dmg"], &["knock"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "n", "beats", "speed", "dmg", "knock"];

/// What an ability of kind `shatter` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShatterCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<ShatterCfg, RosterError> {
    Ok(ShatterCfg {})
}

/// Its numbers must be finite; the count at least 1, the time and speed above 0.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    for key in ["n", "beats", "speed", "dmg", "knock"] {
        if let Some(x) = ab.get(key)
            && !(x.is_finite() && x >= 0.0)
        {
            return Err(e.err(&format!("{prefix}.{key}"), "must be a finite number, not negative"));
        }
    }
    for key in ["n", "beats", "speed"] {
        if let Some(x) = ab.get(key)
            && x <= 0.0
        {
            return Err(e.err(&format!("{prefix}.{key}"), "must be above 0"));
        }
    }
    Ok(())
}
