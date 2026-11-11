//! The option `detune` of a `wave` ability, OBOE's ILL WIND (`fight.py` `trigger` 2845 to 2846; the hit of
//! `effects_step` 3069 to 3071, which calls `_detune` 2447 to 2451; the detuned attacker in `damage` 345 to 346).
//!
//! A stub until OBOE's port: [`PORTED`] is false, so a wave with `detune` loads as `AbilityKind::Unsupported(
//! "wave+detune")` and no fight can have it. The port fills [`DetuneCfg`] (`depth`, `dur_beats`, `weak`: what
//! `effects_step` reads as `dd_['depth']`, ...) and [`parse`], and sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{Entry, RosterError};

/// Whether the engine runs a wave's `detune`. While false a wave with one loads as `Unsupported("wave+detune")`.
pub const PORTED: bool = false;

/// A wave's `detune` table (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DetuneCfg {}

/// Reads a wave's `detune` table (one Python counts as true); called only when [`PORTED`] is true. `prefix` is the
/// ability's path, such as `abilities.low`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _detune: &Json) -> Result<DetuneCfg, RosterError> {
    Ok(DetuneCfg {})
}
