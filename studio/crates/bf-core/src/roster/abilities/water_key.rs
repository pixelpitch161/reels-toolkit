//! Ability kind `water_key`, CONTRABASSOON's WATER KEY: its roster settings. Made from scratch (Nov 16, 2026,
//! `docs/studio/2026-11-16-contrabassoon.md`); the engine code lives in the weapon's module (`bf-sim` `kinds::plumbing`).
//!
//! For \`beats\` beats (default 1) every pipe end and the fighter's own ball spray; an enemy within \`reach\` of a spray takes \`dmg\` once per spray and is knocked away (\`knock\`).

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `water_key`.
pub const PORTED: bool = true;

/// The numbers the engine reads as `ab[name]` for this kind.
pub const REQUIRED: &[&[&str]] = &[&["dmg"], &["reach"]];

/// Every key the engine reads from an ability of this kind.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "beats", "reach", "dmg", "knock"];

/// What an ability of kind `water_key` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WaterKeyCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<WaterKeyCfg, RosterError> {
    Ok(WaterKeyCfg {})
}

/// The kind's own checks on an entry's numbers: every number is finite and not below 0.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    for (name, v) in &ab.params {
        if !v.is_finite() || *v < 0.0 {
            return Err(e.err(&format!("{prefix}.{name}"), "must be a finite number of at least 0"));
        }
    }
    Ok(())
}
