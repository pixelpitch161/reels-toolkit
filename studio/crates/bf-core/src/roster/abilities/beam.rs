//! Ability kind `beam`, TRUMPET's HIGH C: its roster settings. `fight.py` `trigger` 2870 to 2871; `ab_beam_charge` 2990
//! to 2999, `ab_beam_fire` 3001 to 3010, `abilities_step` 3013 to 3016, the ability beams of `beams_step` 857 to 897.
//!
//! The entry holds only numbers (in `AbilityCfg::params`): `len`, `width` and `dmg` are read as `ab[name]`;
//! `charge_beats` (1), `lead` (0.7), `knock` (1.5) and `hold` (0.3) with `ab.get(name, default)`.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `beam`. While false the roster reads it as `AbilityKind::Unsupported("beam")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["len"], &["width"], &["dmg"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("beam+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "charge_beats",
    "lead",
    "len",
    "width",
    "dmg",
    "knock",
    "hold",
];

/// What an ability of kind `beam` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BeamCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<BeamCfg, RosterError> {
    Ok(BeamCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`.
pub(crate) fn check(_e: &Entry, _prefix: &str, _ab: &AbilityCfg) -> Result<(), RosterError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::Value as Json;

    use crate::roster::{AbilityKind, AbilitySettings, FighterCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn trumpet() -> FighterCfg {
        let all = parse_roster(ROSTER).expect("the roster loads");
        all.into_iter().find(|f| f.name == "TRUMPET").expect("in the roster")
    }

    /// The TRUMPET's entry with `key` of HIGH C removed (or set to `value`).
    fn low_with(key: &str, value: Option<Json>) -> Result<FighterCfg, crate::roster::RosterError> {
        let mut raw = trumpet().raw;
        let ab = raw["abilities"]["low"].as_object_mut().expect("a table");
        match value {
            Some(v) => _ = ab.insert(key.into(), v),
            None => _ = ab.remove(key),
        }
        FighterCfg::from_json(&raw)
    }

    #[test]
    fn high_c_is_a_beam_with_its_roster_numbers() {
        let f = trumpet();
        assert_eq!((f.low.name.as_str(), &f.low.kind), ("HIGH C", &AbilityKind::Beam));
        assert!(matches!(f.low.settings, AbilitySettings::Beam(_)));
        for (key, value) in [
            ("charge_beats", 1.0),
            ("lead", 1.0),
            ("len", 7.0),
            ("width", 1.1),
            ("dmg", 9.0),
            ("knock", 1.6),
            ("hold", 0.3),
        ] {
            assert_eq!(f.low.get(key), Some(value), "{key}");
        }
    }

    #[test]
    fn a_beam_needs_its_length_width_and_damage_and_nothing_else() {
        for key in ["len", "width", "dmg"] {
            assert_eq!(
                low_with(key, None).unwrap_err().to_string(),
                format!("roster: TRUMPET: abilities.low.{key}: missing")
            );
        }
        // the options have defaults in fight.py: an entry without them loads
        for key in ["charge_beats", "lead", "knock", "hold"] {
            let f = low_with(key, None).expect("loads");
            assert_eq!((&f.low.kind, f.low.get(key)), (&AbilityKind::Beam, None), "{key}");
        }
        // a key fight.py does not read for a beam: not a beam the engine runs
        let f = low_with("radius", Some(Json::from(2.0))).expect("loads");
        assert_eq!(f.low.kind, AbilityKind::Unsupported("beam+radius".into()));
    }
}
