//! Ability kind `smash`, ELECTRIC's the smash: its roster settings. `fight.py` `trigger` 2939 to 2940; 3923 to 3964
//! (`start_smash`, `splinters_step`); the smashed timer and its `rebuild` event (`run()` 6104 to 6107).
//!
//! The fighter smashes itself on the floor: a ring of kind `smash` (`speed`, `radius`, `dmg`, `knock`) and `n`
//! splinters flying out at `splinter_speed` (each hits once for `splinter_dmg`, knock `splinter_knock`, default
//! 0.4), then it lies in pieces for `rebuild_beats` beats. It holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `smash`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["rebuild_beats"],
    &["radius"],
    &["n"],
    &["speed"],
    &["dmg"],
    &["knock"],
    &["splinter_speed"],
    &["splinter_dmg"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("smash+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "rebuild_beats",
    "radius",
    "n",
    "speed",
    "dmg",
    "knock",
    "splinter_speed",
    "splinter_dmg",
    "splinter_knock",
];

/// What an ability of kind `smash` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SmashCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SmashCfg, RosterError> {
    Ok(SmashCfg {})
}

/// The kind's own checks on an entry's numbers: `int(ab['n'])` splinters, each at the angle `2 pi j / n`, so `n`
/// must be finite (Python raises on `int` of an infinity), and at most 2^53 (the engine's counts are 64-bit).
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(n) = ab.get("n")
        && (!n.is_finite() || n.abs() > 9_007_199_254_740_992.0)
    {
        return Err(e.err(
            &join(prefix, "n"),
            format!("{n:e} splinters is too many (at most 2^53)"),
        ));
    }
    Ok(())
}
