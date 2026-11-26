//! Ability kind `flash_spin`, SOUSAPHONE's HORN FLASH: its roster settings. `fight.py` `trigger` 2977 to 2979;
//! `start_flash` 4985 to 4994.
//!
//! The whole horn spins once over `beats` beats, its bell blast (`len_k` x the blast's length, `width` wide)
//! sweeping all round; it hits once (`dmg`, `knock`). The entry holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `flash_spin`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (`start_flash`): `beats`, `width`, `dmg`, `knock`.
pub const REQUIRED: &[&[&str]] = &[&["beats"], &["width"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind (`len_k` with `ab.get('len_k', 1.0)`); an entry holding
/// another key loads as `Unsupported("flash_spin+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "beats", "len_k", "width", "dmg", "knock"];

/// What an ability of kind `flash_spin` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlashSpinCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<FlashSpinCfg, RosterError> {
    Ok(FlashSpinCfg {})
}

/// The kind's own checks on an entry's numbers: none (a spin of no beats is over in its first step, as in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
