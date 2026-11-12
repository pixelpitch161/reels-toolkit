//! Ability kind `stagedive`, TENOR SAX's STAGE DIVE: its roster settings. `fight.py` `trigger` 2973 to 2976;
//! `start_dive` 4692 to 4709, `dive_step` 4710 to 4757.
//!
//! The tenor leaps into the crowd beyond the nearest rope (`leap_beats` beats; `stand` beyond it,
//! `ab.get('stand', 0.9)`), surfs along the stands toward the enemy (`surf_beats` beats) and is fired back at
//! `speed` x the shared speed at the enemy's spot predicted `lead` x its motion (`ab.get('lead', 0.6)`): passing
//! within 2 R + `hit_r` (`ab.get('hit_r', 0.1)`) of the enemy hits it (`dmg`, `knock`), and where it lands a ring of
//! kind `dive` spreads to `radius` beyond its surface (`ring_dmg`, `ring_knock`, at `ab.get('ring_speed', 9.0)`).
//! From the leap to the landing it is airborne. The entry holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `stagedive`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind.
pub const REQUIRED: &[&[&str]] = &[
    &["leap_beats"],
    &["surf_beats"],
    &["speed"],
    &["dmg"],
    &["knock"],
    &["radius"],
    &["ring_dmg"],
    &["ring_knock"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("stagedive+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "leap_beats",
    "surf_beats",
    "speed",
    "dmg",
    "knock",
    "radius",
    "ring_dmg",
    "ring_knock",
    "stand",
    "lead",
    "hit_r",
    "ring_speed",
];

/// What an ability of kind `stagedive` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StagediveCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<StagediveCfg, RosterError> {
    Ok(StagediveCfg {})
}

/// The kind's own check: the flight's length is divided by `speed * V` at the launch (`ZeroDivisionError` in Python
/// for a speed of 0).
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if ab.get("speed") == Some(0.0) {
        return Err(e.err(
            &join(prefix, "speed"),
            "a cannonball of no speed never lands (the flight's time is its length over speed x the shared speed)",
        ));
    }
    Ok(())
}
