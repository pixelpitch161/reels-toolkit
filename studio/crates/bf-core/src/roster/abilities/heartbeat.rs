//! Ability kind `heartbeat`, BASS DRUM's HEARTBEAT: its roster settings. `fight.py` `trigger` 2983 to 2984; `_thump`,
//! `start_heartbeat` 5440 to 5450.
//!
//! Two shockwaves, lub-dub: one at once and one `gap16` 16ths later (`int(ab.get('gap16', 2))`), each spreading at
//! `speed` to `radius`, hitting once for `dmg` and `knock`. The entry holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, WeaponCfg, join};

/// Whether the engine runs ability kind `heartbeat`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (`_thump`): `speed`, `radius`, `dmg`, `knock`.
pub const REQUIRED: &[&[&str]] = &[&["speed"], &["radius"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind (`gap16` with `ab.get('gap16', 2)`); an entry holding
/// another key loads as `Unsupported("heartbeat+<key>")`.
pub const ALLOWED: &[&str] = &["name", "kind", "every_bars", "speed", "radius", "dmg", "knock", "gap16"];

/// What an ability of kind `heartbeat` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeartbeatCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<HeartbeatCfg, RosterError> {
    Ok(HeartbeatCfg {})
}

/// The kind's own check: `int(ab.get('gap16', 2))` raises for a number that is not finite.
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(gap) = ab.get("gap16")
        && !gap.is_finite()
    {
        return Err(e.err(&join(prefix, "gap16"), format!("{gap:e} is not a finite number")));
    }
    Ok(())
}

/// The kind's check against the fighter's weapon: a `heartbeat` ability needs a `roll_charge` weapon. Python raises on
/// any other ball (`AttributeError`: only `rollc_setup` gives a ball `B.thumps`), so the entry is refused at load.
/// Called by [`crate::roster::kinds::roll_charge::check_ability`].
pub(crate) fn check_weapon(e: &Entry, prefix: &str, weapon: &WeaponCfg) -> Result<(), RosterError> {
    if matches!(weapon, WeaponCfg::RollCharge(_)) {
        return Ok(());
    }
    Err(e.err(
        &join(prefix, "kind"),
        format!(
            "HEARTBEAT (heartbeat) needs a roll_charge weapon, not {} (only a roller can run it)",
            weapon.kind()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use crate::roster::{Entry, FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    #[test]
    fn it_runs_only_on_a_roller() {
        let all = parse_roster(ROSTER).expect("the roster loads");
        let find = |n: &str| -> &FighterCfg { all.iter().find(|f| f.name == n).expect("in the roster") };
        let e = Entry { fighter: "TEST" };
        let drum = &find("BASS DRUM").weapon;
        assert!(matches!(drum, WeaponCfg::RollCharge(_)));
        assert!(super::check_weapon(&e, "abilities.unlock1", drum).is_ok());
        let err = super::check_weapon(&e, "abilities.unlock1", &find("FLUTE").weapon).expect_err("refused");
        assert_eq!(err.field, "abilities.unlock1.kind");
        assert!(
            err.message.contains("needs a roll_charge weapon, not striker"),
            "{}",
            err.message
        );
        // through the kind's dispatch, which the fighter's check calls
        let ab = &find("BASS DRUM").unlock1;
        assert!(crate::roster::kinds::roll_charge::check_ability(&e, "abilities.x", ab, &find("TUBA").weapon).is_err());
        assert!(crate::roster::kinds::roll_charge::check_ability(&e, "abilities.x", ab, drum).is_ok());
        // other kinds are not looked at
        let other = &find("TUBA").unlock1;
        assert!(
            crate::roster::kinds::roll_charge::check_ability(&e, "abilities.x", other, &find("FLUTE").weapon).is_ok()
        );
    }
}
