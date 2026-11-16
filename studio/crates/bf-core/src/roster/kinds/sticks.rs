//! Weapon kind `sticks`, SNARE DRUM's PARADIDDLE: its roster settings. A Studio kind (the Python kit never had it),
//! so the engine's `bf_sim::kinds::sticks` is its source of truth.
//!
//! Two sticks ride the ball, one each side of the line to the enemy (`spread` degrees off it). Every `step16`
//! sixteenths (haste shortens it) one stroke is played on the paradiddle sticking R L R R, L R L L: that side's
//! stick jabs out over `out16` sixteenths to `base` ball radii beyond the ball's edge (`grow` more per landed hit,
//! up to `max_reach`). A jab touching the enemy hits once for `dmg` (x `accent` on the first stroke of each group of
//! four) and a knock of `knock`.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `sticks`.
pub const PORTED: bool = true;

/// Weapon kind `sticks`'s typed settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SticksCfg {
    /// The picture's style (default `sticks`).
    pub style: String,
    /// The weapon's name (default `STICKS`).
    pub name: String,
    /// Sixteenths between two strokes.
    pub step16: f64,
    /// Sixteenths a jab takes to reach out.
    pub out16: f64,
    /// Degrees each stick sits off the line to the enemy.
    pub spread: f64,
    /// A jab's reach beyond the ball's edge at the start, in ball radii.
    pub base: f64,
    /// Added to the reach per landed hit, in ball radii.
    pub grow: f64,
    /// The reach never grows past this, in ball radii.
    pub max_reach: f64,
    /// Damage of a stroke.
    pub dmg: f64,
    /// An accent's damage factor (the first stroke of each group of four).
    pub accent: f64,
    /// Knockback of a stroke.
    pub knock: f64,
}

const KEYS: [&str; 12] = [
    "kind",
    "style",
    "name",
    "step16",
    "out16",
    "spread",
    "base",
    "grow",
    "max_reach",
    "dmg",
    "accent",
    "knock",
];

/// Reads a `sticks` weapon entry.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<SticksCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a sticks weapon", &KEYS)?;
    let cfg = SticksCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "sticks".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "STICKS".to_string(),
        },
        step16: e.f64(w, p, "step16")?,
        out16: e.f64(w, p, "out16")?,
        spread: e.f64(w, p, "spread")?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        dmg: e.f64(w, p, "dmg")?,
        accent: e.f64_or(w, p, "accent", 1.0)?,
        knock: e.f64(w, p, "knock")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// What the engine cannot run: a stroke grid below one sixteenth, a jab that never reaches out, numbers that are not
/// finite or are negative.
pub(crate) fn check(e: &Entry, w: &SticksCfg) -> Result<(), RosterError> {
    let p = "weapon";
    if !(w.step16.is_finite() && (1.0..=64.0).contains(&w.step16)) {
        return Err(e.err(&format!("{p}.step16"), "must be a number from 1 to 64"));
    }
    if !(w.out16.is_finite() && w.out16 > 0.0 && w.out16 <= w.step16) {
        return Err(e.err(&format!("{p}.out16"), "must be above 0 and no more than step16"));
    }
    for (key, x) in [
        ("spread", w.spread),
        ("base", w.base),
        ("grow", w.grow),
        ("max_reach", w.max_reach),
        ("dmg", w.dmg),
        ("accent", w.accent),
        ("knock", w.knock),
    ] {
        if !x.is_finite() || x < 0.0 {
            return Err(e.err(&format!("{p}.{key}"), "must be a finite number, not negative"));
        }
    }
    Ok(())
}

/// What `Ball::new` reads from this weapon: nothing of its own.
pub fn ball_start(_w: &SticksCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste.
pub(crate) fn main16(w: &SticksCfg) -> Option<f64> {
    Some(crate::py::round_half_even(w.step16))
}
