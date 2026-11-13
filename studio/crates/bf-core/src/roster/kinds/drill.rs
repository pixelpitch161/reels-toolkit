//! Weapon kind `drill`, SOUSAPHONE's DRILL SET: its roster settings. `fight.py` 4758 to 5005 (`drill_setup`,
//! `drill_blocked`, `_drill_grow`, `drill_tick`, `_drill_blast`, `_capsule_hit`, `_trample_baskets`, `drill_step`,
//! `start_flash`, `start_front`), and the abilities `flash_spin` and `company_front` (`trigger` 2977 to 2982).
//!
//! [`DrillCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `drill`.
pub const PORTED: bool = true;

/// Weapon kind `drill`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct DrillCfg {
    /// Beats between two drill sets on the 16th grid (`int(round(4 * every))` 16ths; haste divides it).
    pub every: f64,
    /// 16ths after the bar line a set starts (`int(w.get('off16', 0))`).
    pub off16: f64,
    /// Steps of a set (`int(w['steps'])`), at least 1 in a roster entry.
    pub steps: f64,
    /// 16ths a step takes (haste divides it, at least 1).
    pub step16: f64,
    /// 16ths from the last footfall to the blast (`w.get('halt16', 2)`; haste divides it, at least 1).
    pub halt16: f64,
    /// How much of the enemy's motion up to the blast the march aims ahead by (`w.get('lead', 0.0)`).
    pub lead: f64,
    /// Degrees the line is turned by at most, either way, seeded (`w.get('jitter', 0.0)`).
    pub jitter: f64,
    /// The longest line of a set.
    pub span: f64,
    /// The halt stays this far short of the enemy's predicted spot (`w.get('stop')`; `None` when absent or null).
    pub stop: Option<f64>,
    /// With `stop`, a set marches at least this far (`w.get('min_len', 0.0)`).
    pub min_len: f64,
    /// Damage of a trample.
    pub trample_dmg: f64,
    /// Knockback of a trample.
    pub trample_knock: f64,
    /// The share of a trample's knock that goes on ahead along the line (`w.get('trample_fwd', 0.3)`).
    pub trample_fwd: f64,
    /// Seconds between two tramples of one set.
    pub cool: f64,
    /// A basket within this of the marcher's rim is trampled (`w.get('basket_r', 0.2)`).
    pub basket_r: f64,
    /// The blast's starting length (`blade_len`).
    pub base: f64,
    /// Length added per landed hit, trample or blast.
    pub grow: f64,
    /// The blast never grows past this.
    pub max_len: f64,
    /// The blast's width at the bell.
    pub width: f64,
    /// The blast's width at its far end (`w.get('spread', w['width'])`; `None` when absent).
    pub spread: Option<f64>,
    /// Seconds the blast is live (`w.get('hold', 0.2)`).
    pub hold: f64,
    /// Damage of the blast.
    pub dmg: f64,
    /// Knockback of the blast.
    pub knock: f64,
}

impl DrillCfg {
    /// `float(w.get('spread', w['width']))`: the blast's width at its far end.
    pub fn spread(&self) -> f64 {
        self.spread.unwrap_or(self.width)
    }
}

/// The keys a drill entry may hold: `kind`, `style`, `name` and [`DrillCfg`]'s fields.
const DRILL_KEYS: [&str; 26] = [
    "kind",
    "style",
    "name",
    "every",
    "off16",
    "steps",
    "step16",
    "halt16",
    "lead",
    "jitter",
    "span",
    "stop",
    "min_len",
    "trample_dmg",
    "trample_knock",
    "trample_fwd",
    "cool",
    "basket_r",
    "base",
    "grow",
    "max_len",
    "width",
    "spread",
    "hold",
    "dmg",
    "knock",
];

/// Why a set needs a step.
const STEPS_WHY: &str = "a drill set marches at least one step";

/// Reads a `drill` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<DrillCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a drill weapon", &DRILL_KEYS)?;
    let cfg = DrillCfg {
        every: e.grid(w, p, "every")?,
        off16: e.f64_or(w, p, "off16", 0.0)?,
        steps: e.count(w, p, "steps", STEPS_WHY)?,
        step16: e.f64(w, p, "step16")?,
        halt16: e.f64_or(w, p, "halt16", 2.0)?,
        lead: e.f64_or(w, p, "lead", 0.0)?,
        jitter: e.f64_or(w, p, "jitter", 0.0)?,
        span: e.f64(w, p, "span")?,
        // `w.get('stop') is not None`: a null is no stop
        stop: match w.get("stop") {
            None | Some(Json::Null) => None,
            Some(v) => Some(e.number(v, "weapon.stop")?),
        },
        min_len: e.f64_or(w, p, "min_len", 0.0)?,
        trample_dmg: e.f64(w, p, "trample_dmg")?,
        trample_knock: e.f64(w, p, "trample_knock")?,
        trample_fwd: e.f64_or(w, p, "trample_fwd", 0.3)?,
        cool: e.f64(w, p, "cool")?,
        basket_r: e.f64_or(w, p, "basket_r", 0.2)?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_len: e.f64(w, p, "max_len")?,
        width: e.f64(w, p, "width")?,
        spread: match w.get("spread") {
            None => None,
            Some(v) => Some(e.number(v, "weapon.spread")?),
        },
        hold: e.f64_or(w, p, "hold", 0.2)?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(round(4 * every))` raises when it is not finite; a set of no steps raises at its start
/// (`t_steps[-1]` of an empty list); `int(round(step16 / m))` and `int(round(halt16 / m))` raise on a number that
/// is not finite; a jitter that is not finite makes the heading NaN for the rest of the fight.
pub(crate) fn check(e: &Entry, w: &DrillCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    e.count_value(p, "steps", w.steps, STEPS_WHY)?;
    for (key, x) in [
        ("off16", w.off16),
        ("step16", w.step16),
        ("halt16", w.halt16),
        ("jitter", w.jitter),
    ] {
        if !x.is_finite() {
            return Err(e.err(&format!("{p}.{key}"), format!("{x:e} is not a finite number")));
        }
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): `base`, the blast's starting length.
pub fn ball_start(w: &DrillCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float): DOUBLE TIME divides it.
pub(crate) fn main16(w: &DrillCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

#[cfg(test)]
mod tests {
    use crate::roster::{FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn sousaphone() -> serde_json::Value {
        let all: serde_json::Value = serde_json::from_str(ROSTER).expect("the roster is JSON");
        let list = all["fighters"].as_array().expect("a list of fighters");
        list.iter()
            .find(|f| f["name"] == "SOUSAPHONE")
            .expect("in the roster")
            .clone()
    }

    #[test]
    fn the_sousaphone_loads_with_the_rosters_numbers() {
        let all = parse_roster(ROSTER).expect("the roster loads");
        let f = all.iter().find(|f| f.name == "SOUSAPHONE").expect("in the roster");
        let WeaponCfg::Drill(w) = &f.weapon else {
            panic!("the SOUSAPHONE's weapon is a drill: {:?}", f.weapon)
        };
        assert_eq!(
            (w.every, w.off16, w.steps, w.step16, w.halt16),
            (8.0, 0.0, 8.0, 2.0, 2.0)
        );
        assert_eq!(
            (w.lead, w.jitter, w.span, w.stop, w.min_len),
            (0.6, 6.0, 5.0, Some(1.3), 1.6)
        );
        assert_eq!((w.trample_dmg, w.trample_knock, w.trample_fwd), (3.6, 1.3, 0.85));
        assert_eq!(
            (w.cool, w.basket_r, w.base, w.grow, w.max_len),
            (0.5, 0.2, 2.0, 0.12, 3.6)
        );
        assert_eq!(
            (w.width, w.spread(), w.hold, w.dmg, w.knock),
            (0.5, 1.6, 0.25, 12.0, 1.3)
        );
        assert_eq!(f.weapon.ball_start().base, 2.0);
        assert!(f.unlock1.kind.is_ported() && f.unlock2.kind.is_ported() && f.low.kind.is_ported());
    }

    #[test]
    fn the_defaults_are_pythons() {
        let mut v = sousaphone();
        let w = v["weapon"].as_object_mut().expect("a table");
        for key in [
            "off16",
            "halt16",
            "lead",
            "jitter",
            "stop",
            "min_len",
            "trample_fwd",
            "basket_r",
            "spread",
            "hold",
        ] {
            w.remove(key);
        }
        let f = FighterCfg::from_json(&v).expect("every option has a default");
        let WeaponCfg::Drill(w) = &f.weapon else {
            panic!("a drill")
        };
        assert_eq!(
            (w.off16, w.halt16, w.lead, w.jitter, w.stop),
            (0.0, 2.0, 0.0, 0.0, None)
        );
        assert_eq!(
            (w.min_len, w.trample_fwd, w.basket_r, w.spread(), w.hold),
            (0.0, 0.3, 0.2, 0.5, 0.2)
        );
        // `w.get('stop') is not None`: a null is no stop
        let mut v = sousaphone();
        v["weapon"]["stop"] = serde_json::Value::Null;
        let f = FighterCfg::from_json(&v).expect("a null stop is no stop");
        assert!(matches!(&f.weapon, WeaponCfg::Drill(w) if w.stop.is_none()));
    }

    #[test]
    fn what_the_fight_cannot_run_is_refused_naming_the_field() {
        let refused = |key: &str, value: serde_json::Value| {
            let mut v = sousaphone();
            v["weapon"][key] = value;
            let err = FighterCfg::from_json(&v).expect_err("refused");
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str()),
                ("SOUSAPHONE", &*format!("weapon.{key}"))
            );
            err.message
        };
        assert!(refused("steps", 0.into()).contains("marches at least one step"));
        assert!(refused("steps", 0.9.into()).contains("less than 1"));
        assert!(refused("every", 1e308.into()).contains("finite"));
        assert!(refused("step16", "two".into()).contains("number"));
        assert!(refused("pull", 1.into()).contains("not a setting of a drill weapon"));
        let mut v = sousaphone();
        v["weapon"].as_object_mut().expect("a table").remove("cool");
        let err = FighterCfg::from_json(&v).expect_err("a missing number");
        assert_eq!((err.field.as_str(), err.message.as_str()), ("weapon.cool", "missing"));
        // a set of no steps made in code is refused by the check the engine makes again
        let mut f = FighterCfg::from_json(&sousaphone()).expect("loads");
        if let WeaponCfg::Drill(w) = &mut f.weapon {
            w.steps = 0.0;
        }
        assert_eq!(f.check().expect_err("refused").field, "weapon.steps");
        // COMPANY FRONT's wall takes at least one step
        let mut v = sousaphone();
        v["abilities"]["low"]["beats"] = 0.into();
        let err = FighterCfg::from_json(&v).expect_err("a wall of no beats");
        assert_eq!(err.field, "abilities.low.beats");
        // HORN FLASH needs its numbers
        let mut v = sousaphone();
        v["abilities"]["unlock1"]
            .as_object_mut()
            .expect("a table")
            .remove("width");
        let err = FighterCfg::from_json(&v).expect_err("a flash without a width");
        assert_eq!(
            (err.field.as_str(), err.message.as_str()),
            ("abilities.unlock1.width", "missing")
        );
    }
}
