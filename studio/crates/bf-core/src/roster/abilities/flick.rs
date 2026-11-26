//! Ability kind `flick`, HORN's FINAL FLICK: its roster settings. `fight.py` `trigger` 2880 to 2881; 2282 to 2337
//! (`FLICK_STATES`, `start_flick`, `flick_step`).
//!
//! The entry holds only numbers (in `AbilityCfg::params`): `reach`, `dmg`, `boost` and `dmg_rope` are read as
//! `ab[name]`; `wind_beats` (1), `glove_r` (0.4), `rope_t` (0.9) and `show` (0.6) with `ab.get(name, default)`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `flick`. While false the roster reads it as
/// `AbilityKind::Unsupported("flick")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["reach"], &["dmg"], &["boost"], &["dmg_rope"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("flick+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "wind_beats",
    "reach",
    "glove_r",
    "dmg",
    "boost",
    "rope_t",
    "dmg_rope",
    "show",
];

/// What an ability of kind `flick` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlickCfg {}

/// Reads what is not a number from an ability entry of this kind, after [`REQUIRED`] and [`ALLOWED`] are checked:
/// nothing. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<FlickCfg, RosterError> {
    Ok(FlickCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none (any number
/// runs, as in Python).
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::Value as Json;

    use crate::roster::{AbilityKind, AbilitySettings, FighterCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn horn() -> FighterCfg {
        let all = parse_roster(ROSTER).expect("the roster loads");
        all.into_iter().find(|f| f.name == "HORN").expect("in the roster")
    }

    /// The HORN's entry with `key` of FINAL FLICK removed (or set to `value`).
    fn low_with(key: &str, value: Option<Json>) -> Result<FighterCfg, crate::roster::RosterError> {
        let mut raw = horn().raw;
        let ab = raw["abilities"]["low"].as_object_mut().expect("a table");
        match value {
            Some(v) => _ = ab.insert(key.into(), v),
            None => _ = ab.remove(key),
        }
        FighterCfg::from_json(&raw)
    }

    #[test]
    fn final_flick_is_a_flick_with_its_roster_numbers() {
        let f = horn();
        assert_eq!((f.low.name.as_str(), &f.low.kind), ("FINAL FLICK", &AbilityKind::Flick));
        assert!(matches!(f.low.settings, AbilitySettings::Flick(_)));
        for (key, value) in [
            ("wind_beats", 1.0),
            ("reach", 2.4),
            ("glove_r", 0.4),
            ("dmg", 14.0),
            ("boost", 2.6),
            ("rope_t", 0.9),
            ("dmg_rope", 10.0),
        ] {
            assert_eq!(f.low.get(key), Some(value), "{key}");
        }
        assert_eq!(f.low.get("show"), None);
    }

    #[test]
    fn a_flick_needs_its_reach_damages_and_boost_and_nothing_else() {
        for key in ["reach", "dmg", "boost", "dmg_rope"] {
            assert_eq!(
                low_with(key, None).unwrap_err().to_string(),
                format!("roster: HORN: abilities.low.{key}: missing")
            );
        }
        // the options have defaults in fight.py: an entry without them loads
        for key in ["wind_beats", "glove_r", "rope_t"] {
            let f = low_with(key, None).expect("loads");
            assert_eq!((&f.low.kind, f.low.get(key)), (&AbilityKind::Flick, None), "{key}");
        }
        assert_eq!(
            low_with("show", Some(Json::from(0.8))).expect("loads").low.get("show"),
            Some(0.8)
        );
        // a key fight.py does not read for a flick: not a flick the engine runs
        let f = low_with("radius", Some(Json::from(2.0))).expect("loads");
        assert_eq!(f.low.kind, AbilityKind::Unsupported("flick+radius".into()));
    }
}
