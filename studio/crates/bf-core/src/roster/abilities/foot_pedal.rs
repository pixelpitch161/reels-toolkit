//! Ability kind `foot_pedal`, OCTOBASS's FOOT PEDAL: a pedal plate is set where the enemy is; after `tele` beats it snaps up, and an enemy within `radius` ball radii of it takes `dmg` and is stunned for `stun_beats` beats (a Studio kind; the engine's `bf_sim::kinds::levers` runs it).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `foot_pedal`.
pub const PORTED: bool = true;

/// The numbers the engine reads for this kind.
pub const REQUIRED: &[&[&str]] = &[&["tele"], &["radius"], &["dmg"], &["stun_beats"]];

/// Every key an ability of this kind may hold.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "tele", "radius", "dmg", "stun_beats"];

/// What an ability of kind `foot_pedal` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FootPedalCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<FootPedalCfg, RosterError> {
    Ok(FootPedalCfg {})
}

/// Its numbers must be finite and not negative.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    for key in ALLOWED.iter().skip(3) {
        if let Some(x) = ab.get(key)
            && !(x.is_finite() && x >= 0.0)
        {
            return Err(e.err(&format!("{prefix}.{key}"), "must be a finite number, not negative"));
        }
    }
    Ok(())
}
