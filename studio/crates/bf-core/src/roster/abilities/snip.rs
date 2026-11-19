//! Ability kind `snip`, OBOE's DOUBLE REED: its roster settings. `fight.py` `trigger` 2882 to 2883; 2526 to 2567
//! (`snip_ready`, `start_snip`, `snip_step`); the gate in `abilities_step` 3037 to 3039.
//!
//! Two blades scissor out of the ball at the enemy, turning to it at `track` radians a second while they open over
//! `open16` 16ths, and snap shut: a hit when the enemy is within `reach` beyond the surface and `half` degrees of
//! the aim (`dmg`, `knock`). The ability waits until the enemy is within `ready` beyond the surface. BASS CLARINET
//! has one too, and IDENTITY CRISIS copies it.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `snip`. While false the roster reads it as `AbilityKind::Unsupported("snip")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["ready"],
    &["reach"],
    &["half"],
    &["open16"],
    &["track"],
    &["dmg"],
    &["knock"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("snip+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "ready",
    "reach",
    "half",
    "open16",
    "track",
    "dmg",
    "knock",
];

/// What an ability of kind `snip` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnipCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SnipCfg, RosterError> {
    Ok(SnipCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none (the snip
/// divides by none of its numbers and counts with none).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
