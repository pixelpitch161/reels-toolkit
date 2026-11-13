//! Ability kind `arc`, PIANO's GLISSANDO: its roster settings. `fight.py` `trigger` 2852 to 2858; the arcs of
//! `effects_step` 3099 to 3113.
//!
//! Over `beats` beats the ball throws `n` key shots, one after another, in a fan `sweep` degrees wide centred on
//! where the enemy stood when the ability fired; each flies at `speed` and does `dmg`.
//!
//! An arc with a `curve` (the curving volleys of an older kind) is not this kind: it loads as
//! `Unsupported("arc+curve")`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, join};

/// Whether the engine runs ability kind `arc`. While false the roster reads it as `AbilityKind::Unsupported("arc")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["beats"], &["sweep"], &["n"], &["speed"], &["dmg"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("arc+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "beats", "sweep", "n", "speed", "dmg"];

/// What an ability of kind `arc` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArcCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<ArcCfg, RosterError> {
    Ok(ArcCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: `n` is a finite
/// number (`while Ar['emitted'] < n`, 3101: with an endless `n` the Python fight never leaves the first arc step).
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    match ab.get("n") {
        Some(n) if !n.is_finite() => Err(e.err(
            &join(prefix, "n"),
            format!("{n} is not a finite number (an arc throws that many shots)"),
        )),
        _ => Ok(()),
    }
}
