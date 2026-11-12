//! Ability kind `drum_roll`, BASS DRUM's DRUM ROLL: its roster settings. `fight.py` `trigger` 2985 to 2986;
//! `_next_leg`, `start_drum_roll` 5451 to 5476.
//!
//! For `beats` beats the drum rolls nonstop: legs of `leg16` 16ths (`int(ab.get('leg16', 3))`) with no tipping, each
//! aimed at the enemy turned `zig` degrees to alternate sides (`ab.get('zig', 30.0)`), at `speed_k` x the roll's
//! speed (`ab.get('speed_k', 1.0)`), a contact every `cool` s at most (`ab.get('cool', 0.25)`) for `dmg` and `knock`
//! (`fwd` of it on ahead: `ab.get('fwd', w.get('fwd', 0.4))`). The entry holds only numbers.

use serde_json::Value as Json;

use crate::roster::{AbilityCfg, Entry, RosterError, WeaponCfg, join};

/// Whether the engine runs ability kind `drum_roll`.
pub const PORTED: bool = true;

/// The numbers `fight.py` reads as `ab[name]` for this kind (`start_drum_roll`, `rollc_step`): `beats`, `dmg`,
/// `knock`.
pub const REQUIRED: &[&[&str]] = &[&["beats"], &["dmg"], &["knock"]];

/// Every key `fight.py` reads from an ability of this kind; an entry holding another key loads as
/// `Unsupported("drum_roll+<key>")`.
pub const ALLOWED: &[&str] = &[
    "name",
    "kind",
    "every_bars",
    "beats",
    "leg16",
    "zig",
    "speed_k",
    "cool",
    "dmg",
    "knock",
    "fwd",
];

/// What an ability of kind `drum_roll` holds besides its numbers: nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrumRollCfg {}

/// Reads what is not a number from an ability entry of this kind: nothing.
pub(crate) fn parse(_e: &Entry, _prefix: &str, _ab: &Json) -> Result<DrumRollCfg, RosterError> {
    Ok(DrumRollCfg {})
}

/// The kind's own checks: `int(ab.get('leg16', 3))` raises for a number that is not finite, and a `speed_k` of 0
/// raises a division by zero at the first leg (`0.5 * span / (V * self.R.V)`).
pub(crate) fn check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    if let Some(leg) = ab.get("leg16")
        && !leg.is_finite()
    {
        return Err(e.err(&join(prefix, "leg16"), format!("{leg:e} is not a finite number")));
    }
    if ab.get("speed_k") == Some(0.0) {
        return Err(e.err(
            &join(prefix, "speed_k"),
            "0 is no speed (a leg's length in time divides by it)",
        ));
    }
    Ok(())
}

/// The kind's check against the fighter's weapon: a `drum_roll` ability needs a `roll_charge` weapon. Python raises on
/// any other ball (`AttributeError`: only `rollc_setup` gives a ball `B.rl` and `B.roll_v`), so the entry is refused at load.
/// Called by [`crate::roster::kinds::roll_charge::check_ability`].
pub(crate) fn check_weapon(e: &Entry, prefix: &str, weapon: &WeaponCfg) -> Result<(), RosterError> {
    if matches!(weapon, WeaponCfg::RollCharge(_)) {
        return Ok(());
    }
    Err(e.err(
        &join(prefix, "kind"),
        format!(
            "DRUM ROLL (drum_roll) needs a roll_charge weapon, not {} (only a roller can run it)",
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
        let ab = &find("BASS DRUM").unlock2;
        assert!(crate::roster::kinds::roll_charge::check_ability(&e, "abilities.x", ab, &find("TUBA").weapon).is_err());
        assert!(crate::roster::kinds::roll_charge::check_ability(&e, "abilities.x", ab, drum).is_ok());
        // other kinds are not looked at
        let other = &find("TUBA").unlock1;
        assert!(
            crate::roster::kinds::roll_charge::check_ability(&e, "abilities.x", other, &find("FLUTE").weapon).is_ok()
        );
    }
}
