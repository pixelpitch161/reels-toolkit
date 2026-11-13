//! Ability kind `lash`, SYNTH's SAWTOOTH: its roster settings. `fight.py` `trigger` 2868 to 2869; 1661 to 1712
//! (`start_lash`, `lash_angle`, `lash_step`).
//!
//! For `beats` beats, n arms (the tether's slots, at least `n_min`) of length `len` reach out of the ball around the aim
//! at the enemy (`spread` degrees apart) and sweep across `sweep` degrees in a saw-tooth (a linear sweep every
//! `period_beats`, then a snap back; arm j lags by `lag` of a period); a forward sweep across the enemy cuts (`dmg`,
//! `knock`), each arm resting `cool` seconds. IDENTITY CRISIS cannot copy it.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `lash`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["beats"],
    &["len"],
    &["spread"],
    &["sweep"],
    &["period_beats"],
    &["dmg"],
    &["knock"],
    &["cool"],
];

/// Every key `fight.py` reads from an ability of this kind (`n_min` and `lag` with `.get`, defaults 1 and 0.15); an
/// entry holding another key loads as `Unsupported("lash+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "beats",
    "n_min",
    "len",
    "spread",
    "sweep",
    "period_beats",
    "lag",
    "dmg",
    "knock",
    "cool",
];

/// What an ability of kind `lash` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LashCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<LashCfg, RosterError> {
    Ok(LashCfg {})
}

/// The kind's own checks: `lash_angle` divides by the period (`period_beats` x the beat), which Python refuses at 0.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if ab.get("period_beats") == Some(0.0) {
        return Err(e.err(&format!("{prefix}.period_beats"), "0 is not a period (the arms sweep once per period)"));
    }
    Ok(())
}
