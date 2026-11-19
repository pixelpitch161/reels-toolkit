//! Ability kind `clamp`, ACCORDION's FULL BELLOWS: its roster settings. `fight.py` `trigger` 2876 to 2877; 1905 to 1926
//! (`clamp_start`, `clamps_step`). The bellows stretch across the ring for `tele` beats, then clamp for `dmg` (knock
//! `knock`, toward the accordion). It holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `clamp`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["tele"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("clamp+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "tele", "dmg", "knock"];

/// What an ability of kind `clamp` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClampCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<ClampCfg, RosterError> {
    Ok(ClampCfg {})
}

/// The kind's own checks on an entry's numbers: none beyond [`REQUIRED`].
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
