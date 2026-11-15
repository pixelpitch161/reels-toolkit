//! Weapon kind `bite`, ACCORDION's BELLOWS BITE: its roster settings. `fight.py` 1868 to 1904 (`bite_fire`,
//! `bite_snap`) and its branch of `grid16_fire` (1487 to 1507).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `bite`.
pub const PORTED: bool = true;

/// Weapon kind `bite`'s typed settings. Fields without a default are read as `w[name]`; the others are
/// `w.get(name, default)` with `fight.py`'s default.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BiteCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// Beats between two main bites on the 16th grid (a haste divides it).
    pub every: f64,
    /// The jaws' reach beyond the ball a fight starts with (`blade_len`, x R).
    pub base: f64,
    /// Reach added per landed main bite.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// The jaws' half-angle in degrees.
    pub half: f64,
    /// 16ths the jaws stay open before they snap (`.get('open16', 2)`).
    pub open16: f64,
    /// How far ahead along the enemy's heading the bite aims (`.get('lead', 0.6)`).
    pub lead: f64,
    /// A main bite's damage.
    pub dmg: f64,
    /// An off-grid bite's damage factor (`.get('sub_dmg', 0.5)`).
    pub sub_dmg: f64,
    /// Knockback of a main bite (an off-grid one: 0.6 of it).
    pub knock: f64,
}

/// The keys a bite entry may hold: `kind`, `style` and [`BiteCfg`]'s fields.
const BITE_KEYS: [&str; 12] = [
    "kind",
    "style",
    "every",
    "base",
    "grow",
    "max_reach",
    "half",
    "open16",
    "lead",
    "dmg",
    "sub_dmg",
    "knock",
];

/// Reads a `bite` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<BiteCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a bite weapon", &BITE_KEYS)?;
    Ok(BiteCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "bite".to_string(),
        },
        every: e.grid(w, p, "every")?,
        base: e.f64_or(w, p, "base", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        half: e.f64(w, p, "half")?,
        open16: e.f64_or(w, p, "open16", 2.0)?,
        lead: e.f64_or(w, p, "lead", 0.6)?,
        dmg: e.f64(w, p, "dmg")?,
        sub_dmg: e.f64_or(w, p, "sub_dmg", 0.5)?,
        knock: e.f64(w, p, "knock")?,
    })
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &BiteCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): `base`, the jaws' starting reach.
pub fn ball_start(w: &BiteCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste: `int(round(4 * every))`, as a float.
pub(crate) fn main16(w: &BiteCfg) -> Option<f64> {
    Some(4.0 * w.every)
}
