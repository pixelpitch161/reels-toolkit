//! Ability kind `runoff`, PIANO's 88 KEYS: its roster settings. `fight.py` `trigger` 2944 to 2945; `start_runoff` 5245
//! to 5261.
//!
//! The roll runs off its spool: `int(n)` shots, from lanes spread over the whole side (`margin` in from the posts,
//! 0.25 unless given), fire at once straight across the ring at `speed`, `dmg` each; then no row fires for
//! `reload_beats` beats (0 unless given) while a new roll is threaded.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `runoff`. While false the roster reads it as
/// `AbilityKind::Unsupported("runoff")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["n"], &["speed"], &["dmg"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("runoff+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "n",
    "speed",
    "dmg",
    "margin",
    "reload_beats",
];

/// What an ability of kind `runoff` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunoffCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<RunoffCfg, RosterError> {
    Ok(RunoffCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: `n` is a finite
/// number (`int(ab['n'])`, 5249, raises on any other).
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    match ab.get("n") {
        Some(n) if !n.is_finite() => Err(e.err(
            &join(prefix, "n"),
            format!("{n} is not a finite number (the roll fires that many shots at once)"),
        )),
        _ => Ok(()),
    }
}
