//! Ability kind `heartbeat`, BASS DRUM's HEARTBEAT: its roster settings. `fight.py` `trigger` 2983 to 2984; `_thump`,
//! `start_heartbeat` 5440 to 5450.
//!
//! Two shockwaves, lub-dub: one at once and one `gap16` 16ths later (`int(ab.get('gap16', 2))`), each spreading at
//! `speed` to `radius`, hitting once for `dmg` and `knock`. The entry holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `heartbeat`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (`_thump`): `speed`, `radius`, `dmg`, `knock`.
pub const REQUIRED: &[&[&str]] = &[&["speed"], &["radius"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind (`gap16` with `ab.get('gap16', 2)`); an entry holding
/// another key loads as `Unsupported("heartbeat+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "speed", "radius", "dmg", "knock", "gap16"];

/// What an ability of kind `heartbeat` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeartbeatCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<HeartbeatCfg, RosterError> {
    Ok(HeartbeatCfg {})
}

/// The kind's own check: `int(ab.get('gap16', 2))` raises for a number that is not finite.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(gap) = ab.get("gap16")
        && !gap.is_finite()
    {
        return Err(e.err(&join(prefix, "gap16"), format!("{gap:e} is not a finite number")));
    }
    Ok(())
}
