//! Weapon kind `plunger`, CORNET's PLUNGER MUTE (a from-scratch fighter, 2026-11-16; no Python kit reference): its
//! roster settings. The engine side is `bf-sim/src/kinds/plunger.rs`.
//!
//! Every `every` beats (from 16th `off16`), when no cup is out and the fighter is not stunned, a plunger cup on a cord
//! flies at the enemy (aimed `lead` seconds ahead of its motion) at `speed`. Reaching the enemy within the cord's
//! reach (`blade_len`, from `base`) it sticks: `dmg_stick`, then on each beat while stuck a "wah" of `dmg` and a pull of
//! `pull` toward the fighter. It pops off after `hold` beats (grown by `hold_grow` a stick, up to `hold_max`) or when
//! the cord is stretched past `snap` times the reach, then reels back at `speed`. A stick grows the reach by `grow` up
//! to `max_reach`. A shield blocks the stick.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `plunger`.
pub const PORTED: bool = true;

/// Weapon kind `plunger`'s typed settings.
#[derive(Clone, Debug, PartialEq)]
pub struct PlungerCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// The weapon's name on the card.
    pub name: Option<String>,
    /// Beats between two throws on the 16th grid.
    pub every: f64,
    /// The 16th of the grid the first throw falls on (default 0).
    pub off16: f64,
    /// The cup's speed out and back, ring units a second.
    pub speed: f64,
    /// Seconds of the enemy's motion the aim leads by (default 0.3).
    pub lead: f64,
    /// The cord's reach at the start.
    pub base: f64,
    /// Reach added per stick.
    pub grow: f64,
    /// The reach never grows past this.
    pub max_reach: f64,
    /// Beats a stick holds at the start.
    pub hold: f64,
    /// Beats added to the hold per stick.
    pub hold_grow: f64,
    /// The hold never grows past this many beats.
    pub hold_max: f64,
    /// The cup pops when the cord is stretched past this many times the reach (default 1.4).
    pub snap: f64,
    /// Damage when the cup sticks.
    pub dmg_stick: f64,
    /// Damage of each wah.
    pub dmg: f64,
    /// The pull of each wah toward the fighter (a knock's speed boost).
    pub pull: f64,
    /// The cup's head radius, for the contact test.
    pub cup_r: f64,
}

/// The keys a plunger entry may hold.
const PLUNGER_KEYS: [&str; 18] = [
    "kind",
    "style",
    "name",
    "every",
    "off16",
    "speed",
    "lead",
    "base",
    "grow",
    "max_reach",
    "hold",
    "hold_grow",
    "hold_max",
    "snap",
    "dmg_stick",
    "dmg",
    "pull",
    "cup_r",
];

/// Reads a `plunger` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<PlungerCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a plunger weapon", &PLUNGER_KEYS)?;
    let cfg = PlungerCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "plunger".to_string(),
        },
        name: match w.get("name") {
            Some(_) => Some(e.string(w, p, "name")?),
            None => None,
        },
        every: e.grid(w, p, "every")?,
        off16: e.f64_or(w, p, "off16", 0.0)?,
        speed: e.f64(w, p, "speed")?,
        lead: e.f64_or(w, p, "lead", 0.3)?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        hold: e.f64(w, p, "hold")?,
        hold_grow: e.f64(w, p, "hold_grow")?,
        hold_max: e.f64(w, p, "hold_max")?,
        snap: e.f64_or(w, p, "snap", 1.4)?,
        dmg_stick: e.f64(w, p, "dmg_stick")?,
        dmg: e.f64(w, p, "dmg")?,
        pull: e.f64(w, p, "pull")?,
        cup_r: e.f64(w, p, "cup_r")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code: the grid, a cup that moves, a reach and a hold.
pub(crate) fn check(e: &Entry, w: &PlungerCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    if crate::py::round_half_even(4.0 * w.every) < 1.0 {
        return Err(e.err("weapon.every", format!("{} beats is less than a 16th note", w.every)));
    }
    if !(w.off16.is_finite() && w.off16 >= 0.0 && w.off16 < 1e6) {
        return Err(e.err("weapon.off16", format!("{} is not a 16th it can count", w.off16)));
    }
    if !(w.speed.is_finite() && w.speed > 0.0) {
        return Err(e.err("weapon.speed", format!("{} would never reach anything", w.speed)));
    }
    if !(w.base > 0.0 && w.max_reach >= w.base) {
        return Err(e.err(
            "weapon.max_reach",
            "the reach must start above 0 and grow to at least base",
        ));
    }
    if !(w.hold > 0.0 && w.hold_max >= w.hold) {
        return Err(e.err(
            "weapon.hold_max",
            "the hold must start above 0 and grow to at least hold",
        ));
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon: `base`, the cord's reach at the start.
pub fn ball_start(w: &PlungerCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste.
pub(crate) fn main16(w: &PlungerCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}
