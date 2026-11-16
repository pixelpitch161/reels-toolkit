//! Ability kind `take_wing`, FLUGELHORN's TAKE WING (a from-scratch fighter, 2026-11-16; no Python kit reference; design
//! `docs/studio/2026-11-16-flugelhorn-design.md`): its roster settings. The engine side is in
//! `bf-sim/src/kinds/wingbeat.rs`.
//!
//! The wings spread across the ring for `tele` beats, then one giant clap hits an enemy within `radius` of the ball (`dmg`, knock `knock`), and the ball glides `mult` times as fast for `dur` s (or `dur_beats` beats).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `take_wing`.
pub const PORTED: bool = true;

/// The numbers the engine reads as `ab[name]` for this kind.
pub const REQUIRED: &[&[&str]] = &[
    &["tele"],
    &["radius"],
    &["dmg"],
    &["knock"],
    &["mult"],
    &["dur", "dur_beats"],
];

/// Every key the engine reads from an ability of this kind.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "tele",
    "radius",
    "dmg",
    "knock",
    "mult",
    "dur",
    "dur_beats",
];

/// What an ability of kind `take_wing` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TakeWingCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<TakeWingCfg, RosterError> {
    Ok(TakeWingCfg {})
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
