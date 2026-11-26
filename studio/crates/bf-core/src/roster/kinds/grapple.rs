//! Weapon kind `grapple`, CELLO's THE EMBRACE: its roster settings. `fight.py` 4041 to 4201 (`GRAB_STATES`,
//! `grapple_setup`, `grapple_busy`, `grapple_tick`, `_end_rush`, `_nearest_rope_angle`, `grapple_release`,
//! `grapple_step`).
//!
//! [`GrappleCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `grapple`.
pub const PORTED: bool = true;

/// Weapon kind `grapple`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct GrappleCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// The weapon's name on the card (`w.get('name')`; the fight never reads it).
    pub name: Option<String>,
    /// Beats between two rushes on the 16th grid (`int(round(4 * every))`, a haste divides it, at least one 16th).
    pub every: f64,
    /// 16ths after the bar line a rush starts (`int(w.get('off16', 0))`). In a roster entry its size is at most 2^53.
    pub off16: f64,
    /// 16ths a rush lasts (`w['rush16'] * beat / 4`).
    pub rush16: f64,
    /// The rush's speed, times the shared speed (`vmul`).
    pub rush_speed: f64,
    /// Radians a second the rush steers for the enemy (`w.get('track', 6.0)`).
    pub track: f64,
    /// How much of the enemy's motion over the rush the aim leads by (`w.get('lead', 0.0)`).
    pub lead: f64,
    /// The reach beyond contact at the start (`blade_len`; `Ball` reads `.get('base', 0.0)`, `grapple_setup`
    /// `w['base']`).
    pub base: f64,
    /// Reach added per landed throw.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// Beats the enemy is held and swung.
    pub hold_beats: f64,
    /// The swing is at least this many turns (`w.get('turns', 0.5)`).
    pub turns: f64,
    /// Degrees the throw is turned by at most, either way, seeded (`w.get('jitter', 0.0)`).
    pub jitter: f64,
    /// Damage of the grab.
    pub dmg: f64,
    /// Damage of the throw.
    pub dmg_throw: f64,
    /// Damage when the thrown enemy hits a rope in time (the rope slam).
    pub dmg_rope: f64,
    /// Seconds after the throw in which a rope hit is a rope slam.
    pub rope_t: f64,
    /// The thrown enemy's speed boost at the start.
    pub power: f64,
    /// Boost added per landed throw.
    pub power_grow: f64,
    /// The boost never grows past this.
    pub power_max: f64,
}

/// The keys a grapple entry may hold: `kind`, `style`, `name` and [`GrappleCfg`]'s numbers.
const GRAPPLE_KEYS: [&str; 22] = [
    "kind",
    "style",
    "name",
    "every",
    "off16",
    "rush16",
    "rush_speed",
    "track",
    "lead",
    "base",
    "grow",
    "max_reach",
    "hold_beats",
    "turns",
    "jitter",
    "dmg",
    "dmg_throw",
    "dmg_rope",
    "rope_t",
    "power",
    "power_grow",
    "power_max",
];

/// The offset in 16ths: Python takes `int(w.get('off16', 0))`, a whole number of any size; the engine's are 64-bit,
/// so the entry keeps to what a float holds exactly (and is a number Python's `int` takes: finite).
fn off16_value(e: &Entry, x: f64) -> Result<f64, RosterError> {
    if !x.is_finite() || x.abs() > 9_007_199_254_740_992.0 {
        return Err(e.err("weapon.off16", format!("{x:e} sixteenths is too far (at most 2^53)")));
    }
    Ok(x)
}

/// Reads a `grapple` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<GrappleCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a grapple weapon", &GRAPPLE_KEYS)?;
    let cfg = GrappleCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "grapple".to_string(),
        },
        name: match w.get("name") {
            Some(_) => Some(e.string(w, p, "name")?),
            None => None,
        },
        every: e.grid(w, p, "every")?,
        off16: e.f64_or(w, p, "off16", 0.0)?,
        rush16: e.f64(w, p, "rush16")?,
        rush_speed: e.f64(w, p, "rush_speed")?,
        track: e.f64_or(w, p, "track", 6.0)?,
        lead: e.f64_or(w, p, "lead", 0.0)?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        hold_beats: e.f64(w, p, "hold_beats")?,
        turns: e.f64_or(w, p, "turns", 0.5)?,
        jitter: e.f64_or(w, p, "jitter", 0.0)?,
        dmg: e.f64(w, p, "dmg")?,
        dmg_throw: e.f64(w, p, "dmg_throw")?,
        dmg_rope: e.f64(w, p, "dmg_rope")?,
        rope_t: e.f64(w, p, "rope_t")?,
        power: e.f64(w, p, "power")?,
        power_grow: e.f64(w, p, "power_grow")?,
        power_max: e.f64(w, p, "power_max")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(round(4 * every))` raises when `4 * every` is not finite; `int(off16)` when `off16` is not.
pub(crate) fn check(e: &Entry, w: &GrappleCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    off16_value(e, w.off16)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `base`, the reach at the start
/// (`grapple_setup` sets `blade_len` to it again).
pub fn ball_start(w: &GrappleCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float): `grapple_tick` divides it by
/// the haste's factor.
pub(crate) fn main16(w: &GrappleCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

#[cfg(test)]
mod tests {
    use crate::roster::{FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn cello() -> serde_json::Value {
        let all: serde_json::Value = serde_json::from_str(ROSTER).expect("the roster is JSON");
        let list = all["fighters"].as_array().expect("a list of fighters");
        list.iter()
            .find(|f| f["name"] == "CELLO")
            .expect("in the roster")
            .clone()
    }

    #[test]
    fn the_cello_loads_with_the_rosters_numbers() {
        let all = parse_roster(ROSTER).expect("the roster loads");
        let f = all.iter().find(|f| f.name == "CELLO").expect("in the roster");
        let WeaponCfg::Grapple(w) = &f.weapon else {
            panic!("the CELLO's weapon is a grapple: {:?}", f.weapon)
        };
        assert_eq!((w.style.as_str(), w.name.as_deref()), ("embrace", Some("THE EMBRACE")));
        assert_eq!((w.every, w.off16, w.rush16, w.rush_speed), (4.0, 0.0, 2.0, 2.6));
        assert_eq!(
            (w.track, w.lead, w.base, w.grow, w.max_reach),
            (6.0, 0.5, 0.22, 0.03, 0.65)
        );
        assert_eq!((w.hold_beats, w.turns, w.jitter), (1.0, 0.6, 15.0));
        assert_eq!((w.dmg, w.dmg_throw, w.dmg_rope, w.rope_t), (1.0, 1.0, 2.2, 0.9));
        assert_eq!((w.power, w.power_grow, w.power_max), (1.6, 0.06, 2.4));
        assert_eq!(f.weapon.ball_start().base, 0.22);
        assert_eq!(f.weapon.kind(), "grapple");
        assert!(f.unlock1.kind.is_ported() && f.unlock2.kind.is_ported() && f.low.kind.is_ported());
    }

    #[test]
    fn the_defaults_are_pythons() {
        let mut v = cello();
        let w = v["weapon"].as_object_mut().expect("a table");
        for key in ["off16", "track", "lead", "turns", "jitter", "style", "name"] {
            w.remove(key);
        }
        let f = FighterCfg::from_json(&v).expect("every option has a default");
        let WeaponCfg::Grapple(w) = &f.weapon else {
            panic!("a grapple")
        };
        assert_eq!((w.off16, w.track, w.lead, w.turns, w.jitter), (0.0, 6.0, 0.0, 0.5, 0.0));
        assert_eq!((w.style.as_str(), w.name.as_deref()), ("grapple", None));
    }

    #[test]
    fn what_the_fight_cannot_run_is_refused_naming_the_field() {
        let refused = |key: &str, value: serde_json::Value| {
            let mut v = cello();
            v["weapon"][key] = value;
            let err = FighterCfg::from_json(&v).expect_err("refused");
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str()),
                ("CELLO", &*format!("weapon.{key}"))
            );
            err.message
        };
        assert!(refused("every", 1e308.into()).contains("finite"));
        assert!(refused("off16", 1e300.into()).contains("2^53"));
        assert!(refused("dmg", "hard".into()).contains("number"));
        assert!(refused("spin", 1.into()).contains("not a setting of a grapple weapon"));
        for key in [
            "every",
            "rush16",
            "rush_speed",
            "base",
            "grow",
            "max_reach",
            "hold_beats",
            "dmg",
            "dmg_throw",
            "dmg_rope",
            "rope_t",
            "power",
            "power_grow",
            "power_max",
        ] {
            let mut v = cello();
            v["weapon"].as_object_mut().expect("a table").remove(key);
            let err = FighterCfg::from_json(&v).expect_err("a missing number");
            assert_eq!(
                (err.field.as_str(), err.message.as_str()),
                (&*format!("weapon.{key}"), "missing")
            );
        }
    }
}
