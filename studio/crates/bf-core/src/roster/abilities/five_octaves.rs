//! Ability kind `five_octaves`, MARIMBA's FIVE OCTAVES (a Studio kind; the engine's `bf_sim::kinds::mallets` runs it): `n` mallets leave the ball one after another over `beats` beats in a full rising turn from the enemy, each bouncing at most `bounces` times for `dmg`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `five_octaves`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["n"], &["beats"], &["speed"], &["dmg"], &["knock"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "n",
    "beats",
    "speed",
    "dmg",
    "knock",
    "bounces",
];

/// What an ability of kind `five_octaves` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FiveOctavesCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<FiveOctavesCfg, RosterError> {
    Ok(FiveOctavesCfg {})
}

/// Its numbers must be finite and not negative.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(x) = ab.get("n")
        && !(x.is_finite() && x >= 0.0)
    {
        return Err(e.err(&format!("{prefix}.n"), "must be a finite number, not negative"));
    }
    if let Some(x) = ab.get("beats")
        && !(x.is_finite() && x >= 0.0)
    {
        return Err(e.err(&format!("{prefix}.beats"), "must be a finite number, not negative"));
    }
    if let Some(x) = ab.get("speed")
        && !(x.is_finite() && x >= 0.0)
    {
        return Err(e.err(&format!("{prefix}.speed"), "must be a finite number, not negative"));
    }
    if let Some(x) = ab.get("dmg")
        && !(x.is_finite() && x >= 0.0)
    {
        return Err(e.err(&format!("{prefix}.dmg"), "must be a finite number, not negative"));
    }
    if let Some(x) = ab.get("knock")
        && !(x.is_finite() && x >= 0.0)
    {
        return Err(e.err(&format!("{prefix}.knock"), "must be a finite number, not negative"));
    }
    if let Some(x) = ab.get("bounces")
        && !(x.is_finite() && x >= 0.0)
    {
        return Err(e.err(&format!("{prefix}.bounces"), "must be a finite number, not negative"));
    }
    Ok(())
}
