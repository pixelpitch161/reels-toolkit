//! Ability kind `whip`, ACOUSTIC's FINGERPICK: its roster settings. `fight.py` `trigger` 2872 to 2873; `ab_x33` 566 to
//! 572 and the whips of `surf_rake_step` 630 to 658.
//!
//! `n` single strings flick out like whips over `beats` beats (the first at once, the last at the end), each aimed
//! at the enemy where it is then: it grows to `len` beyond the ball's surface in `t_ext` seconds
//! (`ab.get('t_ext', 0.07)`) and snaps back as fast; touching the enemy it hits once (`dmg`, `knock`). The entry
//! holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `whip`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["n"], &["beats"], &["len"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("whip+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "n",
    "beats",
    "len",
    "dmg",
    "knock",
    "t_ext",
];

/// What an ability of kind `whip` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WhipCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<WhipCfg, RosterError> {
    Ok(WhipCfg {})
}

/// The kind's own checks: `while Wp['emitted'] < Wp['n']` never ends in one step for an infinite count, and a
/// whip's length divides by `t_ext` in the step that emits it (`tau / t_ext` with `tau` 0: `ZeroDivisionError` for a
/// `t_ext` of 0).
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(n) = ab.get("n")
        && n.is_infinite()
    {
        return Err(e.err(&join(prefix, "n"), format!("{n:e} is not a finite number of strings")));
    }
    if ab.get("t_ext").unwrap_or(0.07) == 0.0 {
        return Err(e.err(
            &join(prefix, "t_ext"),
            "a whip of no time cannot reach out (its length divides by t_ext)",
        ));
    }
    Ok(())
}
