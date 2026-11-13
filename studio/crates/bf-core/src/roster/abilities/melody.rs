//! Ability kind `melody`, VIOLA's MELODY AT LAST: its roster settings. `fight.py` `trigger` 2923 to 2927; read by
//! `rests_tick` (5073 to 5076).
//!
//! For its duration (`ab_dur`: `dur` s or `dur_beats` beats, plus half a beat) the rests shrink to `int(rest)` beats
//! (`ab.get('rest', 1)`): a whole note every `int(rest) + 1` beats. It holds only numbers, so [`MelodyCfg`] is empty.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `melody`. While false the roster reads it as
/// `AbilityKind::Unsupported("melody")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["dur", "dur_beats"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("melody+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "dur", "dur_beats", "rest"];

/// What an ability of kind `melody` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MelodyCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<MelodyCfg, RosterError> {
    Ok(MelodyCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: the shorter rests'
/// `rest` makes a cycle of `int(rest) + 1` beats, which must be at least one beat.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(rest) = ab.get("rest") {
        // cyc = int(B.melody_rest) + 1; j = (kb - k0) % cyc: Python raises for a cycle of 0 (and cannot int() a NaN)
        if !rest.is_finite() || rest.trunc() < 0.0 {
            let why = "must be a finite number of at least 0 (the rests last int(rest) beats)";
            return Err(e.err(&format!("{prefix}.rest"), why));
        }
    }
    Ok(())
}
