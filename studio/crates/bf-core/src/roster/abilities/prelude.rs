//! Ability kind `prelude`, CELLO's PRELUDE: its roster settings. `fight.py` `trigger` 2948 to 2950; `prelude_step` 4220
//! to 4243.
//!
//! For `beats` beats from the trigger, on every 16th (`int(round(beats * 4))` of them), a note-wave ring leaves the
//! fighter at `speed`, out to `pattern[j % len]` beyond its surface, with `dmg` and `knock`. Besides its numbers it
//! holds `pattern`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, AbilitySettings, Entry, RosterError, join};

/// Whether the engine runs ability kind `prelude`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind. An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["beats"], &["speed"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("prelude+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "beats",
    "speed",
    "pattern",
    "dmg",
    "knock",
];

/// What an ability of kind `prelude` holds besides its numbers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreludeCfg {
    /// `pattern`: each ring's reach beyond the fighter's surface, in turn (`pat[j % len(pat)]`).
    pub pattern: Vec<f64>,
}

/// Reads what is not a number from an ability entry of this kind, after [`REQUIRED`] and [`ALLOWED`] are checked:
/// `pattern`, a list of at least one number (`fight.py` reads `ab['pattern']` and takes `j % len(pat)`, which
/// raises on an empty list). `prefix` is the field's path, such as `abilities.unlock2`.
pub(crate) fn parse(e: &Entry, prefix: &str, ab: &Json) -> Result<PreludeCfg, RosterError> {
    let field = join(prefix, "pattern");
    let list = e
        .get(ab, prefix, "pattern")?
        .as_array()
        .filter(|l| !l.is_empty())
        .ok_or_else(|| e.err(&field, "must be a list of at least one number"))?;
    let pattern = list
        .iter()
        .map(|v| v.as_f64().ok_or_else(|| e.err(&field, "must be a list of numbers")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PreludeCfg { pattern })
}

/// The kind's own checks on an entry, at load time and again in `FighterCfg::check`: the settings are a prelude's
/// with a pattern, and `int(round(beats * 4))` is a number Python's `int` takes (finite).
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    match &ab.settings {
        AbilitySettings::Prelude(p) if !p.pattern.is_empty() => {}
        _ => return Err(e.err(&join(prefix, "pattern"), "missing")),
    }
    if let Some(beats) = ab.get("beats") {
        if !(beats * 4.0).is_finite() || (beats * 4.0).abs() > 9_007_199_254_740_992.0 {
            return Err(e.err(
                &join(prefix, "beats"),
                format!("{beats:e} beats is too long (4 x beats must be a finite number up to 2^53)"),
            ));
        }
    }
    Ok(())
}

/// `ab['pattern']` of a prelude ability.
///
/// # Panics
/// When the ability is not a prelude (`FighterCfg::check` refuses one without its settings).
pub fn pattern(ab: &AbilityCfg) -> &[f64] {
    match &ab.settings {
        AbilitySettings::Prelude(p) => &p.pattern,
        other => panic!("the ability {} has no pattern: {other:?}", ab.name),
    }
}
