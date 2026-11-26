//! Weapon kind `roll`, PIANO's PIANO ROLL: its roster settings. `fight.py` 5164 to 5283 (`ROLL_DEFAULT`, `roll_setup`,
//! `roll_lane`, `roll_tick`, `fire_row`, `start_runoff`, `hold_step`), the projectiles (`spawn_key` 370 to 378, their
//! flight and hits in `move` 3235 to 3278) and their `K` rows in `snap` (5509 to 5514).
//!
//! A player-piano roll lies along the side of the ring through the fighter's own corner. On the 8th grid the row
//! under the read bar fires one key-shaped shot per hole, from the hole's pitch lane across the ring, turned toward
//! the enemy by `aim`. A row fires its first `int(n_vol)` holes: `n0` at the start, `grow` more per landed row, up to
//! `n_max`.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `roll`. While false the roster reads it as `WeaponCfg::Unsupported("roll")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// Weapon kind `roll`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct RollCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// Holes a row fires at the start (`float(w.get('n0', 0))`, the ball's `n_vol`).
    pub n0: f64,
    /// Holes added per landed row.
    pub grow: f64,
    /// A row never fires more holes than this.
    pub n_max: f64,
    /// Units a second a shot flies.
    pub speed: f64,
    /// How far a shot turns from straight across toward the enemy (0 = straight across, 1 = all at the enemy).
    pub aim: f64,
    /// Damage per shot.
    pub dmg: f64,
    /// How far in from the posts the lanes end (`w.get('margin', 0.45)`).
    pub margin: f64,
    /// The roll's pitch range, lowest and highest note (`w.get('reg', cfg['register'])`: `None` reads the fighter's
    /// register).
    pub reg: Option<(i32, i32)>,
}

/// The keys a roll entry may hold: `kind`, `style`, `name` (the weapon's name, which only the card shows) and
/// [`RollCfg`]'s fields.
const ROLL_KEYS: [&str; 11] = [
    "kind", "style", "name", "n0", "grow", "n_max", "speed", "aim", "dmg", "margin", "reg",
];

/// Reads a `roll` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<RollCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a roll weapon", &ROLL_KEYS)?;
    let reg = match w.get("reg") {
        None => None,
        Some(v) => {
            let bad = || e.err("weapon.reg", "must be a list of 2 notes");
            let list = v.as_array().filter(|l| l.len() == 2).ok_or_else(bad)?;
            let note = |j: &Json| e.int(j, "weapon.reg").and_then(|n| i32::try_from(n).map_err(|_| bad()));
            Some((note(&list[0])?, note(&list[1])?))
        }
    };
    let cfg = RollCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "roll".to_string(),
        },
        n0: e.f64_or(w, p, "n0", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        speed: e.f64(w, p, "speed")?,
        aim: e.f64(w, p, "aim")?,
        dmg: e.f64(w, p, "dmg")?,
        margin: e.f64_or(w, p, "margin", 0.45)?,
        reg,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): the hole count is a number
/// `int()` can take (Python raises on `int(nan)` and `int(inf)` in the first row, where the engine would fire a row
/// of its own size).
pub(crate) fn check(e: &Entry, w: &RollCfg) -> Result<(), RosterError> {
    for (key, x) in [("n0", w.n0), ("grow", w.grow), ("n_max", w.n_max)] {
        if !x.is_finite() {
            return Err(e.err(
                &format!("weapon.{key}"),
                format!("{x} is not a finite number (a row fires a whole number of holes)"),
            ));
        }
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`): the roll has
/// only `n0`.
pub fn ball_start(w: &RollCfg) -> BallStart {
    BallStart {
        n0: w.n0,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. The roll runs on the 8th grid whatever a haste says: `None`.
pub(crate) fn main16(_w: &RollCfg) -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use serde_json::{Value as Json, json};

    use crate::roster::{AbilityKind, FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn piano() -> FighterCfg {
        let all = parse_roster(ROSTER).expect("the roster loads");
        all.into_iter().find(|f| f.name == "PIANO").expect("in the roster")
    }

    /// PIANO's entry with one weapon key removed, or set.
    fn weapon_with(key: &str, value: Option<Json>) -> Json {
        let mut raw = piano().raw;
        let w = raw["weapon"].as_object_mut().expect("a weapon");
        match value {
            Some(v) => _ = w.insert(key.into(), v),
            None => _ = w.remove(key),
        }
        raw
    }

    fn roll(f: &FighterCfg) -> &super::RollCfg {
        match &f.weapon {
            WeaponCfg::Roll(w) => w,
            other => panic!("{other:?}"),
        }
    }

    /// The roster's numbers (`fighters.py`: PIANO v2, #45), and the three abilities typed with theirs.
    #[test]
    fn piano_is_a_roll_with_its_roster_numbers() {
        let f = piano();
        let w = roll(&f);
        assert_eq!(
            (w.n0, w.grow, w.n_max, w.speed, w.aim, w.dmg, w.margin),
            (1.0, 0.25, 4.0, 5.2, 0.55, 0.5, 0.45)
        );
        assert_eq!(
            (w.reg, w.style.as_str(), f.register),
            (Some((48, 84)), "roll", (48, 84))
        );
        assert_eq!(f.weapon.kind(), "roll");
        assert_eq!(f.weapon.ball_start().n0, 1.0);
        assert_eq!(
            [&f.unlock1.kind, &f.unlock2.kind, &f.low.kind],
            [&AbilityKind::Sustain, &AbilityKind::Arc, &AbilityKind::Runoff]
        );
        assert_eq!(
            (
                f.unlock1.get("dur_beats"),
                f.unlock1.get("out16"),
                f.unlock1.get("step16")
            ),
            (Some(4.0), Some(2.0), Some(1.0))
        );
        assert_eq!(
            (f.unlock2.get("n"), f.unlock2.get("sweep"), f.unlock2.get("beats")),
            (Some(15.0), Some(150.0), Some(1.0))
        );
        assert_eq!(
            (f.low.get("n"), f.low.get("margin"), f.low.get("reload_beats")),
            (Some(40.0), Some(0.25), Some(4.0))
        );
    }

    /// `w.get(name, default)`: `n0` 0, `margin` 0.45, `reg` the fighter's register; `w[name]` for the rest.
    #[test]
    fn missing_fields_take_fight_pys_defaults_or_are_named() {
        let f = FighterCfg::from_json(&weapon_with("margin", None)).expect("loads");
        assert_eq!(roll(&f).margin, 0.45);
        let f = FighterCfg::from_json(&weapon_with("reg", None)).expect("loads");
        assert_eq!(roll(&f).reg, None);
        let f = FighterCfg::from_json(&weapon_with("n0", None)).expect("loads");
        assert_eq!((roll(&f).n0, f.weapon.ball_start().n0), (0.0, 0.0));
        for key in ["grow", "n_max", "speed", "aim", "dmg"] {
            assert_eq!(
                FighterCfg::from_json(&weapon_with(key, None)).unwrap_err().to_string(),
                format!("roster: PIANO: weapon.{key}: missing")
            );
        }
        assert_eq!(
            FighterCfg::from_json(&weapon_with("reg", Some(json!([48]))))
                .unwrap_err()
                .to_string(),
            "roster: PIANO: weapon.reg: must be a list of 2 notes"
        );
        assert_eq!(
            FighterCfg::from_json(&weapon_with("reg", Some(json!([48.5, 84]))))
                .unwrap_err()
                .to_string(),
            "roster: PIANO: weapon.reg: must be a whole number"
        );
        // a key the engine would ignore (`fight.py` reads `curve`, `bounce`, `hit_r` on a volley, not on a roll)
        assert_eq!(
            FighterCfg::from_json(&weapon_with("bounce", Some(json!(2))))
                .unwrap_err()
                .to_string(),
            "roster: PIANO: weapon.bounce: not a setting of a roll weapon (the engine would ignore it)"
        );
    }

    /// An ability entry with a key its kind does not read is not that kind (never typed with the key dropped), and
    /// one without a number its kind reads as `ab[name]` is named.
    #[test]
    fn the_three_abilities_need_their_numbers_and_hold_nothing_else() {
        for (slot, kind, missing) in [
            ("unlock1", "sustain", vec!["dur_beats"]),
            ("unlock2", "arc", vec!["beats", "sweep", "n", "speed", "dmg"]),
            ("low", "runoff", vec!["n", "speed", "dmg"]),
        ] {
            for key in missing {
                let mut raw = piano().raw;
                raw["abilities"][slot].as_object_mut().expect("a table").remove(key);
                let field = if key == "dur_beats" { "dur or dur_beats" } else { key };
                assert_eq!(
                    FighterCfg::from_json(&raw).unwrap_err().to_string(),
                    format!("roster: PIANO: abilities.{slot}.{field}: missing")
                );
            }
            let mut raw = piano().raw;
            raw["abilities"][slot]["curve"] = json!(true);
            let f = FighterCfg::from_json(&raw).expect("loads");
            let ab = match slot {
                "unlock1" => &f.unlock1,
                "unlock2" => &f.unlock2,
                _ => &f.low,
            };
            assert_eq!(ab.kind, AbilityKind::Unsupported(format!("{kind}+curve")));
        }
    }

    /// `FighterCfg::check` on settings changed in code: a hole count or a shot count that is not a finite number.
    #[test]
    fn check_refuses_counts_that_are_not_numbers() {
        let mut f = piano();
        let WeaponCfg::Roll(w) = &mut f.weapon else {
            panic!("a roll");
        };
        w.n_max = f64::INFINITY;
        assert_eq!(
            f.check().unwrap_err().to_string(),
            "roster: PIANO: weapon.n_max: inf is not a finite number (a row fires a whole number of holes)"
        );
        let mut f = piano();
        f.unlock2.params.insert("n".into(), f64::INFINITY);
        assert_eq!(
            f.check().unwrap_err().to_string(),
            "roster: PIANO: abilities.unlock2.n: inf is not a finite number (an arc throws that many shots)"
        );
        let mut f = piano();
        f.low.params.insert("n".into(), f64::NAN);
        assert_eq!(
            f.check().unwrap_err().to_string(),
            "roster: PIANO: abilities.low.n: NaN is not a finite number (the roll fires that many shots at once)"
        );
    }
}
