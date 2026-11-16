//! Ability kind `stoptime`, CORNET's STOP-TIME (a from-scratch fighter, 2026-11-16; no Python kit reference): its
//! roster settings. The engine side is in `bf-sim/src/kinds/plunger.rs`.
//!
//! `n` beats of stop-time from the next beat. On each beat but the last the enemy freezes for `freeze` of a beat (a
//! stun) and the fighter dashes at it with a speed boost of `boost`; a dash that meets the enemy within the beat hits
//! once for `dmg` with `knock`. On the last beat the band comes back in: a burst ring (`speed`, `radius`) with
//! `dmg_break` and `knock_break`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `stoptime`.
pub const PORTED: bool = true;

/// The numbers the engine reads as `ab[name]` for this kind.
pub const REQUIRED: &[&[&str]] = &[
    &["n"],
    &["freeze"],
    &["boost"],
    &["dmg"],
    &["knock"],
    &["speed"],
    &["radius"],
    &["dmg_break"],
    &["knock_break"],
];

/// Every key the engine reads from an ability of this kind.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "n",
    "freeze",
    "boost",
    "dmg",
    "knock",
    "speed",
    "radius",
    "dmg_break",
    "knock_break",
];

/// What an ability of kind `stoptime` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StoptimeCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<StoptimeCfg, RosterError> {
    Ok(StoptimeCfg {})
}

/// The kind's own checks: `n` is a whole number of beats, at least 2.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    let n = ab.get("n").unwrap_or(0.0);
    if !(2.0..=64.0).contains(&n) || n.fract() != 0.0 {
        return Err(e.err(&format!("{prefix}.n"), format!("{n} beats is not a whole number from 2 to 64")));
    }
    Ok(())
}
