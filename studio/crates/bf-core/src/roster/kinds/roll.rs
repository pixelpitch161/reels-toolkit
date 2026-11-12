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
