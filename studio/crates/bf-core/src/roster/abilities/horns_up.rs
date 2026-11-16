//! Ability kind `horns_up`, MELLOPHONE's HORNS UP: `n` fast straight stabs at the enemy on 16ths (`speed`, `dmg`, knock `knock`), no echoes; the last stuns for `stun_beats` beats (a Studio kind; the engine's `bf_sim::kinds::slapback` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `horns_up`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["n"], &["speed"], &["dmg"], &["knock"], &["stun_beats"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "n", "speed", "dmg", "knock", "stun_beats"];

/// What an ability of kind `horns_up` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HornsUpCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<HornsUpCfg, RosterError> {
    Ok(HornsUpCfg {})
}

/// Its numbers must be finite and not negative.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    for key in ["n", "speed", "dmg", "knock", "stun_beats"] {
        if let Some(x) = ab.get(key)
            && !(x.is_finite() && x >= 0.0)
        {
            return Err(e.err(&format!("{prefix}.{key}"), "must be a finite number, not negative"));
        }
    }
    Ok(())
}
