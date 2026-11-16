//! Weapon kind `needle`, PICCOLO's PIERCING NOTE: its roster settings. A Studio kind (the Python kit never had it), so
//! the engine's `bf_sim::kinds::needle` is its source of truth (spec
//! `docs/superpowers/specs/2026-11-16-piccolo-fighter-design.md`).
//!
//! On its grid (`every` beats; haste divides it) the piccolo fires a needle at the enemy as a bank shot. A needle flies
//! at `speed`, bounces off the ropes `int(n_vol)` times (`n0` at the start, `grow` more per landed hit, up to `n_max`)
//! and fizzles at the next rope; it hits the enemy it passes through (within `hit_r` + a ball radius) once, for `dmg`
//! and a knock of `knock`, and keeps flying.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `needle`.
pub const PORTED: bool = true;

/// Weapon kind `needle`'s typed settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NeedleCfg {
    /// The picture's style (default `needle`).
    pub style: String,
    /// The weapon's name (default `PIERCING NOTE`).
    pub name: String,
    /// Beats between two throws on the 16th grid.
    pub every: f64,
    /// Units a second a needle flies.
    pub speed: f64,
    /// Bounces a needle makes at the start (`n_vol`).
    pub n0: f64,
    /// Added to `n_vol` per landed hit.
    pub grow: f64,
    /// `n_vol` never grows past this.
    pub n_max: f64,
    /// Damage of a hit.
    pub dmg: f64,
    /// Knockback of a hit.
    pub knock: f64,
    /// A needle head's radius (it hits within this plus a ball radius).
    pub hit_r: f64,
}

const KEYS: [&str; 11] = [
    "kind", "style", "name", "every", "speed", "n0", "grow", "n_max", "dmg", "knock", "hit_r",
];

/// Reads a `needle` weapon entry.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<NeedleCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a needle weapon", &KEYS)?;
    let cfg = NeedleCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "needle".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "PIERCING NOTE".to_string(),
        },
        every: e.grid(w, p, "every")?,
        speed: e.f64(w, p, "speed")?,
        n0: e.f64(w, p, "n0")?,
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
        hit_r: e.f64_or(w, p, "hit_r", 0.06)?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// What the engine cannot run: a grid that is not finite, a needle that never moves or never ends, counts that are
/// not finite.
pub(crate) fn check(e: &Entry, w: &NeedleCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    for (key, x) in [("n0", w.n0), ("grow", w.grow), ("n_max", w.n_max)] {
        if !x.is_finite() || x < 0.0 {
            return Err(e.err(&format!("{p}.{key}"), "must be a finite number, not negative"));
        }
    }
    if !(w.speed.is_finite() && w.speed > 0.0 && w.speed < 1000.0) {
        return Err(e.err(&format!("{p}.speed"), "must be a number above 0 and below 1000"));
    }
    if !(w.hit_r.is_finite() && w.hit_r >= 0.0) {
        return Err(e.err(&format!("{p}.hit_r"), "must be a finite number, not negative"));
    }
    Ok(())
}

/// What `Ball::new` reads from this weapon: its `n0`.
pub fn ball_start(w: &NeedleCfg) -> BallStart {
    BallStart {
        n0: w.n0,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste.
pub(crate) fn main16(w: &NeedleCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}
