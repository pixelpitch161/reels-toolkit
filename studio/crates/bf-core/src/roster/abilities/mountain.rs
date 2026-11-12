//! Ability kind `mountain`, BASSOON's MOUNTAIN KING: its roster settings. `fight.py` `trigger` 2917 to 2918;
//! `start_mountain` 3423 to 3428, and the dance in `tiptoe_tick` (3374 to 3391).
//!
//! Its numbers: `step` (degrees round the enemy per hop), `gap` (beyond contact), `reach` (the longest hop), `dmg`
//! and `knock` (each landing's stomp). Besides them it holds `seq16`, the 16ths between its hops' launches.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, AbilitySettings, Entry, RosterError, join};

/// Whether the engine runs ability kind `mountain`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["step"], &["gap"], &["reach"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("mountain+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "seq16",
    "step",
    "gap",
    "reach",
    "dmg",
    "knock",
];

/// What an ability of kind `mountain` holds besides its numbers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MountainCfg {
    /// `seq16`: for each hop of the dance, the 16ths until the next one is launched (`int(ab['seq16'][i])`).
    pub seq16: Vec<i64>,
}

/// Reads what is not a number from an ability entry of this kind, after [`REQUIRED`] and [`ALLOWED`] are checked:
/// `seq16`, a list of whole numbers (`fight.py` reads `ab['seq16']`, so it must be there). `prefix` is the field's
/// path, such as `abilities.low`.
pub(crate) fn parse(e: &Entry, prefix: &str, ab: &Json) -> Result<MountainCfg, RosterError> {
    let field = join(prefix, "seq16");
    let list = e
        .get(ab, prefix, "seq16")?
        .as_array()
        .ok_or_else(|| e.err(&field, "must be a list of whole numbers"))?;
    let seq16 = list.iter().map(|v| e.int(v, &field)).collect::<Result<Vec<_>, _>>()?;
    Ok(MountainCfg { seq16 })
}

/// The kind's own checks on an entry, at load time and again in `FighterCfg::check`: the settings are a mountain's.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    match &ab.settings {
        AbilitySettings::Mountain(_) => Ok(()),
        _ => Err(e.err(&join(prefix, "seq16"), "missing")),
    }
}

/// `ab['seq16']` of a mountain ability.
///
/// # Panics
/// When the ability is not a mountain (`FighterCfg::check` refuses one without its settings).
pub fn seq16(ab: &AbilityCfg) -> &[i64] {
    match &ab.settings {
        AbilitySettings::Mountain(m) => &m.seq16,
        other => panic!("the ability {} has no seq16: {other:?}", ab.name),
    }
}
