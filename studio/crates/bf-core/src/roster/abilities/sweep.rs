//! Ability kind `sweep`, ACOUSTIC's BIG STRUM: its roster settings. `fight.py` `trigger` 2872 to 2873; `ab_x33` 573 to
//! 577 and the sweeps of `surf_rake_step` 659 to 670.
//!
//! All six strings sweep the whole ring from the ball, square to the enemy's direction, over `beats` beats: the
//! front moves `reach` beyond the ball's surface (`R + reach * x ** 1.5`); an enemy it passes is hit once (`dmg`,
//! `knock`, along the sweep). The entry holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `sweep`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["beats"], &["reach"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("sweep+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "beats", "reach", "dmg", "knock"];

/// What an ability of kind `sweep` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SweepCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SweepCfg, RosterError> {
    Ok(SweepCfg {})
}

/// The kind's own check: the front divides by the sweep's length in the step that starts it (`tau / dur` with
/// `tau` 0: `ZeroDivisionError` for a sweep of 0 beats).
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if ab.get("beats") == Some(0.0) {
        return Err(e.err(
            &join(prefix, "beats"),
            "a sweep of no time cannot cross the ring (its front divides by its length)",
        ));
    }
    Ok(())
}
