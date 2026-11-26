//! Ability kind `copy`, EUPHONIUM's IDENTITY CRISIS: its roster settings. `fight.py` `trigger` 2893 to 2894; 2801 to
//! 2822 (`COPYABLE`, `copy_source`, `start_copy`); the gate in `abilities_step` 3046 to 3049; `trigger`'s quiet path
//! 2830 to 2840.
//!
//! The fighter fires the enemy's last ability back as its own (same kind and numbers) when its kind is one of
//! `COPYABLE`, else one round of the enemy's signature. The entry has no numbers of its own beyond an unlock's
//! `every_bars`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `copy`. While false the roster reads it as `AbilityKind::Unsupported("copy")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind: none.
pub const REQUIRED: &[&[&str]] = &[];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("copy+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars"];

/// What an ability of kind `copy` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CopyCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<CopyCfg, RosterError> {
    Ok(CopyCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}
