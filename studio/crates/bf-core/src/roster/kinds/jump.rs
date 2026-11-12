//! Weapon kind `jump`, CLARINET's TWELFTH JUMP: its roster settings. `fight.py` 2338 to 2424 (`jump_setup`, `_jump`,
//! `jump_tick`), and the ability `rhapsody` (`trigger` 2930 to 2932; its chain runs in `jump_tick`).
//!
//! Every `every` beats on the 16th grid the ball vanishes and reappears `land` past the enemy, turned a seeded
//! `jitter` degrees off the line, at most its reach (`base` to `max_reach`, `grow` more per landed cut) from where it
//! was. The zip line between its two spots cuts the enemy when it passes within a ball radius plus `cut` of it.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `jump`. While false the roster reads it as `WeaponCfg::Unsupported("jump")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// Weapon kind `jump`'s typed settings (`fight.py` `jump_setup`, `_jump`, `jump_tick`). Fields without a default are
/// read as `w[name]`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JumpCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// Beats between jumps on the 16th grid (`int(round(4 * every))` 16ths; haste divides it). In a roster entry
    /// 4 x `every` is a finite number.
    pub every: f64,
    /// The starting reach (`blade_len`; `jump_setup` reads `w['base']`).
    pub base: f64,
    /// Reach added per landed cut.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// How far beyond its target the ball lands.
    pub land: f64,
    /// The landing line is turned a seeded angle of at most this many degrees either way (`.get('jitter', 0.0)`).
    pub jitter: f64,
    /// 16ths before a jump at which its target locks (`int(w.get('lock16', 1))`, the whole part).
    pub lock16: i64,
    /// How much of the enemy's motion until the jump the locked target leads by (`.get('lead', 0.0)`).
    pub lead: f64,
    /// The zip line cuts an enemy within a ball radius plus this of it.
    pub cut: f64,
    /// Damage per cut.
    pub dmg: f64,
    /// Knockback per cut.
    pub knock: f64,
}

/// The keys a jump entry may hold: `kind`, `style` and [`JumpCfg`]'s fields.
const JUMP_KEYS: [&str; 13] = [
    "kind",
    "style",
    "every",
    "base",
    "grow",
    "max_reach",
    "land",
    "jitter",
    "lock16",
    "lead",
    "cut",
    "dmg",
    "knock",
];

/// Reads a `jump` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<JumpCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a jump weapon", &JUMP_KEYS)?;
    Ok(JumpCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "jump".to_string(),
        },
        every: e.grid(w, p, "every")?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        land: e.f64(w, p, "land")?,
        jitter: e.f64_or(w, p, "jitter", 0.0)?,
        // `int(w.get('lock16', 1))`: `as` truncates toward zero, as `int()` does
        lock16: e.f64_or(w, p, "lock16", 1.0)? as i64,
        lead: e.f64_or(w, p, "lead", 0.0)?,
        cut: e.f64(w, p, "cut")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &JumpCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`): its `base`,
/// the rest at Python's defaults.
pub fn ball_start(w: &JumpCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it.
pub(crate) fn main16(w: &JumpCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::roster::{FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn clarinet() -> Json {
        let roster: Json = serde_json::from_str(ROSTER).unwrap();
        let list = roster["fighters"].as_array().unwrap().clone();
        list.into_iter().find(|f| f["name"] == "CLARINET").unwrap()
    }

    fn jump(c: &Json) -> JumpCfg {
        match FighterCfg::from_json(c).unwrap().weapon {
            WeaponCfg::Jump(w) => w,
            other => panic!("CLARINET's weapon is {other:?}"),
        }
    }

    #[test]
    fn the_clarinets_jump_is_typed() {
        let cfg = FighterCfg::from_json(&clarinet()).unwrap();
        let w = jump(&clarinet());
        assert_eq!(w.style, "zip");
        assert_eq!(
            (w.every, w.base, w.grow, w.max_reach, w.land, w.jitter, w.lock16, w.lead),
            (1.0, 2.6, 0.06, 4.2, 1.5, 40.0, 2, 0.0)
        );
        assert_eq!((w.cut, w.dmg, w.knock), (0.0, 3.6, 0.6));
        assert_eq!(ball_start(&w).base, 2.6);
        assert_eq!(main16(&w), Some(4.0));
        cfg.check().unwrap();
        // the whole roster still reads, with CLARINET in it
        let all = parse_roster(ROSTER).unwrap();
        assert!(
            all.iter()
                .any(|f| f.name == "CLARINET" && matches!(f.weapon, WeaponCfg::Jump(_)))
        );
    }

    #[test]
    fn the_defaults_are_pythons() {
        let mut c = clarinet();
        let w = c["weapon"].as_object_mut().unwrap();
        for k in ["style", "jitter", "lock16", "lead"] {
            w.remove(k);
        }
        let w = jump(&c);
        assert_eq!((w.style.as_str(), w.jitter, w.lock16, w.lead), ("jump", 0.0, 1, 0.0));
        // int(2.9) is 2, int(-1.5) is -1
        for (given, read) in [(json!(2.9), 2), (json!(-1.5), -1)] {
            let mut c = clarinet();
            c["weapon"]["lock16"] = given;
            assert_eq!(jump(&c).lock16, read);
        }
    }

    /// What Python raises `KeyError` on (in `jump_setup` or the first jump), or would silently ignore, is refused when
    /// the roster is read, naming the fighter and the field.
    #[test]
    fn a_jump_the_engine_cannot_run_is_refused_in_words() {
        for k in ["every", "base", "grow", "max_reach", "land", "cut", "dmg", "knock"] {
            let mut c = clarinet();
            c["weapon"].as_object_mut().unwrap().remove(k);
            let err = FighterCfg::from_json(&c).unwrap_err();
            assert_eq!(
                (err.fighter.as_str(), err.field.as_str(), err.message.as_str()),
                ("CLARINET", &*format!("weapon.{k}"), "missing"),
                "{k}"
            );
            let mut c = clarinet();
            c["weapon"][k] = json!("far");
            let err = FighterCfg::from_json(&c).unwrap_err();
            assert_eq!(
                (err.field.as_str(), err.message.as_str()),
                (&*format!("weapon.{k}"), "must be a number")
            );
        }
        let mut c = clarinet();
        c["weapon"]["pull"] = json!(1.0);
        let err = FighterCfg::from_json(&c).unwrap_err();
        assert_eq!(err.field, "weapon.pull");
        assert!(
            err.message.contains("not a setting of a jump weapon"),
            "{}",
            err.message
        );
        // 4 x every must be finite (`int(round(4 * every))` raises OverflowError otherwise)
        let mut c = clarinet();
        c["weapon"]["every"] = json!(1e308);
        let err = FighterCfg::from_json(&c).unwrap_err();
        assert_eq!(err.field, "weapon.every");
        // and again on typed settings changed in code
        let mut cfg = FighterCfg::from_json(&clarinet()).unwrap();
        if let WeaponCfg::Jump(w) = &mut cfg.weapon {
            w.every = f64::INFINITY;
        }
        assert_eq!(cfg.check().unwrap_err().field, "weapon.every");
    }
}
