//! Ability kind `cannons`, BASS DRUM's 1812 OVERTURE: its roster settings. `fight.py` `trigger` 2987 to 2988;
//! `start_cannons`, `_cannon` 5477 to 5494.
//!
//! `int(n)` cannon booms, one every `gap_beats` beats (`ab.get('gap_beats', 1)`), each aimed at the enemy's spot
//! `tele` beats ahead and landing then like a slam (`radius`, `dmg`, `knock`; its hit is a `cannon`). The entry holds
//! only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `cannons`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (`_cannon`): `n`, `tele`, `radius`, `dmg`, `knock`.
pub const REQUIRED: &[&[&str]] = &[&["n"], &["tele"], &["radius"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind (`gap_beats` with `ab.get('gap_beats', 1)`); an entry
/// holding another key loads as `Unsupported("cannons+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "n",
    "gap_beats",
    "tele",
    "radius",
    "dmg",
    "knock",
];

/// What an ability of kind `cannons` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CannonsCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<CannonsCfg, RosterError> {
    Ok(CannonsCfg {})
}

/// The kind's own check: `int(ab['n'])` raises for a number that is not finite. (A count below 1 fires one cannon,
/// as in Python: the count is tested after each shot.)
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(n) = ab.get("n")
        && !n.is_finite()
    {
        return Err(e.err(&join(prefix, "n"), format!("{n:e} is not a finite number")));
    }
    Ok(())
}
