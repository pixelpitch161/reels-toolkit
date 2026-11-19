//! Ability kind `stadium_echo`, MELLOPHONE's STADIUM ECHO: one giant wave (`half_w`, `speed`, `dmg`, `dmg_step` a leg, knock `knock`) that passes through the enemy and echoes off every rope after `delay` beats until `beats` beats are up (a Studio kind; the engine's `bf_sim::kinds::slapback` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `stadium_echo`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[
    &["beats"],
    &["speed"],
    &["half_w"],
    &["delay"],
    &["dmg"],
    &["dmg_step"],
    &["knock"],
];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "beats",
    "speed",
    "half_w",
    "delay",
    "dmg",
    "dmg_step",
    "knock",
];

/// What an ability of kind `stadium_echo` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StadiumEchoCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<StadiumEchoCfg, RosterError> {
    Ok(StadiumEchoCfg {})
}

/// Its numbers must be finite and not negative.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    for key in ["beats", "speed", "half_w", "delay", "dmg", "dmg_step", "knock"] {
        if let Some(x) = ab.get(key)
            && !(x.is_finite() && x >= 0.0)
        {
            return Err(e.err(&format!("{prefix}.{key}"), "must be a finite number, not negative"));
        }
    }
    Ok(())
}
