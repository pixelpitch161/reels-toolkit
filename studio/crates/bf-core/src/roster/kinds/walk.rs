//! Weapon kind `walk`, CONTRABASS's WALKING BASS: its roster settings. `fight.py` 3965 to 4040 (`walk_setup`,
//! `walk_blocked`, `walk_tick`, `walk_step`, `_step_land`), and the walker's side of the clash in `move` (3190 to 3208,
//! 3227 to 3231).
//!
//! [`WalkCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `walk`.
pub const PORTED: bool = true;

/// Weapon kind `walk`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct WalkCfg {
    /// Beats between two footfalls on the 16th grid (`int(round(4 * every))` 16ths; haste divides it).
    pub every: f64,
    /// 16ths a step takes (`int(round(step16 / m))`, at least 1; haste divides it).
    pub step16: f64,
    /// The longest step.
    pub stride: f64,
    /// A step stops this far short of contact (`w.get('gap', 0.04)`).
    pub gap: f64,
    /// How much of the enemy's motion over the step ahead the step aims ahead by (`w.get('lead', 0.0)`).
    pub lead: f64,
    /// Degrees the step is turned by at most, either way, seeded (`w.get('jitter', 0.0)`).
    pub jitter: f64,
    /// It takes no step while its knock boost is above this (`w.get('stagger', 99.0)`).
    pub stagger: f64,
    /// The quake's starting reach beyond its surface (`blade_len`).
    pub base: f64,
    /// Reach added per landed quake.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// Units a second the quake ring spreads.
    pub speed: f64,
    /// Damage of a quake.
    pub dmg: f64,
    /// Knockback of a quake.
    pub knock: f64,
    /// The quake ring's name (`w.get('name', 'WALK')`).
    pub name: String,
}

/// The keys a walk entry may hold: `kind`, `style` and [`WalkCfg`]'s fields. (`pull_grow`, which `effects_step`
/// reads of the owner of any ring with `grow`, is not one of them: no walk has it, and it needs a `pull`.)
const WALK_KEYS: [&str; 16] = [
    "kind",
    "style",
    "name",
    "every",
    "step16",
    "stride",
    "gap",
    "lead",
    "jitter",
    "stagger",
    "base",
    "grow",
    "max_reach",
    "speed",
    "dmg",
    "knock",
];

/// Reads a `walk` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<WalkCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a walk weapon", &WALK_KEYS)?;
    let name = match w.get("name") {
        None => "WALK".to_string(),
        Some(_) => e.string(w, p, "name")?,
    };
    let cfg = WalkCfg {
        every: e.grid(w, p, "every")?,
        step16: e.f64(w, p, "step16")?,
        stride: e.f64(w, p, "stride")?,
        gap: e.f64_or(w, p, "gap", 0.04)?,
        lead: e.f64_or(w, p, "lead", 0.0)?,
        jitter: e.f64_or(w, p, "jitter", 0.0)?,
        stagger: e.f64_or(w, p, "stagger", 99.0)?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        speed: e.f64(w, p, "speed")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
        name,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(round(4 * every))` and `int(round(step16 / m))` raise when the number is not finite; a jitter that
/// is not finite makes the heading NaN for the rest of the fight.
pub(crate) fn check(e: &Entry, w: &WalkCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    for (key, x) in [("step16", w.step16), ("jitter", w.jitter)] {
        if !x.is_finite() {
            return Err(e.err(&format!("{p}.{key}"), format!("{x:e} is not a finite number")));
        }
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): `base`, the quake's starting reach.
pub fn ball_start(w: &WalkCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float): DOUBLE TIME divides it.
pub(crate) fn main16(w: &WalkCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

#[cfg(test)]
mod tests {
    use crate::roster::{FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn contrabass() -> serde_json::Value {
        let all: serde_json::Value = serde_json::from_str(ROSTER).expect("the roster is JSON");
        let list = all["fighters"].as_array().expect("a list of fighters");
        list.iter()
            .find(|f| f["name"] == "CONTRABASS")
            .expect("in the roster")
            .clone()
    }

    #[test]
    fn the_contrabass_loads_with_the_rosters_numbers() {
        let all = parse_roster(ROSTER).expect("the roster loads");
        let f = all.iter().find(|f| f.name == "CONTRABASS").expect("in the roster");
        let WeaponCfg::Walk(w) = &f.weapon else {
            panic!("the CONTRABASS's weapon is a walk: {:?}", f.weapon)
        };
        assert_eq!((w.every, w.step16, w.stride, w.gap, w.lead), (1.0, 2.0, 0.75, 0.04, 0.6));
        assert_eq!((w.jitter, w.stagger, w.base, w.grow, w.max_reach), (8.0, 0.6, 0.66, 0.05, 1.5));
        assert_eq!((w.speed, w.dmg, w.knock), (9.0, 3.2, 0.2));
        assert_eq!(w.name, "WALKING BASS");
        assert_eq!(f.weapon.ball_start().base, 0.66);
        assert!(f.unlock1.kind.is_ported() && f.unlock2.kind.is_ported() && f.low.kind.is_ported());
    }

    #[test]
    fn the_defaults_are_pythons() {
        let mut v = contrabass();
        let w = v["weapon"].as_object_mut().expect("a table");
        for key in ["gap", "lead", "jitter", "stagger", "name"] {
            w.remove(key);
        }
        let f = FighterCfg::from_json(&v).expect("every option has a default");
        let WeaponCfg::Walk(w) = &f.weapon else {
            panic!("a walk")
        };
        assert_eq!((w.gap, w.lead, w.jitter, w.stagger), (0.04, 0.0, 0.0, 99.0));
        assert_eq!(w.name, "WALK");
    }

    #[test]
    fn what_the_python_cannot_run_is_refused() {
        for (key, value, field) in [
            ("every", serde_json::json!(1e308), "weapon.every"),
            ("step16", serde_json::json!(null), "weapon.step16"),
            ("stride", serde_json::json!("far"), "weapon.stride"),
            ("pull_grow", serde_json::json!(0.1), "weapon.pull_grow"),
        ] {
            let mut v = contrabass();
            v["weapon"][key] = value;
            let err = FighterCfg::from_json(&v).expect_err(key);
            assert_eq!((err.fighter.as_str(), err.field.as_str()), ("CONTRABASS", field), "{err:?}");
        }
        let mut v = contrabass();
        v["weapon"].as_object_mut().expect("a table").remove("max_reach");
        let err = FighterCfg::from_json(&v).expect_err("max_reach is read as w['max_reach']");
        assert_eq!(err.field, "weapon.max_reach");
    }
}
