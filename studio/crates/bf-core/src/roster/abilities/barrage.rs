//! Ability kind `barrage`, EUPHONIUM's NOT A TUBA: its roster settings. `fight.py` `trigger` 2900 to 2901;
//! `start_barrage` 2721 to 2740.
//!
//! `n` shells at once, landing `flight_beats` beats later and on over `beats` beats on the 8ths: the first `aimed`
//! at the enemy's predicted spot (its motion x `lead`, jittered), the rest on a jittered grid over the whole ring.
//! A landing splashes `splash` (+ 0.6 of a ball radius): `dmg` and `knock`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `barrage`. While false the roster reads it as
/// `AbilityKind::Unsupported("barrage")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["n"], &["beats"], &["flight_beats"], &["splash"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind (`aimed` and `lead` through `ab.get`, defaults 0 and 1.0);
/// an entry holding another key loads as `Unsupported("barrage+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "n",
    "beats",
    "flight_beats",
    "aimed",
    "lead",
    "splash",
    "dmg",
    "knock",
];

/// What an ability of kind `barrage` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BarrageCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<BarrageCfg, RosterError> {
    Ok(BarrageCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: `int(ab['n'])` and
/// `int(round(2 * ab['beats']))` raise on a number that is not finite, and `math.sqrt(n)` on a negative count.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    for key in ["n", "beats"] {
        if let Some(x) = ab.get(key)
            && !(2.0 * x).is_finite()
        {
            return Err(e.err(&format!("{prefix}.{key}"), "must be a finite number"));
        }
    }
    if let Some(n) = ab.get("n")
        && n.trunc() < 0.0
    {
        return Err(e.err(&format!("{prefix}.n"), "must not be negative (a count of shells)"));
    }
    Ok(())
}
