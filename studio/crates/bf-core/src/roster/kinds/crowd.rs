//! Weapon kind `crowd`, TENOR SAX's CROWD PLEASER: its roster settings. `fight.py` 4513 to 4757 (`crowd_setup`,
//! `_crowd_volley`, `_launch_throw`, `crowd_tick`, `crowd_step`, `start_runway`, `strut_step`, `start_dive`,
//! `dive_step`), and the abilities `runway`, `subtone` and `stagedive` (`trigger` 2966 to 2976).
//!
//! Every `every` beats (`off16` 16ths later; haste divides it) the tenor strikes a pose for `pose16` 16ths (its speed
//! x `pose_v`) and the crowd throws `int(hype)` things at the enemy from beyond the rope nearest it: `n0` at first,
//! plus `grow` per landed volley up to `n_max`, one every `stagger16` 16ths. A throw flies `flight16` 16ths to the
//! enemy's spot predicted `lead` x its motion (plus or minus `scatter`, seeded), but no further than the reach into
//! the ring (`reach`, plus `reach_grow` per landed volley up to `reach_max`). A landing within `splash` (+ 0.6 R) of
//! the enemy hits it (`dmg`, plus `dmg_grow` per landed volley up to `dmg_max`) and knocks it (`knock`).
//!
//! [`CrowdCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `crowd`.
pub const PORTED: bool = true;

/// Weapon kind `crowd`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct CrowdCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// Beats between two poses on the 16th grid (`int(round(4 * every))` 16ths; haste divides it).
    pub every: f64,
    /// 16ths after the grid's line a pose falls (`int(w.get('off16', 0))`).
    pub off16: f64,
    /// 16ths a pose lasts.
    pub pose16: f64,
    /// The speed factor while posing (`w.get('pose_v', 0.15)`).
    pub pose_v: f64,
    /// Throws of the first volley (`float(w['n0'])`, the hype; `int(hype + 1e-9)` are thrown).
    pub n0: f64,
    /// Hype added per landed volley.
    pub grow: f64,
    /// The hype never grows past this.
    pub n_max: f64,
    /// 16ths between two throws of a volley (`w.get('stagger16', 1)`).
    pub stagger16: f64,
    /// 16ths a throw flies.
    pub flight16: f64,
    /// How much of the enemy's motion during the flight a throw aims ahead by (`w.get('lead', 0.7)`).
    pub lead: f64,
    /// The landing spot's seeded scatter, either way on each axis (`w.get('scatter', 0.0)`).
    pub scatter: f64,
    /// How far beyond the rope the stands are (`w.get('stand', 0.9)`).
    pub stand: f64,
    /// The seeded spread of a throw's start along the stands (`w.get('stand_spread', 0.8)`).
    pub stand_spread: f64,
    /// How far into the ring from the nearest rope a throw reaches at first (`float(w['reach'])`, `blade_len`).
    pub reach: f64,
    /// Reach added per landed volley (`w.get('reach_grow', 0.0)`).
    pub reach_grow: f64,
    /// The reach never grows past this.
    pub reach_max: f64,
    /// A landing within this (+ 0.6 R) of the enemy's centre hits.
    pub splash: f64,
    /// Damage of a throw at first (`float(w['dmg'])`).
    pub dmg: f64,
    /// Damage added per landed volley (`w.get('dmg_grow', 0.0)`).
    pub dmg_grow: f64,
    /// A throw's damage never grows past this.
    pub dmg_max: f64,
    /// Knockback of a landed throw (`w.get('knock', 0.5)`).
    pub knock: f64,
}

/// The keys a crowd entry may hold: `kind`, `style`, `name` and [`CrowdCfg`]'s fields.
const CROWD_KEYS: [&str; 24] = [
    "kind",
    "style",
    "name",
    "every",
    "off16",
    "pose16",
    "pose_v",
    "n0",
    "grow",
    "n_max",
    "stagger16",
    "flight16",
    "lead",
    "scatter",
    "stand",
    "stand_spread",
    "reach",
    "reach_grow",
    "reach_max",
    "splash",
    "dmg",
    "dmg_grow",
    "dmg_max",
    "knock",
];

/// Reads a `crowd` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<CrowdCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a crowd weapon", &CROWD_KEYS)?;
    let cfg = CrowdCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "crowd".to_string(),
        },
        every: e.grid(w, p, "every")?,
        off16: e.f64_or(w, p, "off16", 0.0)?,
        pose16: e.f64(w, p, "pose16")?,
        pose_v: e.f64_or(w, p, "pose_v", 0.15)?,
        n0: e.f64(w, p, "n0")?,
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        stagger16: e.f64_or(w, p, "stagger16", 1.0)?,
        flight16: e.f64(w, p, "flight16")?,
        lead: e.f64_or(w, p, "lead", 0.7)?,
        scatter: e.f64_or(w, p, "scatter", 0.0)?,
        stand: e.f64_or(w, p, "stand", 0.9)?,
        stand_spread: e.f64_or(w, p, "stand_spread", 0.8)?,
        reach: e.f64(w, p, "reach")?,
        reach_grow: e.f64_or(w, p, "reach_grow", 0.0)?,
        reach_max: e.f64(w, p, "reach_max")?,
        splash: e.f64(w, p, "splash")?,
        dmg: e.f64(w, p, "dmg")?,
        dmg_grow: e.f64_or(w, p, "dmg_grow", 0.0)?,
        dmg_max: e.f64(w, p, "dmg_max")?,
        knock: e.f64_or(w, p, "knock", 0.5)?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(round(4 * every))` and `int(off16)` raise when the number is not finite, and so does
/// `int(B.n_vol + 1e-9)`, the count of a volley, when the hype is not (`n0`, and `grow` and `n_max` through
/// `min(n_max, n_vol + grow)`). Every other number runs as it is: a grid that rounds to 0 is every 16th note
/// (`max(1, ...)`), a hype below 1 throws nothing.
pub(crate) fn check(e: &Entry, w: &CrowdCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    for (key, x) in [("off16", w.off16), ("n0", w.n0), ("grow", w.grow), ("n_max", w.n_max)] {
        if !x.is_finite() {
            return Err(e.err(&format!("{p}.{key}"), format!("{x:e} is not a finite number")));
        }
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): `float(w.get('n0', 0))`, the throws of the
/// first volley (`n_vol`). A crowd entry has no `base`: the reach (`blade_len`) is set by `crowd_setup`.
pub fn ball_start(w: &CrowdCfg) -> BallStart {
    BallStart {
        n0: w.n0,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float): `crowd_tick` divides it by
/// the haste's factor.
pub(crate) fn main16(w: &CrowdCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

#[cfg(test)]
mod tests {
    //! What Python does with each refused entry is printed by
    //! `tools/studio_export/unit_checks/crowd_unit_vectors.py` (its last section).
    use serde_json::{Value as Json, json};

    use super::*;
    use crate::roster::{AbilityKind, FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn tenor() -> Json {
        let doc: Json = serde_json::from_str(ROSTER).expect("the roster is JSON");
        let fighters = doc["fighters"].as_array().expect("fighters");
        fighters
            .iter()
            .find(|f| f["name"] == "TENOR SAX")
            .expect("TENOR SAX is in the roster")
            .clone()
    }

    /// TENOR SAX's entry with one edit, read.
    fn edited(edit: impl Fn(&mut Json)) -> Result<FighterCfg, RosterError> {
        let mut raw = tenor();
        edit(&mut raw);
        FighterCfg::from_json(&raw)
    }

    /// TENOR SAX's entry without `key` of the table at `path` (`weapon`, or `abilities` and a slot).
    fn without(path: &[&str], key: &str) -> Result<FighterCfg, RosterError> {
        edited(|raw| {
            let mut at = &mut *raw;
            for p in path {
                at = &mut at[*p];
            }
            at.as_object_mut().expect("a table").remove(key);
        })
    }

    #[track_caller]
    fn refused(got: Result<FighterCfg, RosterError>, field: &str, says: &str) {
        let e = got.expect_err("refused");
        assert_eq!((e.fighter.as_str(), e.field.as_str()), ("TENOR SAX", field), "{e}");
        assert!(e.message.contains(says), "{e}");
    }

    #[test]
    fn the_tenor_is_a_crowd_pleaser_with_its_roster_numbers() {
        let roster = parse_roster(ROSTER).expect("the roster loads");
        let f = roster.iter().find(|f| f.name == "TENOR SAX").expect("TENOR SAX");
        let WeaponCfg::Crowd(w) = &f.weapon else {
            panic!("{:?}", f.weapon)
        };
        let want = CrowdCfg {
            style: "crowd".into(),
            every: 4.0,
            off16: 0.0,
            pose16: 2.0,
            pose_v: 0.15,
            n0: 3.0,
            grow: 0.34,
            n_max: 5.0,
            stagger16: 1.0,
            flight16: 6.0,
            lead: 0.8,
            scatter: 0.25,
            stand: 0.9,
            stand_spread: 0.8,
            reach: 1.9,
            reach_grow: 0.06,
            reach_max: 2.7,
            splash: 0.32,
            dmg: 1.9,
            dmg_grow: 0.1,
            dmg_max: 2.4,
            knock: 0.5,
        };
        assert_eq!(w, &want);
        let start = BallStart {
            n0: 3.0,
            ..BallStart::default()
        };
        assert_eq!(
            (f.weapon.kind(), main16(w), ball_start(w)),
            ("crowd", Some(16.0), start)
        );
        assert_eq!(
            (&f.unlock1.kind, &f.unlock2.kind, &f.low.kind),
            (&AbilityKind::Runway, &AbilityKind::Subtone, &AbilityKind::Stagedive)
        );
        assert_eq!(f.unlock1.get("honk_r"), Some(0.9));
        assert_eq!(f.unlock2.get("reach_k"), Some(1.3));
        assert_eq!(f.low.get("hit_r"), Some(0.15));
    }

    /// The defaults are `fight.py`'s `w.get(name, default)`.
    #[test]
    fn the_optional_numbers_take_pythons_defaults() {
        let keys = [
            "style",
            "off16",
            "pose_v",
            "stagger16",
            "lead",
            "scatter",
            "stand",
            "stand_spread",
            "reach_grow",
            "dmg_grow",
            "knock",
        ];
        let f = edited(|raw| {
            let w = raw["weapon"].as_object_mut().expect("weapon");
            for key in keys {
                w.remove(key);
            }
        })
        .expect("loads");
        let WeaponCfg::Crowd(w) = &f.weapon else {
            panic!("{:?}", f.weapon)
        };
        assert_eq!(
            (w.style.as_str(), w.off16, w.pose_v, w.stagger16, w.lead, w.scatter),
            ("crowd", 0.0, 0.15, 1.0, 0.7, 0.0)
        );
        assert_eq!(
            (w.stand, w.stand_spread, w.reach_grow, w.dmg_grow, w.knock),
            (0.9, 0.8, 0.0, 0.0, 0.5)
        );
    }

    /// Python raises `KeyError` (at setup, at the first pose, at the first launch or at the first landing) for a
    /// number it reads as `w[name]`; an unknown key it would ignore in silence.
    #[test]
    fn numbers_the_crowd_cannot_run_are_refused_by_name() {
        for key in [
            "every",
            "pose16",
            "n0",
            "grow",
            "n_max",
            "flight16",
            "reach",
            "reach_max",
            "splash",
            "dmg",
            "dmg_max",
        ] {
            refused(without(&["weapon"], key), &format!("weapon.{key}"), "missing");
        }
        refused(
            edited(|raw| raw["weapon"]["splash"] = json!("wide")),
            "weapon.splash",
            "must be a number",
        );
        refused(
            edited(|raw| raw["weapon"]["pull"] = json!(1.0)),
            "weapon.pull",
            "not a setting of a crowd weapon",
        );
        // a grid that rounds to no 16th note at all still runs (`max(1, ...)`): every 16th note
        edited(|raw| raw["weapon"]["every"] = json!(0.1)).expect("loads");
    }

    /// The three abilities need every number `fight.py` reads as `ab[name]`.
    #[test]
    fn the_abilities_need_their_numbers() {
        for key in [
            "beats",
            "dmg",
            "knock",
            "cool",
            "honk_r",
            "honk_dmg",
            "honk_knock",
            "every_bars",
        ] {
            refused(
                without(&["abilities", "unlock1"], key),
                &format!("abilities.unlock1.{key}"),
                "missing",
            );
        }
        for key in ["hush_beats", "n_k", "mult", "reach_k", "every_bars"] {
            refused(
                without(&["abilities", "unlock2"], key),
                &format!("abilities.unlock2.{key}"),
                "missing",
            );
        }
        for key in [
            "leap_beats",
            "surf_beats",
            "speed",
            "dmg",
            "knock",
            "radius",
            "ring_dmg",
            "ring_knock",
        ] {
            refused(
                without(&["abilities", "low"], key),
                &format!("abilities.low.{key}"),
                "missing",
            );
        }
        // the optional ones (`ab.get`): the dive's `lead`, `hit_r`, `stand`, `ring_speed`, the strut's `honk_speed`
        for key in ["lead", "hit_r"] {
            without(&["abilities", "low"], key).expect("loads");
        }
        edited(|raw| raw["abilities"]["low"]["ring_speed"] = json!(7.0)).expect("loads");
        edited(|raw| raw["abilities"]["unlock1"]["honk_speed"] = json!(7.0)).expect("loads");
    }

    /// A cannonball of no speed: Python divides the flight's length by `speed * V` (`ZeroDivisionError` at the
    /// launch).
    #[test]
    fn a_stage_dive_of_no_speed_is_refused() {
        refused(
            edited(|raw| raw["abilities"]["low"]["speed"] = json!(0)),
            "abilities.low.speed",
            "no speed",
        );
    }
}
