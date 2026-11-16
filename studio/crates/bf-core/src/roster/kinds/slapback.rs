//! Weapon kind `slapback`, MELLOPHONE's SLAPBACK: its roster settings. A Studio kind (the Python kit never had it), so
//! the engine's `bf_sim::kinds::slapback` is its source of truth (spec
//! `docs/superpowers/specs/2026-11-16-mellophone-fighter-design.md`).
//!
//! On its grid (`every` beats; haste divides it) the mellophone blows a wave at the enemy: a wide arc (half-width
//! `half_w`) flying at `speed`. It hits the enemy it touches for `dmg` + `dmg_step` x its echo generation (knock
//! `knock`) and is gone. At a rope it stops; while it has echoes left, `delay` beats later a new wave leaves that rope
//! point, re-aimed at the enemy, one generation up and one echo fewer. A first wave carries `int(n_echo)` echoes (`n0`
//! at the start, `grow` more a landed hit, up to `n_max`).

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `slapback`.
pub const PORTED: bool = true;

/// Weapon kind `slapback`'s typed settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlapbackCfg {
    /// The picture's style (default `slapback`).
    pub style: String,
    /// The weapon's name (default `SLAPBACK`).
    pub name: String,
    /// Beats between two waves on the 16th grid.
    pub every: f64,
    /// Units a second a wave flies.
    pub speed: f64,
    /// Half the wave's width (it hits within this plus a ball radius).
    pub half_w: f64,
    /// Beats from a wave reaching a rope to its echo leaving it.
    pub delay: f64,
    /// Echoes a first wave carries at the start (`n_vol`).
    pub n0: f64,
    /// Added to `n_vol` per landed hit.
    pub grow: f64,
    /// `n_vol` never grows past this.
    pub n_max: f64,
    /// Damage of a first wave.
    pub dmg: f64,
    /// Damage added per echo generation.
    pub dmg_step: f64,
    /// Knockback of a hit.
    pub knock: f64,
}

const KEYS: [&str; 13] = [
    "kind", "style", "name", "every", "speed", "half_w", "delay", "n0", "grow", "n_max", "dmg", "dmg_step", "knock",
];

/// Reads a `slapback` weapon entry.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<SlapbackCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a slapback weapon", &KEYS)?;
    let cfg = SlapbackCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "slapback".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "SLAPBACK".to_string(),
        },
        every: e.grid(w, p, "every")?,
        speed: e.f64(w, p, "speed")?,
        half_w: e.f64(w, p, "half_w")?,
        delay: e.f64(w, p, "delay")?,
        n0: e.f64(w, p, "n0")?,
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        dmg: e.f64(w, p, "dmg")?,
        dmg_step: e.f64(w, p, "dmg_step")?,
        knock: e.f64(w, p, "knock")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// What the engine cannot run: a grid that is not finite, a wave that never moves, counts that are not finite.
pub(crate) fn check(e: &Entry, w: &SlapbackCfg) -> Result<(), RosterError> {
    let p = "weapon";
    e.grid_value(p, "every", w.every)?;
    if crate::py::round_half_even(4.0 * w.every) < 1.0 {
        return Err(e.err("weapon.every", format!("{} beats is less than a 16th note", w.every)));
    }
    for (key, x) in [
        ("n0", w.n0),
        ("grow", w.grow),
        ("n_max", w.n_max),
        ("half_w", w.half_w),
        ("delay", w.delay),
        ("dmg", w.dmg),
        ("dmg_step", w.dmg_step),
        ("knock", w.knock),
    ] {
        if !x.is_finite() || x < 0.0 {
            return Err(e.err(&format!("{p}.{key}"), "must be a finite number, not negative"));
        }
    }
    if w.n_max > 16.0 {
        return Err(e.err(&format!("{p}.n_max"), "more than 16 echoes would never end"));
    }
    if !(w.speed.is_finite() && w.speed > 0.0 && w.speed < 1000.0) {
        return Err(e.err(&format!("{p}.speed"), "must be a number above 0 and below 1000"));
    }
    Ok(())
}

/// What `Ball::new` reads from this weapon: its `n0`.
pub fn ball_start(w: &SlapbackCfg) -> BallStart {
    BallStart {
        n0: w.n0,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste.
pub(crate) fn main16(w: &SlapbackCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}
