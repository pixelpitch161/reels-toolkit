//! Ability kind `encore`, ALTO SAX's ENCORE: its roster settings. `fight.py` `trigger` 2964 to 2965; `start_encore`
//! 4501 to 4512.
//!
//! For `beats` beats one light (the weapon's `max_r` x `r_k`, 1.0 when left out) chases the enemy at `speed` x the
//! shared speed and burns `dmg` on every 16th; on every 8th `int(n_phone)` phone lights pick spots within `spread`
//! of the enemy, and one within `phone_r` of it burns `phone_dmg` (`spot_tick` 4419 to 4427, `spot_step` 4474 to
//! 4488). It holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `encore`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["beats"],
    &["dmg"],
    &["speed"],
    &["n_phone"],
    &["spread"],
    &["phone_r"],
    &["phone_dmg"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("encore+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "beats",
    "dmg",
    "speed",
    "r_k",
    "n_phone",
    "spread",
    "phone_r",
    "phone_dmg",
];

/// What an ability of kind `encore` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EncoreCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<EncoreCfg, RosterError> {
    Ok(EncoreCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
