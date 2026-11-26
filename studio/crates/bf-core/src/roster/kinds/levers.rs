//! Weapon kind `levers`, OCTOBASS's LEVER PUNCH: its roster settings. A Studio kind (the Python kit never had it),
//! so the engine's `bf_sim::kinds::levers` is its source of truth.
//!
//! Every `every16` sixteenths (haste shortens it) the octobass fires a volley of lever punches, one every `gap16`
//! sixteenths: `n0` arms at the start, `grow` more per landed punch, up to `n_max`, side by side `spread` degrees
//! apart. Each felt fist shoots straight at the enemy over `out16` sixteenths, up to `reach` ball radii beyond the
//! ball's edge; one that touches the enemy stops there (it never passes through), hits once for `dmg` and a knock of
//! `knock`, holds for `hold16` sixteenths, then pulls back over `back16`.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `levers`.
pub const PORTED: bool = true;

/// Weapon kind `levers`'s typed settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LeversCfg {
    /// The picture's style (default `levers`).
    pub style: String,
    /// The weapon's name (default `LEVERS`).
    pub name: String,
    /// Sixteenths between two volleys.
    pub every16: f64,
    /// Sixteenths between two punches of a volley.
    pub gap16: f64,
    /// Arms a volley has at the start.
    pub n0: f64,
    /// Arms added per landed punch.
    pub grow: f64,
    /// A volley never has more arms than this.
    pub n_max: f64,
    /// Degrees between two arms of a volley.
    pub spread: f64,
    /// Sixteenths a punch takes to reach out.
    pub out16: f64,
    /// A punch's reach beyond the ball's edge, in ball radii.
    pub reach: f64,
    /// Sixteenths a fist holds where it stopped (or at full reach).
    pub hold16: f64,
    /// Sixteenths it takes to pull back.
    pub back16: f64,
    /// Damage of a punch.
    pub dmg: f64,
    /// Knockback of a punch.
    pub knock: f64,
}

const KEYS: [&str; 15] = [
    "kind", "style", "name", "every16", "gap16", "n0", "grow", "n_max", "spread", "out16", "reach", "hold16", "back16",
    "dmg", "knock",
];

/// Reads a `levers` weapon entry.
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<LeversCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a levers weapon", &KEYS)?;
    let cfg = LeversCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "levers".to_string(),
        },
        name: match w.get("name") {
            Some(_) => e.string(w, p, "name")?,
            None => "LEVERS".to_string(),
        },
        every16: e.f64(w, p, "every16")?,
        gap16: e.f64(w, p, "gap16")?,
        n0: e.f64(w, p, "n0")?,
        grow: e.f64(w, p, "grow")?,
        n_max: e.f64(w, p, "n_max")?,
        spread: e.f64(w, p, "spread")?,
        out16: e.f64(w, p, "out16")?,
        reach: e.f64(w, p, "reach")?,
        hold16: e.f64(w, p, "hold16")?,
        back16: e.f64(w, p, "back16")?,
        dmg: e.f64(w, p, "dmg")?,
        knock: e.f64(w, p, "knock")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// What the engine cannot run: a volley grid below one sixteenth, no arms, a punch that never reaches out, numbers
/// that are not finite or are negative.
pub(crate) fn check(e: &Entry, w: &LeversCfg) -> Result<(), RosterError> {
    let p = "weapon";
    if !(w.every16.is_finite() && (1.0..=64.0).contains(&w.every16)) {
        return Err(e.err(&format!("{p}.every16"), "must be a number from 1 to 64"));
    }
    if !(w.n0.is_finite() && w.n0 >= 1.0 && w.n_max >= w.n0 && w.n_max <= 7.0) {
        return Err(e.err(&format!("{p}.n0"), "must be at least 1, with n_max from n0 to 7"));
    }
    if !(w.out16.is_finite() && w.out16 > 0.0 && w.back16.is_finite() && w.back16 > 0.0) {
        return Err(e.err(&format!("{p}.out16"), "out16 and back16 must be above 0"));
    }
    for (key, x) in [
        ("gap16", w.gap16),
        ("grow", w.grow),
        ("spread", w.spread),
        ("reach", w.reach),
        ("hold16", w.hold16),
        ("dmg", w.dmg),
        ("knock", w.knock),
    ] {
        if !x.is_finite() || x < 0.0 {
            return Err(e.err(&format!("{p}.{key}"), "must be a finite number, not negative"));
        }
    }
    Ok(())
}

/// What `Ball::new` reads from this weapon: nothing of its own.
pub fn ball_start(_w: &LeversCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste.
pub(crate) fn main16(w: &LeversCfg) -> Option<f64> {
    Some(crate::py::round_half_even(w.every16))
}
