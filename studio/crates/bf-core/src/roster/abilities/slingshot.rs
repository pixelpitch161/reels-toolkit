//! Ability kind `slingshot`, BASS GUITAR's SLINGSHOT: two strings tie to the corner posts behind the ball; it draws back `back` ball radii over `pull_beats` beats, then fires itself at the enemy at `speed` x the shared speed up to `span` ball radii and stops dead in it (`dmg`, knock `knock`) (a Studio kind; the engine's `bf_sim::kinds::pick` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `slingshot`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["pull_beats"], &["back"], &["speed"], &["span"], &["dmg"], &["knock"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "pull_beats",
    "back",
    "speed",
    "span",
    "dmg",
    "knock",
];

/// What an ability of kind `slingshot` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlingshotCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SlingshotCfg, RosterError> {
    Ok(SlingshotCfg {})
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
