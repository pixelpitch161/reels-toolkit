//! Weapon kind `striker`, FLUTE's cobras: its roster settings (`fight.py` `striker_setup`, `striker_beat`,
//! `striker_fire`, `striker_step`, `swat_cobras`, and the start of the fight loop).

use serde_json::Value as Json;

use crate::roster::{Entry, RosterError};

/// Weapon kind `striker` (`fight.py` `striker_setup`, `striker_beat`, `striker_fire`, `striker_step`,
/// `swat_cobras`, and the start of the fight loop). Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct StrikerCfg {
    /// The picture's style (default: the kind name, as `kit/preview.py` reads it).
    pub style: String,
    /// Beats between strikes on the 16th grid (`int(round(4 * every))` 16ths; haste divides it). In a roster entry
    /// 4 x `every` is a finite number.
    pub every: f64,
    /// A basket drops every `int(drop_every)` beats; a roster entry must give at least 1.
    pub drop_every: f64,
    /// At most `int(max_baskets)` baskets, at least 1 in a roster entry; the oldest is taken back for a new one.
    pub max_baskets: f64,
    /// Beats a cobra takes to rise out of its basket.
    pub rise: f64,
    /// The starting reach (`blade_len`; `Ball` reads `.get('base', 0.0)`).
    pub base: f64,
    /// Reach added per landed bite.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// A cobra lunges when the enemy is within reach plus this (`.get('slack', 0.25)`).
    pub slack: f64,
    /// Seconds a lunge takes at the start.
    pub t_strike: f64,
    /// A lunge never gets quicker than this.
    pub t_min: f64,
    /// Seconds taken off a lunge per landed bite (`.get('speed_grow', 0.0)`).
    pub speed_grow: f64,
    /// The fang line's radius.
    pub head_r: f64,
    /// Damage per bite.
    pub dmg: f64,
    /// Knockback per bite.
    pub knock: f64,
    /// Units a second the cobras slither (x 1.6 on the way home).
    pub slither: f64,
}

/// The keys a striker entry may hold: `kind`, `style` and [`StrikerCfg`]'s fields.
const STRIKER_KEYS: [&str; 17] = [
    "kind",
    "style",
    "every",
    "drop_every",
    "max_baskets",
    "rise",
    "base",
    "grow",
    "max_reach",
    "slack",
    "t_strike",
    "t_min",
    "speed_grow",
    "head_r",
    "dmg",
    "knock",
    "slither",
];

/// Reads a striker weapon entry.
pub(crate) fn parse(e: &Entry, w: &Json, style: String) -> Result<StrikerCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a striker weapon", &STRIKER_KEYS)?;
    Ok(StrikerCfg {
        style,
        every: e.grid(w, p, "every")?,
        drop_every: e.count(w, p, "drop_every", "a basket drops every that many beats")?,
        max_baskets: e.count(w, p, "max_baskets", "there must be room for a basket")?,
        rise: e.f64(w, p, "rise")?,
        base: e.f64_or(w, p, "base", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        slack: e.f64_or(w, p, "slack", 0.25)?,
        t_strike: e.f64(w, p, "t_strike")?,
        t_min: e.f64(w, p, "t_min")?,
        speed_grow: e.f64_or(w, p, "speed_grow", 0.0)?,
        head_r: e.f64(w, p, "head_r")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
        slither: e.f64(w, p, "slither")?,
    })
}

/// [`parse`]'s checks made again on typed settings (`FighterCfg::check`).
pub(crate) fn check(e: &Entry, w: &StrikerCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    e.count_value(p, "drop_every", w.drop_every, "a basket drops every that many beats")?;
    e.count_value(p, "max_baskets", w.max_baskets, "there must be room for a basket")?;
    Ok(())
}
