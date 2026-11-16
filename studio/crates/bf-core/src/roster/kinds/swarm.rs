//! Weapon kind `swarm`, SOPRANO SAX's HORNET SWARM (a from-scratch fighter, 2026-11-16; no Python kit reference): its
//! roster settings. The engine side is `bf-sim/src/kinds/swarm.rs`.
//!
//! Hornets orbit the fighter (`start` of them, growing to `max`). Every `every` beats (from 16th `off16`), when it is
//! not stunned, hornets peel off and chase the enemy (one, plus one for every two the swarm has grown, never more than
//! are orbiting) at `speed`, steering toward it at `steer` a second, for up to `life` beats. A chaser that touches the
//! enemy (within `bee_r` of its edge) stings for `dmg` with a knock of `knock`, flies home and adds a hornet.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `swarm`.
pub const PORTED: bool = true;

/// Weapon kind `swarm`'s typed settings.
#[derive(Clone, Debug, PartialEq)]
pub struct SwarmCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// The weapon's name on the card.
    pub name: Option<String>,
    /// Beats between two launches on the 16th grid.
    pub every: f64,
    /// The 16th of the grid the first launch falls on (default 0).
    pub off16: f64,
    /// Hornets at the start.
    pub start: f64,
    /// The swarm never grows past this many.
    pub max: f64,
    /// A chaser's speed, ring units a second.
    pub speed: f64,
    /// How fast a chaser turns toward the enemy (a blend rate a second).
    pub steer: f64,
    /// Beats a chaser hunts before it gives up.
    pub life: f64,
    /// A sting's damage.
    pub dmg: f64,
    /// A sting's knock.
    pub knock: f64,
    /// A hornet's radius, for the contact test.
    pub bee_r: f64,
}

/// The keys a swarm entry may hold.
const SWARM_KEYS: [&str; 13] = [
    "kind", "style", "name", "every", "off16", "start", "max", "speed", "steer", "life", "dmg", "knock", "bee_r",
];

/// Reads a `swarm` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<SwarmCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a swarm weapon", &SWARM_KEYS)?;
    let cfg = SwarmCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "swarm".to_string(),
        },
        name: match w.get("name") {
            Some(_) => Some(e.string(w, p, "name")?),
            None => None,
        },
        every: e.grid(w, p, "every")?,
        off16: e.f64_or(w, p, "off16", 0.0)?,
        start: e.f64(w, p, "start")?,
        max: e.f64(w, p, "max")?,
        speed: e.f64(w, p, "speed")?,
        steer: e.f64(w, p, "steer")?,
        life: e.f64(w, p, "life")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
        bee_r: e.f64(w, p, "bee_r")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code: the grid, a swarm that exists, hornets that move.
pub(crate) fn check(e: &Entry, w: &SwarmCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    if crate::py::round_half_even(4.0 * w.every) < 1.0 {
        return Err(e.err("weapon.every", format!("{} beats is less than a 16th note", w.every)));
    }
    if !(w.off16.is_finite() && w.off16 >= 0.0 && w.off16 < 1e6) {
        return Err(e.err("weapon.off16", format!("{} is not a 16th it can count", w.off16)));
    }
    if !(w.start >= 1.0 && w.max >= w.start && w.max <= 16.0 && w.start.fract() == 0.0 && w.max.fract() == 0.0) {
        return Err(e.err(
            "weapon.max",
            "the swarm must start with 1 to 16 whole hornets and grow to at least start",
        ));
    }
    if !(w.speed.is_finite() && w.speed > 0.0 && w.life > 0.0) {
        return Err(e.err("weapon.speed", "a hornet must fly and hunt for some time"));
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon: nothing of its own.
pub fn ball_start(_w: &SwarmCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste.
pub(crate) fn main16(w: &SwarmCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}
