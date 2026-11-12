//! Ability kind `tapping`, ELECTRIC's TAPPING: its roster settings. `fight.py` `trigger` 2936 to 2938.
//!
//! For `beats` beats the second hand taps a bolt of the LIGHTNING RIFF on every 8th on top of the riff, each hitting
//! for the weapon's `dmg` times `dmg_k` (`tap_ab.get('dmg_k', 1.0)`); taps do not grow the riff. It holds only
//! numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `tapping`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["beats"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("tapping+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "beats", "dmg_k"];

/// What an ability of kind `tapping` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TappingCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<TappingCfg, RosterError> {
    Ok(TappingCfg {})
}

/// The kind's own checks on an entry's numbers: none (every number runs in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
