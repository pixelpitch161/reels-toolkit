//! Weapon kind `anchor`, BASS CLARINET's ENDPIN ANCHOR: its roster settings. `fight.py` 2568 to 2658 (`anchor_setup`,
//! `anchor_plant`, `anchor_release`, `anchor_tick`, `anchor_step`).
//!
//! Every `every` beats on the 16th grid the fighter drives its floor peg into the canvas an orbit ahead of it
//! (`base` to `max_orbit`, `grow` more per landed smash), toward the enemy's spot `lead` x half a whirl ahead, and
//! whirls round it: `turns` turns in `whirl_beats` beats, the radius easing to the orbit over `settle` s. The arm
//! smashes an enemy it sweeps over (`dmg`, `knock`, then `cool` s of rest for that arm); a ghost twin's arm does
//! `twin_dmg` x the damage. Then the peg is pulled and the ball flung off along the tangent with a `release` boost.
//!
//! [`AnchorCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `anchor`. While false the roster reads it as `WeaponCfg::Unsupported("anchor")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// Weapon kind `anchor`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnchorCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// The weapon's name (`cfg['weapon'].get('name', w['kind'].upper())`: what a `copy` event calls it).
    pub name: String,
    /// Beats between two pegs on the 16th grid (`int(round(4 * every))` 16ths; haste divides it). In a roster entry
    /// 4 x `every` is a finite number.
    pub every: f64,
    /// Beats a whirl lasts. Never 0 in a roster entry (the whirl's speed divides by it).
    pub whirl_beats: f64,
    /// Turns round the peg in one whirl.
    pub turns: f64,
    /// The starting orbit (`blade_len`; `anchor_setup` reads `w['base']`).
    pub base: f64,
    /// Orbit added per landed smash of the fighter's own whirl.
    pub grow: f64,
    /// The orbit never grows past this.
    pub max_orbit: f64,
    /// How much of the enemy's motion over half a whirl the peg leads by (`w.get('lead', 0.0)`).
    pub lead: f64,
    /// Seconds the radius takes to ease to the orbit (`w.get('settle', 0.12)`). Never 0 in a roster entry.
    pub settle: f64,
    /// Damage of a smash.
    pub dmg: f64,
    /// Knockback of a smash.
    pub knock: f64,
    /// Seconds an arm rests after its smash (`w.get('cool', 0.3)`).
    pub cool: f64,
    /// The boost the ball is flung off with (`w.get('release', 0.0)`).
    pub release: f64,
    /// The ghost twin's arm does this share of the damage (`w.get('twin_dmg', 0.6)`).
    pub twin_dmg: f64,
}

/// The keys an anchor entry may hold: `kind` and [`AnchorCfg`]'s fields.
const ANCHOR_KEYS: [&str; 16] = [
    "kind",
    "style",
    "name",
    "every",
    "whirl_beats",
    "turns",
    "base",
    "grow",
    "max_orbit",
    "lead",
    "settle",
    "dmg",
    "knock",
    "cool",
    "release",
    "twin_dmg",
];

/// Reads a `anchor` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<AnchorCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "an anchor weapon", &ANCHOR_KEYS)?;
    let cfg = AnchorCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "anchor".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "ANCHOR".to_string(),
        },
        every: e.grid(w, p, "every")?,
        whirl_beats: e.f64(w, p, "whirl_beats")?,
        turns: e.f64(w, p, "turns")?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_orbit: e.f64(w, p, "max_orbit")?,
        lead: e.f64_or(w, p, "lead", 0.0)?,
        settle: e.f64_or(w, p, "settle", 0.12)?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
        cool: e.f64_or(w, p, "cool", 0.3)?,
        release: e.f64_or(w, p, "release", 0.0)?,
        twin_dmg: e.f64_or(w, p, "twin_dmg", 0.6)?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(round(4 * every))` raises when it is not finite; `2 * math.pi / tw` raises `ZeroDivisionError`
/// in the first peg when `whirl_beats` is 0, and `(self.t - An['t0']) / w.get('settle', 0.12)` in the whirl's first
/// step when `settle` is 0.
pub(crate) fn check(e: &Entry, w: &AnchorCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    for (key, x, why) in [
        ("whirl_beats", w.whirl_beats, "the whirl's speed divides by its length"),
        ("settle", w.settle, "the orbit eases in over it"),
    ] {
        if x == 0.0 {
            return Err(e.err(&format!("{p}.{key}"), format!("must not be 0 ({why})")));
        }
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`): its `base`,
/// the rest at Python's defaults.
pub fn ball_start(w: &AnchorCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it.
pub(crate) fn main16(w: &AnchorCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::roster::{AbilityKind, FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn bass_clarinet() -> Json {
        let roster: Json = serde_json::from_str(ROSTER).unwrap();
        let list = roster["fighters"].as_array().unwrap().clone();
        list.into_iter().find(|f| f["name"] == "BASS CLARINET").unwrap()
    }

    fn anchor(c: &Json) -> AnchorCfg {
        match FighterCfg::from_json(c).unwrap().weapon {
            WeaponCfg::Anchor(w) => w,
            other => panic!("BASS CLARINET's weapon is {other:?}"),
        }
    }

    #[test]
    fn the_bass_clarinets_anchor_is_typed() {
        let cfg = FighterCfg::from_json(&bass_clarinet()).unwrap();
        let w = anchor(&bass_clarinet());
        assert_eq!((w.style.as_str(), w.name.as_str()), ("endpin", "ENDPIN ANCHOR"));
        assert_eq!(
            (w.every, w.whirl_beats, w.turns, w.base, w.grow, w.max_orbit),
            (4.0, 2.0, 1.5, 0.95, 0.06, 1.8)
        );
        assert_eq!(
            (w.lead, w.settle, w.dmg, w.knock, w.cool, w.release, w.twin_dmg),
            (0.5, 0.12, 4.5, 1.1, 0.8, 0.8, 0.6)
        );
        assert_eq!(ball_start(&w).base, 0.95);
        assert_eq!(main16(&w), Some(16.0));
        assert_eq!(
            (&cfg.unlock1.kind, &cfg.unlock2.kind, &cfg.low.kind),
            (&AbilityKind::Snip, &AbilityKind::Twin, &AbilityKind::LightsOut)
        );
        cfg.check().unwrap();
        // the whole roster still reads, with BASS CLARINET in it
        let all = parse_roster(ROSTER).unwrap();
        assert!(
            all.iter()
                .any(|f| f.name == "BASS CLARINET" && matches!(f.weapon, WeaponCfg::Anchor(_)))
        );
    }

    #[test]
    fn the_defaults_are_pythons() {
        let mut c = bass_clarinet();
        let w = c["weapon"].as_object_mut().unwrap();
        for k in ["style", "name", "lead", "settle", "cool", "release", "twin_dmg"] {
            w.remove(k);
        }
        let w = anchor(&c);
        assert_eq!((w.style.as_str(), w.name.as_str()), ("anchor", "ANCHOR"));
        assert_eq!(
            (w.lead, w.settle, w.cool, w.release, w.twin_dmg),
            (0.0, 0.12, 0.3, 0.0, 0.6)
        );
    }

    /// What Python raises on (`KeyError` in `anchor_setup` or the first peg, `ZeroDivisionError`, `OverflowError`),
    /// or would silently ignore, is refused when the roster is read, naming the fighter and the field
    /// (`unit_checks/anchor_unit_vectors.py` shows what the kit does with each).
    #[test]
    fn an_anchor_the_engine_cannot_run_is_refused_in_words() {
        for k in [
            "every",
            "whirl_beats",
            "turns",
            "base",
            "grow",
            "max_orbit",
            "dmg",
            "knock",
        ] {
            let mut c = bass_clarinet();
            c["weapon"].as_object_mut().unwrap().remove(k);
            let err = FighterCfg::from_json(&c).unwrap_err();
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str(), err.message.as_str()),
                ("BASS CLARINET", &*format!("weapon.{k}"), "missing"),
                "{k}"
            );
            let mut c = bass_clarinet();
            c["weapon"][k] = json!("far");
            let err = FighterCfg::from_json(&c).unwrap_err();
            assert_eq!(
                (err.field.as_str(), err.message.as_str()),
                (&*format!("weapon.{k}"), "must be a number")
            );
        }
        let mut c = bass_clarinet();
        c["weapon"]["pull"] = json!(1.0);
        let err = FighterCfg::from_json(&c).unwrap_err();
        assert_eq!(err.field, "weapon.pull");
        assert!(
            err.message.contains("not a setting of an anchor weapon"),
            "{}",
            err.message
        );
        // 4 x every must be finite (`int(round(4 * every))` raises OverflowError otherwise)
        let mut c = bass_clarinet();
        c["weapon"]["every"] = json!(1e308);
        let err = FighterCfg::from_json(&c).unwrap_err();
        assert_eq!(err.field, "weapon.every");
        for (k, why) in [
            ("whirl_beats", "must not be 0 (the whirl's speed divides by its length)"),
            ("settle", "must not be 0 (the orbit eases in over it)"),
        ] {
            let mut c = bass_clarinet();
            c["weapon"][k] = json!(0);
            let err = FighterCfg::from_json(&c).unwrap_err();
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str(), err.message.as_str()),
                ("BASS CLARINET", &*format!("weapon.{k}"), why)
            );
        }
        // typed settings changed in code are checked again
        let mut cfg = FighterCfg::from_json(&bass_clarinet()).unwrap();
        if let WeaponCfg::Anchor(w) = &mut cfg.weapon {
            w.settle = 0.0;
        }
        assert_eq!(cfg.check().unwrap_err().field, "weapon.settle");
    }
}
