//! Ability kind `pipe_box`, CONTRABASSOON's FIVE AND A HALF METRES: its roster settings. Made from scratch (Nov 16, 2026,
//! `docs/studio/2026-11-16-contrabassoon.md`); the engine code lives in the weapon's module (`bf-sim` `kinds::plumbing`).
//!
//! A square of tube of half-side \`half\` closes round the enemy for \`beats\` beats; the enemy bounces inside it and takes \`dmg\` a wall hit (once per \`cool\` s, default 0.2). Its owner passes through it.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `pipe_box`.
pub const PORTED: bool = true;

/// The numbers the engine reads as `ab[name]` for this kind.
pub const REQUIRED: &[&[&str]] = &[&["beats"], &["half"], &["dmg"]];

/// Every key the engine reads from an ability of this kind.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "beats", "half", "dmg", "cool"];

/// What an ability of kind `pipe_box` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PipeBoxCfg {}

/// Reads what is not a number from an ability entry of this kind.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<PipeBoxCfg, RosterError> {
    Ok(PipeBoxCfg {})
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
