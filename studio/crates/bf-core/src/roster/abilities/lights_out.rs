//! Ability kind `lights_out`, BASS CLARINET's LIGHTS OUT: its roster settings. `fight.py` `trigger` 2895 to 2896; 2767
//! to 2800 (`start_lights_out`, `lights_out_step`).
//!
//! For `dur` s or `dur_beats` beats (`ab_dur`) the fighter is sunk in shadow: it cannot be hit, clashes with
//! nothing and starts no attack, and it homes on the enemy, turning at most `turn` radians a second with a boost of
//! `home` - 1. Then it erupts where it is: `dmg` and `knock` to an enemy within `radius` (plus 0.6 of a ball radius).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `lights_out`. While false the roster reads it as
/// `AbilityKind::Unsupported("lights_out")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["dur", "dur_beats"],
    &["home"],
    &["turn"],
    &["radius"],
    &["dmg"],
    &["knock"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("lights_out+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "dur",
    "dur_beats",
    "home",
    "turn",
    "radius",
    "dmg",
    "knock",
];

/// What an ability of kind `lights_out` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LightsOutCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<LightsOutCfg, RosterError> {
    Ok(LightsOutCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
