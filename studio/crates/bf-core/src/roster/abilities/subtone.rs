//! Ability kind `subtone`, TENOR SAX's SUBTONE: its roster settings. `fight.py` `trigger` 2969 to 2972; the hush's
//! end in `crowd_tick` 4592 to 4596.
//!
//! The crowd hushes for `hush_beats` beats (no poses, the pending throws dropped), then throws one volley of
//! `max(1, int(round(int(hype) * n_k)))` throws at `mult` x the damage with `reach_k` x the reach. The entry holds
//! only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `subtone`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["hush_beats"], &["n_k"], &["mult"], &["reach_k"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("subtone+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "hush_beats", "n_k", "mult", "reach_k"];

/// What an ability of kind `subtone` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubtoneCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<SubtoneCfg, RosterError> {
    Ok(SubtoneCfg {})
}

/// The kind's own check: `int(round(int(hype) * n_k))` raises when `n_k` is not a finite number.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(n_k) = ab.get("n_k")
        && !n_k.is_finite()
    {
        return Err(e.err(&join(prefix, "n_k"), format!("{n_k:e} is not a finite number")));
    }
    Ok(())
}
