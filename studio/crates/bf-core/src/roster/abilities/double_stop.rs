//! Ability kind `double_stop`, VIOLA's DOUBLE STOP: its roster settings. `fight.py` `trigger` 2928 to 2929; 5108 to
//! 5117 (`start_double_stop`, in the `rests` kind's code).
//!
//! Two whole notes at once instead of this beat's rest: one at the enemy, one where it is heading (`lead2` x its
//! motion over the note's travel, `ab.get('lead2', 1.6)`). It holds only numbers, so [`DoubleStopCfg`] is empty.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `double_stop`. While false the roster reads it as
/// `AbilityKind::Unsupported("double_stop")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("double_stop+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "lead2"];

/// What an ability of kind `double_stop` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DoubleStopCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<DoubleStopCfg, RosterError> {
    Ok(DoubleStopCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none (any `lead2`
/// runs, as in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
