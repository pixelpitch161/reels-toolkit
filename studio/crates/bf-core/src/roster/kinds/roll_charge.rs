//! Weapon kind `roll_charge`, BASS DRUM's STEAMROLLER: its roster settings. `fight.py` 5284 to 5431 (`rollc_setup`,
//! `rollc_blocked`, `_roll_line`, `_start_roll`, `_end_roll`, `rollc_tick`, `rollc_step`), and the abilities
//! `heartbeat`, `drum_roll` and `cannons` (`trigger` 2983 to 2988; 5432 to 5494).
//!
//! [`RollChargeCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `roll_charge`.
pub const PORTED: bool = true;

/// Weapon kind `roll_charge`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct RollChargeCfg {
    /// Beats between two rolls on the 16th grid (`int(round(4 * every))` 16ths, at least 1).
    pub every: f64,
    /// 16ths after the bar line a roll starts (`int(w.get('off16', 0))`).
    pub off16: f64,
    /// 16ths the drum tips onto its side before it rolls (`int(w.get('tip16', 2))`): the line is telegraphed.
    pub tip16: f64,
    /// How far ahead along the enemy's motion the line is aimed, as a share of the roll's time
    /// (`w.get('lead', 0.0)`).
    pub lead: f64,
    /// Degrees the line is turned by at most, either way, seeded (`w.get('jitter', 0.0)`).
    pub jitter: f64,
    /// The longest roll (`w['span']`), stopping a radius short of the ropes.
    pub span: f64,
    /// A roll shorter than this is not started: boxed in against the ropes (`w.get('min_len', 0.6)`).
    pub min_len: f64,
    /// The roll's speed, a multiple of the shared speed, at the start (`w['speed']`).
    pub speed: f64,
    /// Speed added per landed BOOM (`w.get('speed_grow', 0.0)`).
    pub speed_grow: f64,
    /// The roll's speed never grows past this (`w.get('speed_max', B.roll_v)`: `None` keeps the speed as it is).
    pub speed_max: Option<f64>,
    /// The drum's size at the start, held in `blade_len` (`w.get('size0', 0.0)`): its reach past the ball.
    pub size0: f64,
    /// Size added per landed BOOM (`w.get('size_grow', 0.0)`).
    pub size_grow: f64,
    /// The size never grows past this (`w.get('size_max', B.blade_len)`: `None` keeps it as it is; the look reads
    /// `.get('size_max', 0.25)`).
    pub size_max: Option<f64>,
    /// Damage of a BOOM.
    pub dmg: f64,
    /// Knockback of a BOOM.
    pub knock: f64,
    /// The share of a BOOM's knock that goes on ahead along the line, the rest out to the side
    /// (`w.get('fwd', 0.4)`; DRUM ROLL's contacts use it too unless the ability has its own).
    pub fwd: f64,
}

/// The keys a roll_charge entry may hold: `kind`, `style`, `name` and [`RollChargeCfg`]'s fields.
const ROLL_KEYS: [&str; 19] = [
    "kind",
    "style",
    "name",
    "every",
    "off16",
    "tip16",
    "lead",
    "jitter",
    "span",
    "min_len",
    "speed",
    "speed_grow",
    "speed_max",
    "size0",
    "size_grow",
    "size_max",
    "dmg",
    "knock",
    "fwd",
];

/// `w.get(key)` as an optional number.
fn optional(e: &Entry, w: &Json, key: &str) -> Result<Option<f64>, RosterError> {
    match w.get(key) {
        None => Ok(None),
        Some(v) => Ok(Some(e.number(v, &format!("weapon.{key}"))?)),
    }
}

/// Reads a `roll_charge` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<RollChargeCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a roll_charge weapon", &ROLL_KEYS)?;
    let cfg = RollChargeCfg {
        every: e.grid(w, p, "every")?,
        off16: e.f64_or(w, p, "off16", 0.0)?,
        tip16: e.f64_or(w, p, "tip16", 2.0)?,
        lead: e.f64_or(w, p, "lead", 0.0)?,
        jitter: e.f64_or(w, p, "jitter", 0.0)?,
        span: e.f64(w, p, "span")?,
        min_len: e.f64_or(w, p, "min_len", 0.6)?,
        speed: e.f64(w, p, "speed")?,
        speed_grow: e.f64_or(w, p, "speed_grow", 0.0)?,
        speed_max: optional(e, w, "speed_max")?,
        size0: e.f64_or(w, p, "size0", 0.0)?,
        size_grow: e.f64_or(w, p, "size_grow", 0.0)?,
        size_max: optional(e, w, "size_max")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
        fwd: e.f64_or(w, p, "fwd", 0.4)?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(round(4 * every))`, `int(off16)` and `int(tip16)` raise when the number is not finite; a jitter
/// that is not finite makes the line NaN for the rest of the fight; a roll speed of 0 raises a division by zero at
/// the next roll (`0.5 * span / (roll_v * V)`), and so does a `speed_max` of 0 after the first landed BOOM.
pub(crate) fn check(e: &Entry, w: &RollChargeCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    for (key, x) in [("off16", w.off16), ("tip16", w.tip16), ("jitter", w.jitter)] {
        if !x.is_finite() {
            return Err(e.err(&format!("{p}.{key}"), format!("{x:e} is not a finite number")));
        }
    }
    for (key, x) in [("speed", Some(w.speed)), ("speed_max", w.speed_max)] {
        if x == Some(0.0) {
            return Err(e.err(
                &format!("{p}.{key}"),
                "0 is no speed (the next roll's length in time divides by it)",
            ));
        }
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): nothing of its own (`rollc_setup` sets the
/// drum's size, `blade_len`, from `size0`).
pub fn ball_start(_w: &RollChargeCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid: none a haste divides (`rollc_tick` reads `every` as it is, and the drum has no
/// haste).
pub(crate) fn main16(_w: &RollChargeCfg) -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use crate::roster::{FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn bass_drum() -> serde_json::Value {
        let all: serde_json::Value = serde_json::from_str(ROSTER).expect("the roster is JSON");
        let list = all["fighters"].as_array().expect("a list of fighters");
        list.iter()
            .find(|f| f["name"] == "BASS DRUM")
            .expect("in the roster")
            .clone()
    }

    #[test]
    fn the_bass_drum_loads_with_the_rosters_numbers() {
        let all = parse_roster(ROSTER).expect("the roster loads");
        let f = all.iter().find(|f| f.name == "BASS DRUM").expect("in the roster");
        let WeaponCfg::RollCharge(w) = &f.weapon else {
            panic!("the BASS DRUM's weapon is a roll_charge: {:?}", f.weapon)
        };
        assert_eq!((w.every, w.off16, w.tip16, w.lead, w.jitter), (4.0, 0.0, 2.0, 0.5, 8.0));
        assert_eq!((w.span, w.min_len, w.speed, w.speed_grow), (5.2, 0.6, 2.4, 0.06));
        assert_eq!((w.speed_max, w.size0, w.size_grow, w.size_max), (Some(3.4), 0.0, 0.025, Some(0.25)));
        assert_eq!((w.dmg, w.knock, w.fwd), (7.0, 1.8, 0.35));
        assert_eq!(f.weapon.ball_start(), crate::roster::BallStart::default());
        assert!(f.unlock1.kind.is_ported() && f.unlock2.kind.is_ported() && f.low.kind.is_ported());
        assert_eq!(
            (f.unlock1.kind.name(), f.unlock2.kind.name(), f.low.kind.name()),
            ("heartbeat", "drum_roll", "cannons")
        );
    }

    #[test]
    fn the_defaults_are_pythons() {
        let mut v = bass_drum();
        let w = v["weapon"].as_object_mut().expect("a table");
        for key in [
            "off16",
            "tip16",
            "lead",
            "jitter",
            "min_len",
            "speed_grow",
            "speed_max",
            "size0",
            "size_grow",
            "size_max",
            "fwd",
        ] {
            w.remove(key);
        }
        let f = FighterCfg::from_json(&v).expect("every option has a default");
        let WeaponCfg::RollCharge(w) = &f.weapon else {
            panic!("a roll_charge")
        };
        assert_eq!((w.off16, w.tip16, w.lead, w.jitter, w.min_len), (0.0, 2.0, 0.0, 0.0, 0.6));
        assert_eq!((w.speed_grow, w.speed_max, w.size0, w.size_grow), (0.0, None, 0.0, 0.0));
        assert_eq!((w.size_max, w.fwd), (None, 0.4));
    }

    #[test]
    fn what_the_fight_cannot_run_is_refused_naming_the_field() {
        let refused = |key: &str, value: serde_json::Value| {
            let mut v = bass_drum();
            v["weapon"][key] = value;
            let err = FighterCfg::from_json(&v).expect_err("refused");
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str()),
                ("BASS DRUM", &*format!("weapon.{key}"))
            );
            err.message
        };
        assert!(refused("every", 1e308.into()).contains("finite"));
        assert!(refused("speed", 0.into()).contains("no speed"));
        assert!(refused("speed_max", 0.0.into()).contains("no speed"));
        assert!(refused("span", "far".into()).contains("number"));
        assert!(refused("base", 1.into()).contains("not a setting of a roll_charge weapon"));
        let mut v = bass_drum();
        v["weapon"].as_object_mut().expect("a table").remove("dmg");
        let err = FighterCfg::from_json(&v).expect_err("a missing number");
        assert_eq!((err.field.as_str(), err.message.as_str()), ("weapon.dmg", "missing"));
        // made in code: the checks the engine makes again
        for key in ["tip16", "jitter"] {
            let mut f = FighterCfg::from_json(&bass_drum()).expect("loads");
            if let WeaponCfg::RollCharge(w) = &mut f.weapon {
                if key == "tip16" {
                    w.tip16 = f64::INFINITY;
                } else {
                    w.jitter = f64::NAN;
                }
            }
            assert_eq!(f.check().expect_err("refused").field, format!("weapon.{key}"));
        }
        // the abilities' own numbers
        for (slot, key) in [("unlock1", "radius"), ("unlock2", "beats"), ("low", "tele")] {
            let mut v = bass_drum();
            v["abilities"][slot].as_object_mut().expect("a table").remove(key);
            let err = FighterCfg::from_json(&v).expect_err("missing");
            assert_eq!(
                (err.field.as_str(), err.message.as_str()),
                (&*format!("abilities.{slot}.{key}"), "missing")
            );
        }
        let mut v = bass_drum();
        v["abilities"]["unlock2"]["speed_k"] = 0.into();
        let err = FighterCfg::from_json(&v).expect_err("a drum roll of no speed");
        assert_eq!(err.field, "abilities.unlock2.speed_k");
        let mut v = bass_drum();
        v["abilities"]["unlock1"]["pause"] = 1.into();
        let f = FighterCfg::from_json(&v).expect("an unknown key makes the kind one the engine has not ported");
        assert!(!f.unlock1.kind.is_ported(), "{:?}", f.unlock1.kind);
    }
}
