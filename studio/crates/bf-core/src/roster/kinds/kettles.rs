//! Weapon kind `kettles`, TIMPANI's TWO KETTLES: its roster settings. A Studio kind (the Python kit never had it), so
//! the engine's `bf_sim::kinds::kettles` is its source of truth (spec
//! `docs/superpowers/specs/2026-11-16-timpani-fighter-design.md`).
//!
//! The timpani sets a low kettle (`r_low`) and a high kettle (`r_high`) on the floor and hops between them every
//! `every` beats (`air16` 16ths in the air; a haste divides both). Each landing booms that kettle: a ring leaves its
//! centre at `speed_low` / `speed_high` out to `reach_low` / `reach_high` and hits the enemy once for `dmg_low` /
//! `dmg_high` (knock `knock_low` / `knock_high`). Hit by both kettles within `pair_t` seconds: `dmg_pair` more.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `kettles`.
pub const PORTED: bool = true;

/// Weapon kind `kettles`'s typed settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KettlesCfg {
    /// The picture's style (default `kettles`).
    pub style: String,
    /// The weapon's name (default `TWO KETTLES`).
    pub name: String,
    /// Beats between two hops on the 16th grid.
    pub every: f64,
    /// 16ths a hop is in the air.
    pub air16: f64,
    /// Where the kettles stand, as a share of the ring's half-size along the diagonal.
    pub spot: f64,
    /// The low kettle's radius.
    pub r_low: f64,
    /// The high kettle's radius.
    pub r_high: f64,
    /// FULL SET's extra kettles' radius.
    pub r_mid: f64,
    /// Units a second the low kettle's ring grows.
    pub speed_low: f64,
    /// How far the low kettle's ring reaches from the kettle's centre.
    pub reach_low: f64,
    /// Damage of the low kettle's ring.
    pub dmg_low: f64,
    /// Knockback of the low kettle's ring.
    pub knock_low: f64,
    /// Units a second the high kettle's ring grows.
    pub speed_high: f64,
    /// How far the high kettle's ring reaches.
    pub reach_high: f64,
    /// Damage of the high kettle's ring.
    pub dmg_high: f64,
    /// Knockback of the high kettle's ring.
    pub knock_high: f64,
    /// Seconds within which a hit by both kettles is the interval.
    pub pair_t: f64,
    /// Damage of the interval hit.
    pub dmg_pair: f64,
}

const KEYS: [&str; 19] = [
    "kind",
    "style",
    "name",
    "every",
    "air16",
    "spot",
    "r_low",
    "r_high",
    "r_mid",
    "speed_low",
    "reach_low",
    "dmg_low",
    "knock_low",
    "speed_high",
    "reach_high",
    "dmg_high",
    "knock_high",
    "pair_t",
    "dmg_pair",
];

/// Reads a `kettles` weapon entry.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<KettlesCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a kettles weapon", &KEYS)?;
    let cfg = KettlesCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "kettles".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "TWO KETTLES".to_string(),
        },
        every: e.grid(w, p, "every")?,
        air16: e.f64_or(w, p, "air16", 2.0)?,
        spot: e.f64_or(w, p, "spot", 0.5)?,
        r_low: e.f64(w, p, "r_low")?,
        r_high: e.f64(w, p, "r_high")?,
        r_mid: e.f64_or(w, p, "r_mid", 0.5)?,
        speed_low: e.f64(w, p, "speed_low")?,
        reach_low: e.f64(w, p, "reach_low")?,
        dmg_low: e.f64(w, p, "dmg_low")?,
        knock_low: e.f64(w, p, "knock_low")?,
        speed_high: e.f64(w, p, "speed_high")?,
        reach_high: e.f64(w, p, "reach_high")?,
        dmg_high: e.f64(w, p, "dmg_high")?,
        knock_high: e.f64(w, p, "knock_high")?,
        pair_t: e.f64_or(w, p, "pair_t", 0.6)?,
        dmg_pair: e.f64_or(w, p, "dmg_pair", 0.0)?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// What the engine cannot run: a grid that is not finite, a ring that never grows, sizes that are not finite.
pub(crate) fn check(e: &Entry, w: &KettlesCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    for (key, x) in [
        ("air16", w.air16),
        ("r_low", w.r_low),
        ("r_high", w.r_high),
        ("r_mid", w.r_mid),
        ("reach_low", w.reach_low),
        ("reach_high", w.reach_high),
        ("dmg_low", w.dmg_low),
        ("dmg_high", w.dmg_high),
        ("knock_low", w.knock_low),
        ("knock_high", w.knock_high),
        ("pair_t", w.pair_t),
        ("dmg_pair", w.dmg_pair),
    ] {
        if !x.is_finite() || x < 0.0 {
            return Err(e.err(&format!("{p}.{key}"), "must be a finite number, not negative"));
        }
    }
    if !(w.spot.is_finite() && w.spot > 0.0 && w.spot < 1.0) {
        return Err(e.err(&format!("{p}.spot"), "must be above 0 and below 1"));
    }
    for (key, x) in [("speed_low", w.speed_low), ("speed_high", w.speed_high)] {
        if !(x.is_finite() && x > 0.0 && x < 1000.0) {
            return Err(e.err(&format!("{p}.{key}"), "must be a number above 0 and below 1000"));
        }
    }
    Ok(())
}

/// What `Ball::new` reads from this weapon: nothing.
pub fn ball_start(_w: &KettlesCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste.
pub(crate) fn main16(w: &KettlesCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}
