//! Ability kind `spout`, UKULELE's UKE-NADO: its roster settings. `fight.py` `trigger` 2872 to 2873; `ab_x33` 578 to
//! 583 and the `spouts` of `surf_rake_step` (671 to 699); the path in `X33_SPOUT` (6220 to 6221).
//!
//! A waterspout starts 1.2 R in front of the ball toward the enemy and chases it at `speed`, turning at most `turn`
//! rad/s, bouncing off the ropes (`radius` inside them), for `life` seconds; within `radius` + R of the enemy it hits
//! every `tick` seconds (`dmg`, `knock`, spun round it), at most `max_ticks` times. The entry holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `spout`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["life"],
    &["speed"],
    &["turn"],
    &["radius"],
    &["tick"],
    &["dmg"],
    &["max_ticks"],
    &["knock"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("spout+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "life",
    "speed",
    "turn",
    "radius",
    "tick",
    "dmg",
    "max_ticks",
    "knock",
];

/// What an ability of kind `spout` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpoutCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SpoutCfg, RosterError> {
    Ok(SpoutCfg {})
}

/// The kind's own checks: a spout whose `life` is not finite and whose `max_ticks` is infinite never ends.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(life) = ab.get("life")
        && !life.is_finite()
    {
        return Err(e.err(
            &join(prefix, "life"),
            format!("{life:e} is not a finite number of seconds"),
        ));
    }
    Ok(())
}
