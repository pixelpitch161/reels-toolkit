//! Weapon kind `surf`, UKULELE's SURF'S UP: its roster settings, and the closed-form ride the fight and the look
//! share. `fight.py` 502 to 527 (`surf_dist`, `fire_surf`), the `surfs` of `surf_rake_step` (588 to 609), on the `dc`
//! 16th counter of `run()` (5964 to 5974, shared with the rake).
//!
//! Every `every` beats on the dash / cone 16th grid (haste divides it) the strum raises a wave the ball rides at the
//! enemy: a speed burst (`p0 + p_h * h`, `h` the wave's height `blade_len`; x `sub_power` off the grid) and a front
//! `w0 + w_h * h` wide that crashes into the enemy once (`dmg`, x `sub_dmg` off the grid; `knock` x (0.8 + 0.5 h));
//! a landed wave on the grid adds `grow` to the height, up to `max_h`. A wave lives `life` seconds or until its front
//! reaches the ropes.

use serde_json::Value as Json;

use crate::dmath;
use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `surf`.
pub const PORTED: bool = true;

/// Weapon kind `surf`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfCfg {
    /// The picture's style (default: the kind name; the fight never reads it).
    pub style: String,
    /// Beats between two waves on the dash / cone 16th grid.
    pub every: f64,
    /// The wave's starting height (`blade_len`; `w.get('base', 0.0)`).
    pub base: f64,
    /// Height added per landed wave on the grid.
    pub grow: f64,
    /// The height never grows past this.
    pub max_h: f64,
    /// The ride's speed burst at no height.
    pub p0: f64,
    /// The burst added per unit of height.
    pub p_h: f64,
    /// The front's half width at no height.
    pub w0: f64,
    /// The half width added per unit of height.
    pub w_h: f64,
    /// Seconds a wave lives at most.
    pub life: f64,
    /// How much of the aim is the enemy's direction (`w.get('aim_k', 1.0)`).
    pub aim_k: f64,
    /// Damage of a wave on the grid.
    pub dmg: f64,
    /// The damage factor off it (`w.get('sub_dmg', 0.5)`).
    pub sub_dmg: f64,
    /// The burst's factor off it (`w.get('sub_power', 1.0)`).
    pub sub_power: f64,
    /// Knockback (x (0.8 + 0.5 h)).
    pub knock: f64,
}

/// The keys a surf entry may hold.
const SURF_KEYS: [&str; 16] = [
    "kind",
    "style",
    "every",
    "base",
    "grow",
    "max_h",
    "p0",
    "p_h",
    "w0",
    "w_h",
    "life",
    "aim_k",
    "dmg",
    "sub_dmg",
    "sub_power",
    "knock",
];

/// Reads a `surf` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<SurfCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a surf weapon", &SURF_KEYS)?;
    let cfg = SurfCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "surf".to_string(),
        },
        every: e.grid(w, p, "every")?,
        base: e.f64_or(w, p, "base", 0.0)?,
        grow: e.f64(w, p, "grow")?,
        max_h: e.f64(w, p, "max_h")?,
        p0: e.f64(w, p, "p0")?,
        p_h: e.f64(w, p, "p_h")?,
        w0: e.f64(w, p, "w0")?,
        w_h: e.f64(w, p, "w_h")?,
        life: e.f64(w, p, "life")?,
        aim_k: e.f64_or(w, p, "aim_k", 1.0)?,
        dmg: e.f64(w, p, "dmg")?,
        sub_dmg: e.f64_or(w, p, "sub_dmg", 0.5)?,
        sub_power: e.f64_or(w, p, "sub_power", 1.0)?,
        knock: e.f64(w, p, "knock")?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code: a grid that rounds to no 16th raises at the
/// first 16th (`dc_k % dc_main`), and a wave of infinite life never ends inside the ropes' test alone.
pub(crate) fn check(e: &Entry, w: &SurfCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    if crate::py::round_half_even(4.0 * w.every) == 0.0 {
        return Err(e.err(
            "weapon.every",
            format!(
                "{} beats rounds to no 16th note (the wave's grid divides by it)",
                w.every
            ),
        ));
    }
    if !w.life.is_finite() {
        return Err(e.err("weapon.life", format!("{:e} is not a finite number of seconds", w.life)));
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon: only `base`, the wave's starting height.
pub fn ball_start(w: &SurfCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste (`dc_main = int(round(4 * every))`).
pub(crate) fn main16(w: &SurfCfg) -> Option<f64> {
    Some(crate::py::round_half_even(4.0 * w.every))
}

/// `Fight.surf_dist(tau, V, power, kt)` (502 to 504): how far a wave's front has travelled `tau` s after the strum.
pub fn surf_dist(tau: f64, v: f64, power: f64, kt: f64) -> f64 {
    v * (tau + power * kt * (1.0 - dmath::exp(-tau / kt)))
}
