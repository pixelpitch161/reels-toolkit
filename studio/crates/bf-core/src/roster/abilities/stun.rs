//! Ability kind `stun`, CLARINET's SQUEAK: its roster settings. `fight.py` `trigger` 2889 to 2892; the stun hit of
//! `effects_step` 3072 to 3074.
//!
//! A ring like a wave's (`speed`, `radius`, `dmg`, `knock`) whose hit also stuns: the fighter it lands on holds still
//! and starts no attack for `stun` seconds. It holds only numbers, so [`StunCfg`] is empty.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError};

/// Whether the engine runs ability kind `stun`. While false the roster reads it as `AbilityKind::Unsupported("stun")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (a group of names: one of them, as `dur` or `dur_beats`).
/// An unlock's `every_bars` is checked for every kind.
pub const REQUIRED: &[&[&str]] = &[&["speed"], &["radius"], &["dmg"], &["knock"], &["stun"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("stun+<key>")`. (The ring is made with `slow=None`: a `slow` here would be ignored, so it is not let
/// in.)
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "speed", "radius", "dmg", "knock", "stun"];

/// What an ability of kind `stun` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StunCfg {}

/// Reads what is not a number from an ability entry of this kind; called only when [`PORTED`] is true, after
/// [`REQUIRED`] and [`ALLOWED`] are checked. `prefix` is the field's path, such as `abilities.unlock1`.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<StunCfg, RosterError> {
    Ok(StunCfg {})
}

/// The kind's own checks on an entry's numbers, at load time and again in `FighterCfg::check`: none (any number
/// runs, as in Python: a `stun` of 0 stuns nobody).
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
    fn squeak_is_typed_and_needs_its_numbers() {
        let cfg = FighterCfg::from_json(&clarinet()).unwrap();
        assert_eq!(cfg.unlock1.kind, AbilityKind::Stun);
        assert_eq!(cfg.unlock1.name, "SQUEAK");
        let got: Vec<f64> = ["speed", "radius", "dmg", "knock", "stun", "every_bars"]
            .iter()
            .map(|k| cfg.unlock1.get(k).unwrap())
            .collect();
        assert_eq!(got, [9.0, 2.4, 2.0, 0.4, 0.5, 4.0]);
        for k in ["speed", "radius", "dmg", "knock", "stun", "every_bars"] {
            let mut c = clarinet();
            c["abilities"]["unlock1"].as_object_mut().unwrap().remove(k);
            let err = FighterCfg::from_json(&c).unwrap_err();
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str(), err.message.as_str()),
                ("CLARINET", &*format!("abilities.unlock1.{k}"), "missing"),
                "{k}"
            );
        }
        // a key the kind never reads makes it another kind, never typed with the key dropped
        let mut c = clarinet();
        c["abilities"]["unlock1"]["slow"] = json!({"mult": 0.5, "dur": 1.0});
        let cfg = FighterCfg::from_json(&c).unwrap();
        assert_eq!(cfg.unlock1.kind, AbilityKind::Unsupported("stun+slow".into()));
    }
}
