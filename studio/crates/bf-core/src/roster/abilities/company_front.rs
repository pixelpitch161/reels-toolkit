//! Ability kind `company_front`, SOUSAPHONE's COMPANY FRONT: its roster settings. `fight.py` `trigger` 2980 to 2982;
//! `start_front` 4995 to 5005.
//!
//! A wall marches across the whole ring, one step a beat for `int(beats)` beats; the enemy it meets (within half of
//! `thick` of the wall) takes `dmg` and is knocked on ahead (`knock`). The entry holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `company_front`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (`start_front`, `drill_step`): `beats`, `dmg`, `knock`.
pub const REQUIRED: &[&[&str]] = &[&["beats"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind (`thick` with `ab.get('thick', 0.35)`); an entry holding
/// another key loads as `Unsupported("company_front+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "beats", "thick", "dmg", "knock"];

/// What an ability of kind `company_front` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompanyFrontCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<CompanyFrontCfg, RosterError> {
    Ok(CompanyFrontCfg {})
}

/// The kind's own check: the wall takes `int(beats)` steps, at least 1 (with none `drill_step` divides by zero in the
/// wall's first step).
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(beats) = ab.get("beats")
        && (beats as i64) < 1
    {
        return Err(e.err(
            &join(prefix, "beats"),
            format!("{beats} is less than 1 (the wall marches at least one beat)"),
        ));
    }
    Ok(())
}
