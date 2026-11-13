//! The option `detune` of a `wave` ability, OBOE's ILL WIND (`fight.py` `trigger` 2845 to 2846; the hit of
//! `effects_step` 3069 to 3071, which calls `_detune` 2447 to 2451; the detuned attacker in `damage` 345 to 346).
//!
//! A ring with the option puts the enemy it lands on out of tune: `depth` deep for `dur_beats` beats, its hits
//! `weak` x depth weaker.

use serde_json::Value as Json;

use crate::roster::{Entry, RosterError};

/// Whether the engine runs a wave's `detune`. While false a wave with one loads as `Unsupported("wave+detune")`.
pub const PORTED: bool = true;

/// A wave's `detune` table: what `effects_step` reads as `dd_['depth']`, `dd_['dur_beats']`, `dd_['weak']`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DetuneCfg {
    /// How deep out of tune the enemy goes.
    pub depth: f64,
    /// For how many beats.
    pub dur_beats: f64,
    /// How much weaker its hits land, per unit of depth.
    pub weak: f64,
}

/// Reads a wave's `detune` table (one Python counts as true); called only when [`PORTED`] is true. `prefix` is the
/// ability's path, such as `abilities.low`. Each of the three numbers is read as `dd_[name]` when the ring lands, so
/// a table without one is refused here (Python raises `KeyError` mid-fight).
pub(crate) fn parse(e: &Entry, prefix: &str, detune: &Json) -> Result<DetuneCfg, RosterError> {
    let p = format!("{prefix}.detune");
    if !detune.is_object() {
        return Err(e.err(&p, "must be a table with depth, dur_beats and weak"));
    }
    e.only(detune, &p, "a wave's detune", &["depth", "dur_beats", "weak"])?;
    Ok(DetuneCfg {
        depth: e.f64(detune, &p, "depth")?,
        dur_beats: e.f64(detune, &p, "dur_beats")?,
        weak: e.f64(detune, &p, "weak")?,
    })
}
