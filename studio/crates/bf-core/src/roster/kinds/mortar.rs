//! Weapon kind `mortar`, EUPHONIUM's BELL MORTAR: its roster settings. `fight.py` 2659 to 2766 (`mortar_setup`,
//! `predict`, `_shell`, `mortar_fire`, `mortar_tick`, `start_barrage`, `shells_step`), the shells a lance swats
//! (`shell_height`, `swat_shells` 1243 to 1271), and the ability `barrage` (`trigger` 2900 to 2901).
//!
//! Every `every` beats on the 16th grid (haste divides it) the upright bell lobs a salvo of `int(n)` shells (`n0` at
//! the start, `grow` more per salvo that lands, up to `n_max`; a twin doubles the salvo). They land `flight_beats`
//! beats later: the first on the enemy's predicted spot (its motion x `lead`), the rest on a ring of `spread` round
//! it. A landing splashes `splash` (+ 0.6 of a ball radius): `dmg`, and a knock of `knock` away from the impact.
//!
//! [`MortarCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `mortar`. While false the roster reads it as `WeaponCfg::Unsupported("mortar")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// Weapon kind `mortar`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MortarCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// The weapon's name (`cfg['weapon'].get('name', w['kind'].upper())`: what a `copy` event calls it).
    pub name: String,
    /// Beats between two salvos on the 16th grid (`int(round(4 * every))` 16ths; haste divides it). In a roster entry
    /// 4 x `every` is a finite number.
    pub every: f64,
    /// Beats a shell flies.
    pub flight_beats: f64,
    /// Shells in the first salvo (`n_vol` starts at `float(w['n0'])`).
    pub n0: f64,
    /// Shells added per salvo that lands.
    pub grow: f64,
    /// The salvo never grows past this.
    pub n_max: f64,
    /// The radius of the ring the other shells of a salvo land on round the first.
    pub spread: f64,
    /// How much of the enemy's motion over the flight the aim leads by (`w.get('lead', 1.0)`).
    pub lead: f64,
    /// The splash radius of a landing (plus 0.6 of a ball radius).
    pub splash: f64,
    /// Damage of a landing.
    pub dmg: f64,
    /// Knockback of a landing.
    pub knock: f64,
}

/// The keys a mortar entry may hold: `kind` and [`MortarCfg`]'s fields.
const MORTAR_KEYS: [&str; 13] = [
    "kind",
    "style",
    "name",
    "every",
    "flight_beats",
    "n0",
    "grow",
    "n_max",
    "spread",
    "lead",
    "splash",
    "dmg",
    "knock",
];

/// Reads a `mortar` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<MortarCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a mortar weapon", &MORTAR_KEYS)?;
    let cfg = MortarCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "mortar".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "MORTAR".to_string(),
        },
        every: e.grid(w, p, "every")?,
        flight_beats: e.f64(w, p, "flight_beats")?,
        n0: e.f64(w, p, "n0")?,
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        spread: e.f64(w, p, "spread")?,
        lead: e.f64_or(w, p, "lead", 1.0)?,
        splash: e.f64(w, p, "splash")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(round(4 * every))` raises when it is not finite, and `int(B.n_vol + 1e-9)` when the volley is not
/// (`n0`, and `grow` and `n_max`, which it grows by and to).
pub(crate) fn check(e: &Entry, w: &MortarCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    for (key, x) in [("n0", w.n0), ("grow", w.grow), ("n_max", w.n_max)] {
        if !x.is_finite() {
            let why = "must be a finite number (the volley is a whole number of shells)";
            return Err(e.err(&format!("{p}.{key}"), why));
        }
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`): its `n0`, the
/// rest at Python's defaults.
pub fn ball_start(w: &MortarCfg) -> BallStart {
    BallStart {
        n0: w.n0,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it.
pub(crate) fn main16(w: &MortarCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::roster::{AbilityKind, FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn euphonium() -> Json {
        let roster: Json = serde_json::from_str(ROSTER).unwrap();
        let list = roster["fighters"].as_array().unwrap().clone();
        list.into_iter().find(|f| f["name"] == "EUPHONIUM").unwrap()
    }

    fn mortar(c: &Json) -> MortarCfg {
        match FighterCfg::from_json(c).unwrap().weapon {
            WeaponCfg::Mortar(w) => w,
            other => panic!("EUPHONIUM's weapon is {other:?}"),
        }
    }

    #[test]
    fn the_euphoniums_mortar_is_typed() {
        let cfg = FighterCfg::from_json(&euphonium()).unwrap();
        let w = mortar(&euphonium());
        assert_eq!((w.style.as_str(), w.name.as_str()), ("shells", "BELL MORTAR"));
        assert_eq!(
            (w.every, w.flight_beats, w.n0, w.grow, w.n_max, w.spread),
            (2.0, 2.0, 1.0, 0.5, 6.0, 0.75)
        );
        assert_eq!((w.lead, w.splash, w.dmg, w.knock), (1.0, 0.6, 2.6, 0.7));
        assert_eq!(ball_start(&w).n0, 1.0);
        assert_eq!(main16(&w), Some(8.0));
        assert_eq!(
            (&cfg.unlock1.kind, &cfg.unlock2.kind, &cfg.low.kind),
            (&AbilityKind::Haste, &AbilityKind::Copy, &AbilityKind::Barrage)
        );
        cfg.check().unwrap();
        let all = parse_roster(ROSTER).unwrap();
        assert!(
            all.iter()
                .any(|f| f.name == "EUPHONIUM" && matches!(f.weapon, WeaponCfg::Mortar(_)))
        );
    }

    #[test]
    fn the_defaults_are_pythons() {
        let mut c = euphonium();
        let w = c["weapon"].as_object_mut().unwrap();
        for k in ["style", "name", "lead"] {
            w.remove(k);
        }
        let w = mortar(&c);
        assert_eq!((w.style.as_str(), w.name.as_str(), w.lead), ("mortar", "MORTAR", 1.0));
    }

    /// What Python raises on (`KeyError` at the first salvo or landing, `OverflowError`) is refused when the roster is
    /// read, naming the fighter and the field.
    #[test]
    fn a_mortar_the_engine_cannot_run_is_refused_in_words() {
        for k in [
            "every",
            "flight_beats",
            "n0",
            "grow",
            "n_max",
            "spread",
            "splash",
            "dmg",
            "knock",
        ] {
            let mut c = euphonium();
            c["weapon"].as_object_mut().unwrap().remove(k);
            let err = FighterCfg::from_json(&c).unwrap_err();
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str(), err.message.as_str()),
                ("EUPHONIUM", &*format!("weapon.{k}"), "missing"),
                "{k}"
            );
            let mut c = euphonium();
            c["weapon"][k] = json!("far");
            let err = FighterCfg::from_json(&c).unwrap_err();
            assert_eq!(
                (err.field.as_str(), err.message.as_str()),
                (&*format!("weapon.{k}"), "must be a number")
            );
        }
        let mut c = euphonium();
        c["weapon"]["calibre"] = json!(1.0);
        let err = FighterCfg::from_json(&c).unwrap_err();
        assert_eq!(err.field, "weapon.calibre");
        assert!(
            err.message.contains("not a setting of a mortar weapon"),
            "{}",
            err.message
        );
        let mut c = euphonium();
        c["weapon"]["every"] = json!(1e308);
        let err = FighterCfg::from_json(&c).unwrap_err();
        assert_eq!(err.field, "weapon.every");
    }
}
