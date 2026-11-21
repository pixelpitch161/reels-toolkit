//! Weapon kind `pick`, BASS GUITAR's PICK TOSS: its roster settings. A Studio kind (the Python kit never had it), so
//! the engine's `bf_sim::kinds::pick` is its source of truth.
//!
//! On its grid (`every` beats; haste divides it) the bass throws `int(n_vol)` big picks (`n0` at the start, `grow`
//! more per landed pick, up to `n_max`). Each flies a curved loop over `flight_beats` beats: out to the enemy's spot
//! (`lead` x its velocity ahead), bending `bend` of the distance to one side, and back to the ball; extra picks fan
//! out `fan` radians apart, bending to alternate sides. A pick hits once (within `hit_r` ball radii + the enemy's
//! radius) for `dmg` and a knock of `knock` along its flight.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `pick`.
pub const PORTED: bool = true;

/// Weapon kind `pick`'s typed settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PickCfg {
    /// The picture's style (default `pick`).
    pub style: String,
    /// The weapon's name (default `PICK`).
    pub name: String,
    /// Beats between two throws.
    pub every: f64,
    /// Beats a pick's loop takes, out and back.
    pub flight_beats: f64,
    /// How far ahead of the enemy it aims, in seconds of the enemy's velocity per beat of flight.
    pub lead: f64,
    /// The loop's sideways bend, as a share of the distance.
    pub bend: f64,
    /// Radians between two picks of a fan.
    pub fan: f64,
    /// Picks a throw at the start.
    pub n0: f64,
    /// Added per landed pick.
    pub grow: f64,
    /// The most picks a throw.
    pub n_max: f64,
    /// Damage of a pick.
    pub dmg: f64,
    /// Knockback of a pick.
    pub knock: f64,
    /// A pick's radius, in ball radii.
    pub hit_r: f64,
}

const KEYS: [&str; 14] = [
    "kind",
    "style",
    "name",
    "every",
    "flight_beats",
    "lead",
    "bend",
    "fan",
    "n0",
    "grow",
    "n_max",
    "dmg",
    "knock",
    "hit_r",
];

/// Reads a `pick` weapon entry.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<PickCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a pick weapon", &KEYS)?;
    let cfg = PickCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "pick".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "PICK".to_string(),
        },
        every: e.grid(w, p, "every")?,
        flight_beats: e.f64(w, p, "flight_beats")?,
        lead: e.f64_or(w, p, "lead", 0.0)?,
        bend: e.f64(w, p, "bend")?,
        fan: e.f64(w, p, "fan")?,
        n0: e.f64(w, p, "n0")?,
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
        hit_r: e.f64_or(w, p, "hit_r", 0.2)?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// What the engine cannot run: a grid it cannot place, a loop that takes no time, at least one pick a throw, numbers
/// that are not finite or are negative.
pub(crate) fn check(e: &Entry, w: &PickCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    if !(w.flight_beats.is_finite() && w.flight_beats > 0.0 && w.flight_beats <= 16.0) {
        return Err(e.err(&format!("{p}.flight_beats"), "must be a number above 0, up to 16"));
    }
    for (key, x) in [
        ("lead", w.lead),
        ("bend", w.bend),
        ("fan", w.fan),
        ("grow", w.grow),
        ("dmg", w.dmg),
        ("knock", w.knock),
        ("hit_r", w.hit_r),
    ] {
        if !x.is_finite() || x < 0.0 {
            return Err(e.err(&format!("{p}.{key}"), "must be a finite number, not negative"));
        }
    }
    if !(w.n0.is_finite() && w.n0 >= 1.0 && w.n_max.is_finite() && w.n_max >= w.n0 && w.n_max <= 8.0) {
        return Err(e.err(&format!("{p}.n_max"), "needs 1 <= n0 <= n_max <= 8"));
    }
    Ok(())
}

/// What `Ball::new` reads from this weapon: its first pick count.
pub fn ball_start(w: &PickCfg) -> BallStart {
    BallStart {
        n0: w.n0,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste.
pub(crate) fn main16(w: &PickCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}
