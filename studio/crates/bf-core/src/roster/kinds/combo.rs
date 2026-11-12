//! Weapon kind `combo`, TRUMPET's valve combo: its roster settings. `fight.py` 898 to 966 (`combo_setup`,
//! `combo_beat`, `combo_step`).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `combo`. While false the roster reads it as `WeaponCfg::Unsupported("combo")`,
/// which the engine refuses in words.
pub const PORTED: bool = true;

/// Weapon kind `combo`'s typed settings (`fight.py` `combo_beat`, `combo_step`, `Ball.__init__`). Fields without a
/// default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct ComboCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// Beats between combos (`max(1, int(round(every / ab_mult)))` beats; haste divides it).
    pub every: f64,
    /// Punches in a combo (`int(w.get('punches', 3))`), from 0 to [`MAX_PUNCHES`] in a roster entry.
    pub punches: i64,
    /// Beats between two punches of a combo (`.get('gap', 0.5)`; haste divides it).
    pub gap: f64,
    /// How far apart the pistons sit across the aim, per piston (`.get('spread', 0.15)`).
    pub spread: f64,
    /// Radians a second the valves turn toward the enemy (`.get('aim_rate', 8.0)`).
    pub aim_rate: f64,
    /// A piston head's radius.
    pub head_r: f64,
    /// The starting reach (`blade_len`; `.get('base', 0.0)`).
    pub base: f64,
    /// Reach added per landed punch.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_len: f64,
    /// Seconds a piston takes to shoot out; never 0 in a roster entry.
    pub t_out: f64,
    /// Seconds it holds out.
    pub t_hold: f64,
    /// Seconds it takes to pull back.
    pub t_back: f64,
    /// Damage per punch.
    pub dmg: f64,
    /// Damage of the last punch of a combo.
    pub dmg3: f64,
    /// Knockback per punch.
    pub knock: f64,
    /// Knockback of the last punch of a combo.
    pub knock3: f64,
}

/// The most punches a combo may have in a roster entry. The limit is the Studio's own, stricter than the kit on
/// purpose (Python takes any integer and queues that many punches): a roster is Studio-owned data checked when it is
/// loaded, and a count outside 0 to 1,000 is a typing mistake the app names in words. The TRUMPET has 3.
pub const MAX_PUNCHES: i64 = 1_000;

/// The keys a combo entry may hold: `kind`, `style` and [`ComboCfg`]'s fields.
const COMBO_KEYS: [&str; 18] = [
    "kind", "style", "every", "punches", "gap", "spread", "aim_rate", "head_r", "base", "grow", "max_len", "t_out",
    "t_hold", "t_back", "dmg", "dmg3", "knock", "knock3",
];

/// A combo's punch count, from 0 to [`MAX_PUNCHES`].
fn punches_value(e: &Entry, n: i64) -> Result<i64, RosterError> {
    if !(0..=MAX_PUNCHES).contains(&n) {
        return Err(e.err(
            "weapon.punches",
            format!("{n} is not a number of punches (0 to {MAX_PUNCHES})"),
        ));
    }
    Ok(n)
}

/// A piston's `t_out`: `combo_step` divides by it (`u / t_out`), so Python raises `ZeroDivisionError` on 0.
fn t_out_value(e: &Entry, x: f64) -> Result<f64, RosterError> {
    if x == 0.0 {
        return Err(e.err(
            "weapon.t_out",
            "0 seconds: a piston needs time to shoot out (the fight divides by it)",
        ));
    }
    Ok(x)
}

/// Reads a `combo` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<ComboCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a combo weapon", &COMBO_KEYS)?;
    Ok(ComboCfg {
        style: match w.get("style") {
            Some(Json::String(s)) => s.clone(),
            Some(_) => return Err(e.err("weapon.style", "must be a text")),
            None => "combo".to_string(),
        },
        every: e.grid(w, p, "every")?,
        punches: match w.get("punches") {
            // `int(x)` truncates toward zero
            Some(v) => punches_value(e, e.number(v, "weapon.punches")? as i64)?,
            None => 3,
        },
        gap: e.f64_or(w, p, "gap", 0.5)?,
        spread: e.f64_or(w, p, "spread", 0.15)?,
        aim_rate: e.f64_or(w, p, "aim_rate", 8.0)?,
        head_r: e.f64(w, p, "head_r")?,
        base: e.f64_or(w, p, "base", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        max_len: e.f64(w, p, "max_len")?,
        t_out: t_out_value(e, e.f64(w, p, "t_out")?)?,
        t_hold: e.f64(w, p, "t_hold")?,
        t_back: e.f64(w, p, "t_back")?,
        dmg: e.f64(w, p, "dmg")?,
        dmg3: e.f64(w, p, "dmg3")?,
        knock: e.f64(w, p, "knock")?,
        knock3: e.f64(w, p, "knock3")?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &ComboCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    punches_value(e, w.punches)?;
    t_out_value(e, w.t_out)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `base`, the starting reach.
pub fn ball_start(w: &ComboCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The number a haste divides, as a float: the combo's grid is in beats (`int(round(every / ab_mult(k)))`, `fight.py`
/// 910), not in 16ths, so this is `every` itself; the roster then refuses a haste factor that makes the quotient
/// not finite, where Python raises.
pub(crate) fn main16(w: &ComboCfg) -> Option<f64> {
    Some(w.every)
}

#[cfg(test)]
mod tests {
    use serde_json::Value as Json;

    use super::*;
    use crate::roster::{FighterCfg, WeaponCfg, parse_roster};

    const ROSTER: &str = include_str!("../../../../bf-content/data/roster.json");

    fn trumpet() -> FighterCfg {
        let all = parse_roster(ROSTER).expect("the roster loads");
        all.into_iter().find(|f| f.name == "TRUMPET").expect("in the roster")
    }

    /// The TRUMPET's entry with `key` of its weapon removed (or set to `value`).
    fn weapon_with(key: &str, value: Option<Json>) -> Result<FighterCfg, RosterError> {
        let mut raw = trumpet().raw;
        let w = raw["weapon"].as_object_mut().expect("a table");
        match value {
            Some(v) => _ = w.insert(key.into(), v),
            None => _ = w.remove(key),
        }
        FighterCfg::from_json(&raw)
    }

    fn combo(f: &FighterCfg) -> &ComboCfg {
        match &f.weapon {
            WeaponCfg::Combo(w) => w,
            other => panic!("not a combo: {other:?}"),
        }
    }

    #[test]
    fn trumpet_is_a_combo_with_its_roster_numbers() {
        let f = trumpet();
        let want = ComboCfg {
            style: "pistons".into(),
            every: 2.0,
            punches: 3,
            gap: 0.5,
            spread: 0.15,
            aim_rate: 8.0,
            head_r: 0.1,
            base: 0.6,
            grow: 0.05,
            max_len: 1.7,
            t_out: 0.05,
            t_hold: 0.04,
            t_back: 0.09,
            dmg: 1.2,
            dmg3: 2.0,
            knock: 0.35,
            knock3: 1.0,
        };
        assert_eq!(combo(&f), &want);
        assert_eq!(f.weapon.kind(), "combo");
        assert_eq!(
            f.weapon.ball_start(),
            BallStart {
                base: 0.6,
                ..BallStart::default()
            }
        );
        assert_eq!(main16(&want), Some(2.0));
        f.check().expect("its own entry passes the check");
    }

    #[test]
    fn the_options_take_the_fight_py_defaults() {
        for (key, read) in [("gap", 0.5), ("spread", 0.15), ("aim_rate", 8.0), ("base", 0.0)] {
            let f = weapon_with(key, None).expect("loads");
            let w = combo(&f);
            let got = match key {
                "gap" => w.gap,
                "spread" => w.spread,
                "aim_rate" => w.aim_rate,
                _ => w.base,
            };
            assert_eq!(got, read, "{key}");
        }
        assert_eq!(combo(&weapon_with("punches", None).expect("loads")).punches, 3);
        assert_eq!(combo(&weapon_with("style", None).expect("loads")).style, "combo");
        // int(punches): the whole part
        assert_eq!(
            combo(&weapon_with("punches", Some(Json::from(2.9))).expect("loads")).punches,
            2
        );
    }

    #[test]
    fn what_the_fight_cannot_run_is_refused_by_name() {
        let err = |key: &str, value: Option<Json>| weapon_with(key, value).unwrap_err().to_string();
        for key in [
            "every", "head_r", "grow", "max_len", "t_out", "t_hold", "t_back", "dmg", "dmg3", "knock", "knock3",
        ] {
            assert_eq!(
                err(key, None),
                format!("roster: TRUMPET: weapon.{key}: missing"),
                "{key}"
            );
        }
        assert_eq!(
            err("t_out", Some(Json::from(0.0))),
            "roster: TRUMPET: weapon.t_out: 0 seconds: a piston needs time to shoot out (the fight divides by it)"
        );
        assert_eq!(
            err("punches", Some(Json::from(-1))),
            "roster: TRUMPET: weapon.punches: -1 is not a number of punches (0 to 1000)"
        );
        assert_eq!(
            err("punches", Some(Json::from(1e9))),
            "roster: TRUMPET: weapon.punches: 1000000000 is not a number of punches (0 to 1000)"
        );
        assert_eq!(
            err("pull", Some(Json::from(0.5))),
            "roster: TRUMPET: weapon.pull: not a setting of a combo weapon (the engine would ignore it)"
        );
        assert_eq!(
            err("style", Some(Json::from(3))),
            "roster: TRUMPET: weapon.style: must be a text"
        );
        // typed settings changed in code are checked again
        let mut f = trumpet();
        if let WeaponCfg::Combo(w) = &mut f.weapon {
            w.t_out = 0.0;
        }
        assert!(f.check().unwrap_err().to_string().contains("weapon.t_out"));
    }
}
