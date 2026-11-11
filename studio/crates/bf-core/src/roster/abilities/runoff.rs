//! Ability kind `runoff`, PIANO's 88 KEYS: its roster settings. `fight.py` `trigger` 2944 to 2945; `start_runoff` 5245
//! to 5261.
//!
//! A stub until PIANO's port: [`PORTED`] is false, so an ability of kind `runoff` loads as
//! `AbilityKind::Unsupported("runoff")` and no fight can have it. The port lists the numbers `fight.py` reads as
//! `ab[name]` in [`REQUIRED`] and every key it reads in [`ALLOWED`], fills [`RunoffCfg`] (what is not a number: lists,
//! tables), [`parse`] and [`check`], and sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `runoff`. While false the roster reads it as
/// `AbilityKind::Unsupported("runoff")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("runoff+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars"];

/// What an ability of kind `runoff` holds besides its numbers (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunoffCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<RunoffCfg, RosterError> {
    Ok(RunoffCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
