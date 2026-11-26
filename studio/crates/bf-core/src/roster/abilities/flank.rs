//! Ability kind `flank`, FLUGELHORN's FLANKING CALL (a from-scratch fighter, 2026-11-16; no Python kit reference; design
//! `docs/studio/2026-11-16-flugelhorn-design.md`): its roster settings. The engine side is in
//! `bf-sim/src/kinds/wingbeat.rs`.
//!
//! For `dur` s (or `dur_beats` beats) the ball swoops round to the enemy's side, steered toward a point `side` units beside it with a speed boost of `boost`; once within `reach` of the two surfaces it claps from there (`dmg`, knock `knock` sideways) and the swoop ends.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `flank`.
pub const PORTED: bool = true;

/// The numbers the engine reads as `ab[name]` for this kind.
pub const REQUIRED: &[&[&str]] = &[
    &["dur", "dur_beats"],
    &["side"],
    &["boost"],
    &["reach"],
    &["dmg"],
    &["knock"],
];

/// Every key the engine reads from an ability of this kind.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "dur",
    "dur_beats",
    "side",
    "boost",
    "reach",
    "dmg",
    "knock",
];

/// What an ability of kind `flank` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlankCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<FlankCfg, RosterError> {
    Ok(FlankCfg {})
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
