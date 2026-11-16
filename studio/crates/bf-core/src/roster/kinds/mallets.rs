//! Weapon kind `mallets`, MARIMBA's MALLET RUN: its roster settings. A Studio kind (the Python kit never had it), so
//! the engine's `bf_sim::kinds::mallets` is its source of truth.
//!
//! On its grid (`every` beats; haste divides it) the marimba throws a mallet at the enemy while fewer than
//! `int(n_vol)` of its mallets are in the air (`n0` at the start, `grow` more per landed hit, up to `n_max`). A mallet
//! flies at `speed`, bounces off the ropes up to `bounces` times (each bounce adds `dmg_step`), and hits the enemy it
//! touches (within `hit_r` + a ball radius) for `dmg` + `dmg_step` x its bounces and a knock of `knock`.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `mallets`.
pub const PORTED: bool = true;

/// Weapon kind `mallets`'s typed settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MalletsCfg {
    /// The picture's style (default `mallets`).
    pub style: String,
    /// The weapon's name (default `MALLETS`).
    pub name: String,
    /// Beats between two throws on the 16th grid.
    pub every: f64,
    /// Units a second a mallet flies.
    pub speed: f64,
    /// Wall bounces before a mallet fizzles.
    pub bounces: f64,
    /// Mallets in the air at the start (`n_vol`).
    pub n0: f64,
    /// Added to `n_vol` per landed hit.
    pub grow: f64,
    /// `n_vol` never grows past this.
    pub n_max: f64,
    /// Damage of a hit before any bounce.
    pub dmg: f64,
    /// Damage added per wall bounce.
    pub dmg_step: f64,
    /// Knockback of a hit.
    pub knock: f64,
    /// A mallet head's radius (it hits within this plus a ball radius).
    pub hit_r: f64,
}

const KEYS: [&str; 13] = [
    "kind", "style", "name", "every", "speed", "bounces", "n0", "grow", "n_max", "dmg", "dmg_step", "knock", "hit_r",
];

/// Reads a `mallets` weapon entry.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<MalletsCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a mallets weapon", &KEYS)?;
    let cfg = MalletsCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "mallets".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "MALLETS".to_string(),
        },
        every: e.grid(w, p, "every")?,
        speed: e.f64(w, p, "speed")?,
        bounces: e.f64(w, p, "bounces")?,
        n0: e.f64(w, p, "n0")?,
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        dmg: e.f64(w, p, "dmg")?,
        dmg_step: e.f64(w, p, "dmg_step")?,
        knock: e.f64(w, p, "knock")?,
        hit_r: e.f64_or(w, p, "hit_r", 0.12)?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// What the engine cannot run: a grid that is not finite, a mallet that never moves or never ends, counts that are
/// not finite.
pub(crate) fn check(e: &Entry, w: &MalletsCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    for (key, x) in [
        ("n0", w.n0),
        ("grow", w.grow),
        ("n_max", w.n_max),
        ("bounces", w.bounces),
    ] {
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
pub fn ball_start(w: &MalletsCfg) -> BallStart {
    BallStart {
        n0: w.n0,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste.
pub(crate) fn main16(w: &MalletsCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}
