//! Ability kind `rewind`, TURNTABLES's REWIND: its roster settings. `fight.py` `trigger` 2866 to 2867; 1927 to 1998
//! (`_rewind_ab`, `_rewind_then`, `rewind_ready`, `start_rewind`, `rewind_step`); the gate in `abilities_step` 3035 to
//! 3036; a rewinding ball is intangible (`rw_left`).
//!
//! The ball undoes the damage it took in the last `window` s (HP back to its recorded value then) and zips back along
//! its own recorded path to where it was then, in `zip_beats` beats, intangible while it zips. It waits until that
//! would undo at least `min_undo` HP (`ab.get('min_undo', 0.0)`).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `rewind`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["window"], &["zip_beats"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("rewind+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "window", "zip_beats", "min_undo"];

/// What an ability of kind `rewind` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RewindCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<RewindCfg, RosterError> {
    Ok(RewindCfg {})
}

/// The kind's own checks on an entry's numbers: the history is `int(round(window / DT)) + 3` deep and the zip divides
/// by its length, so `window` must be a finite number of at least 0 and `zip_beats` above 0.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    let window = ab.get("window").unwrap_or(0.0);
    if !(window.is_finite() && (0.0..=60.0).contains(&window)) {
        return Err(e.err(&format!("{prefix}.window"), "a rewind's window is 0 to 60 s"));
    }
    let zip = ab.get("zip_beats").unwrap_or(0.0);
    if !(zip.is_finite() && zip > 0.0) {
        return Err(e.err(&format!("{prefix}.zip_beats"), "a rewind's zip takes more than 0 beats"));
    }
    Ok(())
}
