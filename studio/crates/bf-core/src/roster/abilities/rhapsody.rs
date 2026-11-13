//! Ability kind `rhapsody`, CLARINET's RHAPSODY: its roster settings. `fight.py` `trigger` 2930 to 2932 (the chain runs
//! in `jump_tick`).
//!
//! `n` jumps `step16` 16ths apart, zig-zagging `zig` degrees either side through the enemy as it is, landing `land`
//! past it, each at most `reach` long and each a cut (`dmg`, `knock`, within a ball radius plus `cut`: the weapon's
//! `cut` when the entry has none). It holds only numbers, so [`RhapsodyCfg`] is empty.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `rhapsody`. While false the roster reads it as
/// `AbilityKind::Unsupported("rhapsody")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[
    &["n"],
    &["step16"],
    &["zig"],
    &["land"],
    &["reach"],
    &["dmg"],
    &["knock"],
];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("rhapsody+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "n",
    "step16",
    "zig",
    "land",
    "reach",
    "dmg",
    "knock",
    "cut",
];

/// What an ability of kind `rhapsody` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RhapsodyCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<RhapsodyCfg, RosterError> {
    Ok(RhapsodyCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none (any number
/// runs, as in Python: a chain of 0 or fewer jumps makes one and ends; a step of 0 or less jumps on every 16th).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::roster::{AbilityKind, FighterCfg};

    fn clarinet() -> Json {
        let roster: Json = serde_json::from_str(include_str!("../../../../bf-content/data/roster.json")).unwrap();
        let list = roster["fighters"].as_array().unwrap().clone();
        list.into_iter().find(|f| f["name"] == "CLARINET").unwrap()
    }

    #[test]
    fn rhapsody_is_typed_and_needs_its_numbers() {
        let cfg = FighterCfg::from_json(&clarinet()).unwrap();
        assert_eq!(cfg.low.kind, AbilityKind::Rhapsody);
        assert_eq!(cfg.low.name, "RHAPSODY");
        let got: Vec<f64> = ["n", "step16", "zig", "land", "reach", "dmg", "knock", "cut"]
            .iter()
            .map(|k| cfg.low.get(k).unwrap())
            .collect();
        assert_eq!(got, [5.0, 2.0, 35.0, 1.4, 5.5, 3.2, 0.8, 0.25]);
        for k in ["n", "step16", "zig", "land", "reach", "dmg", "knock"] {
            let mut c = clarinet();
            c["abilities"]["low"].as_object_mut().unwrap().remove(k);
            let err = FighterCfg::from_json(&c).unwrap_err();
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str(), err.message.as_str()),
                ("CLARINET", &*format!("abilities.low.{k}"), "missing"),
                "{k}"
            );
        }
        // `ab.get('cut', w['cut'])`: the entry may leave `cut` out
        let mut c = clarinet();
        c["abilities"]["low"].as_object_mut().unwrap().remove("cut");
        let cfg = FighterCfg::from_json(&c).unwrap();
        assert_eq!(
            (cfg.low.kind.clone(), cfg.low.get("cut")),
            (AbilityKind::Rhapsody, None)
        );
        let mut c = clarinet();
        c["abilities"]["low"]["curve"] = json!(1.0);
        let cfg = FighterCfg::from_json(&c).unwrap();
        assert_eq!(cfg.low.kind, AbilityKind::Unsupported("rhapsody+curve".into()));
    }
}
