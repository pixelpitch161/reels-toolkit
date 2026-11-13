//! Weapon kind `rests`, VIOLA's COUNTING RESTS: its roster settings. `fight.py` 5021 to 5107 (`rests_setup`,
//! `fire_note`, `rests_tick`, `_crush`), 5137 to 5181 (`notes_step`), the resting `guard` in `damage` (342 to 343), and
//! the abilities `double_stop` (5108 to 5117) and `melody` (`trigger` 2923 to 2927), which run in this kind's code.
//!
//! On every beat of the 16th grid the fighter rests (`int(every) - 1` beats of rests, counted, taking x `guard` damage)
//! and then fires a whole note: a wheel of radius `base` (+ `grow` per landed note up to `max_reach`) aimed ahead of the
//! enemy (its motion x `lead` over the note's travel, `jitter` degrees of seeded wobble), rolling at `roll` for `hold`
//! beats. It crushes the enemy once on contact (`dmg`, + `dmg_grow` per landed note up to `dmg_max`; `knock`) and
//! flattens the enemy's projectiles it rolls over.
//!
//! [`RestsCfg`] holds every field `fight.py` reads: `w[name]`, and `w.get(name, default)` with its default.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `rests`. While false the roster reads it as
/// `WeaponCfg::Unsupported("rests")`, which the engine refuses in words.
pub const PORTED: bool = true;

/// Weapon kind `rests`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RestsCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// The weapon's name (`cfg['weapon'].get('name', w['kind'].upper())`).
    pub name: String,
    /// Beats a cycle lasts: `int(every) - 1` beats of rests, then a note (`cyc = int(B.w['every'])`).
    pub every: f64,
    /// The damage a resting fighter takes, as a factor (`V.w.get('guard', 1.0)`, `damage` 342 to 343).
    pub guard: f64,
    /// The first note's radius (`blade_len` starts at `float(w['base'])`).
    pub base: f64,
    /// The radius a landed note adds to the next.
    pub grow: f64,
    /// The note radius never grows past this.
    pub max_reach: f64,
    /// A note's rolling speed.
    pub roll: f64,
    /// Beats a note rolls (`t1 = t_land + hold * beat`).
    pub hold: f64,
    /// How much of the enemy's motion over the note's travel the aim leads by (`w.get('lead', 0.8)`).
    pub lead: f64,
    /// The aim's seeded wobble in degrees (`w.get('jitter', 0.0)`).
    pub jitter: f64,
    /// The first note's damage (`note_dmg` starts at `float(w['dmg'])`).
    pub dmg: f64,
    /// The damage a landed note adds to the next.
    pub dmg_grow: f64,
    /// The note damage never grows past this.
    pub dmg_max: f64,
    /// Knockback of a crush.
    pub knock: f64,
}

/// The keys a `rests` entry may hold: `kind` and [`RestsCfg`]'s fields.
const RESTS_KEYS: [&str; 16] = [
    "kind",
    "style",
    "name",
    "every",
    "guard",
    "base",
    "grow",
    "max_reach",
    "roll",
    "hold",
    "lead",
    "jitter",
    "dmg",
    "dmg_grow",
    "dmg_max",
    "knock",
];

/// Reads a `rests` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<RestsCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a rests weapon", &RESTS_KEYS)?;
    let cfg = RestsCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "rests".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "RESTS".to_string(),
        },
        every: e.f64(w, p, "every")?,
        guard: e.f64_or(w, p, "guard", 1.0)?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        roll: e.f64(w, p, "roll")?,
        hold: e.f64(w, p, "hold")?,
        lead: e.f64_or(w, p, "lead", 0.8)?,
        jitter: e.f64_or(w, p, "jitter", 0.0)?,
        dmg: e.f64(w, p, "dmg")?,
        dmg_grow: e.f64(w, p, "dmg_grow")?,
        dmg_max: e.f64(w, p, "dmg_max")?,
        knock: e.f64(w, p, "knock")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): what the Python fight
/// cannot run. `int(every)` must be a whole cycle of at least one beat (`kb % cyc` raises for 0, `int()` for a NaN),
/// and a note must roll (`tl = ... / w['roll']` divides by it).
pub(crate) fn check(e: &Entry, w: &RestsCfg) -> Result<(), RosterError> {
    let p = "weapon";
    if !w.every.is_finite() || w.every.trunc() < 1.0 {
        let why = "must be a finite number of at least 1 (a cycle of int(every) beats)";
        return Err(e.err(&format!("{p}.every"), why));
    }
    if w.roll == 0.0 || !w.roll.is_finite() {
        return Err(e.err(&format!("{p}.roll"), "must be a finite number other than 0 (a note's rolling speed)"));
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`): its `base`,
/// the rest at Python's defaults.
pub fn ball_start(w: &RestsCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste: none (the rests count on the beat grid, `rests_tick`'s own counter,
/// which no haste divides).
pub(crate) fn main16(_w: &RestsCfg) -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use crate::roster::{AbilityKind, FighterCfg, WeaponCfg, parse_roster};

    fn viola() -> FighterCfg {
        let older = parse_roster(include_str!("../../../../bf-sim/tests/older_roster.json")).unwrap();
        older.into_iter().find(|f| f.name == "VIOLA").unwrap()
    }

    #[test]
    fn the_violas_rests_are_typed() {
        let cfg = viola();
        let WeaponCfg::Rests(w) = &cfg.weapon else {
            panic!("VIOLA's weapon is {:?}", cfg.weapon)
        };
        assert_eq!(
            (w.every, w.guard, w.base, w.grow, w.max_reach, w.roll, w.hold, w.lead, w.jitter),
            (4.0, 0.5, 0.45, 0.04, 0.8, 2.0, 4.0, 0.5, 12.0)
        );
        assert_eq!((w.dmg, w.dmg_grow, w.dmg_max, w.knock), (3.0, 0.04, 3.4, 1.4));
        assert_eq!((w.style.as_str(), w.name.as_str()), ("wholenote", "COUNTING RESTS"));
        assert_eq!(cfg.unlock1.kind, AbilityKind::DoubleStop);
        assert_eq!(cfg.unlock2.kind, AbilityKind::Melody);
        assert_eq!(cfg.low.kind, AbilityKind::Joke);
        assert_eq!(cfg.low.get("tele"), Some(1.0));
        assert_eq!(cfg.unlock2.get("rest"), Some(1.0));
    }
}
