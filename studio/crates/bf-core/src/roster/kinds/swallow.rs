//! Weapon kind `swallow`, TUBA's the bell: its roster settings. `fight.py` 1045 to 1130 (`swallow_setup`,
//! `swallow_beat`, `swallow_step` with the `swallow_rollers` rule at 1100).
//!
//! [`SwallowCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `swallow`.
pub const PORTED: bool = true;

/// Weapon kind `swallow`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct SwallowCfg {
    /// Beats between two "oom"s on the beat grid (`max(1, int(round(every / ab_mult)))`: a haste divides it).
    pub every: f64,
    /// Beats the bell inhales after an "oom" (a haste divides it).
    pub inhale: f64,
    /// Beats from the "oom" to the "pah" (a haste divides it).
    pub pah: f64,
    /// The suction's starting reach beyond the two balls' surfaces (`blade_len`; `Ball` reads `.get('base', 0.0)`).
    pub base: f64,
    /// Reach added per landed gulp.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// How hard the suction turns the enemy's heading toward the bell, in units a second squared.
    pub suck: f64,
    /// The enemy is gulped when the balls' centres are closer than two radii plus this (`w.get('catch', 0.12)`).
    pub catch: f64,
    /// Radians a second the bell turns toward the enemy (`w.get('aim_rate', 6.0)`).
    pub aim_rate: f64,
    /// Damage of a gulp.
    pub dmg: f64,
    /// Damage of the spit.
    pub dmg_spit: f64,
    /// Damage when the spat enemy hits a rope in time (the cannonball).
    pub dmg_rope: f64,
    /// The spat enemy's speed boost (at least this).
    pub spit_boost: f64,
    /// Seconds after the spit in which a rope hit is a cannonball (`w.get('spit_t', 0.9)`).
    pub spit_t: f64,
    /// Degrees the spit is turned by at most, either way, seeded (`w.get('jitter', 25.0)`).
    pub jitter: f64,
}

/// The keys a swallow entry may hold: `kind`, `style` and [`SwallowCfg`]'s fields.
const SWALLOW_KEYS: [&str; 17] = [
    "kind",
    "style",
    "every",
    "inhale",
    "pah",
    "base",
    "grow",
    "max_reach",
    "suck",
    "catch",
    "aim_rate",
    "dmg",
    "dmg_spit",
    "dmg_rope",
    "spit_boost",
    "spit_t",
    "jitter",
];

/// Reads a `swallow` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<SwallowCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a swallow weapon", &SWALLOW_KEYS)?;
    let cfg = SwallowCfg {
        every: e.grid(w, p, "every")?,
        inhale: e.f64(w, p, "inhale")?,
        pah: e.f64(w, p, "pah")?,
        base: e.f64_or(w, p, "base", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        suck: e.f64(w, p, "suck")?,
        catch: e.f64_or(w, p, "catch", 0.12)?,
        aim_rate: e.f64_or(w, p, "aim_rate", 6.0)?,
        dmg: e.f64(w, p, "dmg")?,
        dmg_spit: e.f64(w, p, "dmg_spit")?,
        dmg_rope: e.f64(w, p, "dmg_rope")?,
        spit_boost: e.f64(w, p, "spit_boost")?,
        spit_t: e.f64_or(w, p, "spit_t", 0.9)?,
        jitter: e.f64_or(w, p, "jitter", 25.0)?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(round(every / m))` raises when `every` is not finite (the roster's haste check covers the
/// quotient).
pub(crate) fn check(e: &Entry, w: &SwallowCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `base`, the suction's starting reach.
pub fn ball_start(w: &SwallowCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The number a haste divides: `swallow_beat` takes `int(round(every / ab_mult(k)))`, which raises in Python when
/// the quotient is not finite. The bell's grid is in beats, not 16ths; the roster's check is the same division.
pub(crate) fn main16(w: &SwallowCfg) -> Option<f64> {
    Some(w.every)
}

#[cfg(test)]
mod tests {
    use crate::roster::{FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn tuba() -> serde_json::Value {
        let all: serde_json::Value = serde_json::from_str(ROSTER).expect("the roster is JSON");
        let list = all["fighters"].as_array().expect("a list of fighters");
        list.iter()
            .find(|f| f["name"] == "TUBA")
            .expect("in the roster")
            .clone()
    }

    #[test]
    fn the_tuba_loads_with_the_rosters_numbers() {
        let all = parse_roster(ROSTER).expect("the roster loads");
        let f = all.iter().find(|f| f.name == "TUBA").expect("in the roster");
        let WeaponCfg::Swallow(w) = &f.weapon else {
            panic!("the TUBA's weapon is a swallow: {:?}", f.weapon)
        };
        assert_eq!((w.every, w.inhale, w.pah), (4.0, 1.0, 2.0));
        assert_eq!((w.base, w.grow, w.max_reach), (1.8, 0.12, 3.0));
        assert_eq!((w.suck, w.catch, w.aim_rate), (26.0, 0.12, 6.0));
        assert_eq!((w.dmg, w.dmg_spit, w.dmg_rope), (3.0, 5.0, 5.0));
        assert_eq!((w.spit_boost, w.spit_t, w.jitter), (2.2, 0.9, 25.0));
        assert_eq!(f.weapon.ball_start().base, 1.8);
        assert_eq!(f.weapon.kind(), "swallow");
        assert!(f.unlock1.kind.is_ported() && f.unlock2.kind.is_ported() && f.low.kind.is_ported());
    }

    #[test]
    fn the_defaults_are_pythons() {
        let mut v = tuba();
        let w = v["weapon"].as_object_mut().expect("a table");
        for key in ["base", "catch", "aim_rate", "spit_t", "jitter", "style"] {
            w.remove(key);
        }
        let f = FighterCfg::from_json(&v).expect("every option has a default");
        let WeaponCfg::Swallow(w) = &f.weapon else {
            panic!("a swallow")
        };
        assert_eq!(
            (w.base, w.catch, w.aim_rate, w.spit_t, w.jitter),
            (0.0, 0.12, 6.0, 0.9, 25.0)
        );
    }

    #[test]
    fn what_the_fight_cannot_run_is_refused_naming_the_field() {
        let refused = |key: &str, value: serde_json::Value| {
            let mut v = tuba();
            v["weapon"][key] = value;
            let err = FighterCfg::from_json(&v).expect_err("refused");
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str()),
                ("TUBA", &*format!("weapon.{key}"))
            );
            err.message
        };
        assert!(refused("every", 1e308.into()).contains("finite"));
        assert!(refused("suck", "hard".into()).contains("number"));
        assert!(refused("pull", 1.into()).contains("not a setting of a swallow weapon"));
        for key in [
            "every",
            "inhale",
            "pah",
            "grow",
            "max_reach",
            "suck",
            "dmg",
            "dmg_spit",
            "dmg_rope",
            "spit_boost",
        ] {
            let mut v = tuba();
            v["weapon"].as_object_mut().expect("a table").remove(key);
            let err = FighterCfg::from_json(&v).expect_err("a missing number");
            assert_eq!(
                (err.field.as_str(), err.message.as_str()),
                (&*format!("weapon.{key}"), "missing")
            );
        }
        // an `every` that is not finite, made in code, is refused by the check the engine makes again
        let mut f = FighterCfg::from_json(&tuba()).expect("loads");
        if let WeaponCfg::Swallow(w) = &mut f.weapon {
            w.every = f64::INFINITY;
        }
        assert_eq!(f.check().expect_err("refused").field, "weapon.every");
        // OOMPAH's factor divides the bell's grid: a factor too small for it is refused
        let mut v = tuba();
        v["abilities"]["unlock1"]["mult"] = 1e-320.into();
        let err = FighterCfg::from_json(&v).expect_err("a quotient that is not finite");
        assert_eq!(err.field, "abilities.unlock1.mult");
    }
}
