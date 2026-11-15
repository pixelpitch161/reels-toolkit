//! Weapon kind `wrecking`, BARI SAX's WRECKING BALL: its roster settings. `fight.py` 3594 to 3786 (`wreck_setup`,
//! `_wreck_aim`, `wreck_tick`, `_sails_over`, `wreck_step`) and the ability `offbeat` (SKANK, `trigger` 2911 to 2914),
//! whose cycles run in `wreck_tick`.

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `wrecking`.
pub const PORTED: bool = true;

/// Weapon kind `wrecking`'s typed settings. Fields without a default are read as `w[name]`.
#[derive(Clone, Debug, PartialEq)]
pub struct WreckingCfg {
    /// The picture's style (default: the kind name).
    pub style: String,
    /// The weapon's name on the card (`w.get('name')`; the fight never reads it).
    pub name: Option<String>,
    /// Beats between two cable drops on the 16th grid (`int(round(4 * every))`, at least one 16th).
    pub every: f64,
    /// The 16th of the grid the first drop falls on (`int(w.get('off16', 0))`).
    pub off16: f64,
    /// 16ths the cable takes to drop and hook.
    pub drop16: f64,
    /// 16ths the hoist back takes.
    pub lift16: f64,
    /// 16ths the swing takes (a pendulum's half period).
    pub swing16: f64,
    /// The cable's length at the start (`blade_len`).
    pub base: f64,
    /// Cable length added per landed smash.
    pub grow: f64,
    /// The cable never grows past this.
    pub max_reach: f64,
    /// The swing's widest angle, degrees.
    pub theta: f64,
    /// How much of the enemy's motion the aim leads by (`.get('lead', 1.0)`).
    pub lead: f64,
    /// The swing bottoms out this many ball diameters short of the target (`.get('short', 0.0)`).
    pub short: f64,
    /// The 16th of each bar a SKANK swing starts on (`int(w.get('skank_off16', 2))`).
    pub skank_off16: f64,
    /// A SKANK swing's `short` (`.get('skank_short', short)`).
    pub skank_short: f64,
    /// The aim turns by up to this many degrees either way, seeded (`.get('jitter', 0.0)`).
    pub jitter: f64,
    /// A smash hits an enemy within this beyond contact.
    pub hit_r: f64,
    /// A hop sails over the swing between this much of its start and its end (`.get('air_edge', 0.0)`).
    pub air_edge: f64,
    /// Damage of a smash at full speed.
    pub dmg: f64,
    /// The share of `dmg` a smash at no speed deals.
    pub dmg_min: f64,
    /// The damage grows as (cable / base) to this power (`.get('len_pow', 0.0)`).
    pub len_pow: f64,
    /// Knockback of a smash.
    pub knock: f64,
    /// #40 v2: the swing may go out this far over the ropes (`.get('overhang', 0.0)`; 0: kept inside).
    pub overhang: f64,
    /// The corner posts' limit `|x| + |z|` (`.get('corner_l1', 2 * lim + .4)`; `None`: that default).
    pub corner_l1: Option<f64>,
    /// 16ths a ball dropped on the ropes takes to be slung back in (`.get('back16', 3)`).
    pub back16: f64,
    /// It lands this far inside the limit (`.get('back_in', .25)`).
    pub back_in: f64,
    /// #39 v4: near a corner the swing's plane turns up to this many degrees (`.get('steer', 0.0)`).
    pub steer: f64,
}

/// The keys a wrecking entry may hold: `kind`, `style`, `name` and [`WreckingCfg`]'s numbers.
const WRECKING_KEYS: [&str; 28] = [
    "kind",
    "style",
    "name",
    "every",
    "off16",
    "drop16",
    "lift16",
    "swing16",
    "base",
    "grow",
    "max_reach",
    "theta",
    "lead",
    "short",
    "skank_off16",
    "skank_short",
    "jitter",
    "hit_r",
    "air_edge",
    "dmg",
    "dmg_min",
    "len_pow",
    "knock",
    "overhang",
    "corner_l1",
    "back16",
    "back_in",
    "steer",
];

/// Python's `int(x)` of a 16th count: a 64-bit engine keeps to what a float holds exactly.
fn sixteenths(e: &Entry, key: &str, x: f64) -> Result<f64, RosterError> {
    if !x.is_finite() || x.abs() > 9_007_199_254_740_992.0 {
        return Err(e.err(
            &format!("weapon.{key}"),
            format!("{x:e} sixteenths is not a whole number it can count"),
        ));
    }
    Ok(x)
}

/// Reads a `wrecking` weapon entry (the roster's `weapon`).
pub(crate) fn parse(e: &Entry, w: &Json) -> Result<WreckingCfg, RosterError> {
    let p = "weapon";
    e.only(w, p, "a wrecking weapon", &WRECKING_KEYS)?;
    let short = e.f64_or(w, p, "short", 0.0)?;
    let cfg = WreckingCfg {
        style: match w.get("style") {
            Some(_) => e.string(w, p, "style")?,
            None => "wrecking".to_string(),
        },
        name: match w.get("name") {
            Some(_) => Some(e.string(w, p, "name")?),
            None => None,
        },
        every: e.grid(w, p, "every")?,
        off16: e.f64_or(w, p, "off16", 0.0)?,
        drop16: e.f64(w, p, "drop16")?,
        lift16: e.f64(w, p, "lift16")?,
        swing16: e.f64(w, p, "swing16")?,
        base: e.f64(w, p, "base")?,
        grow: e.f64(w, p, "grow")?,
        max_reach: e.f64(w, p, "max_reach")?,
        theta: e.f64(w, p, "theta")?,
        lead: e.f64_or(w, p, "lead", 1.0)?,
        short,
        skank_off16: e.f64_or(w, p, "skank_off16", 2.0)?,
        skank_short: e.f64_or(w, p, "skank_short", short)?,
        jitter: e.f64_or(w, p, "jitter", 0.0)?,
        hit_r: e.f64(w, p, "hit_r")?,
        air_edge: e.f64_or(w, p, "air_edge", 0.0)?,
        dmg: e.f64(w, p, "dmg")?,
        dmg_min: e.f64(w, p, "dmg_min")?,
        len_pow: e.f64_or(w, p, "len_pow", 0.0)?,
        knock: e.f64(w, p, "knock")?,
        overhang: e.f64_or(w, p, "overhang", 0.0)?,
        corner_l1: match w.get("corner_l1") {
            Some(v) => Some(e.number(v, "weapon.corner_l1")?),
            None => None,
        },
        back16: e.f64_or(w, p, "back16", 3.0)?,
        back_in: e.f64_or(w, p, "back_in", 0.25)?,
        steer: e.f64_or(w, p, "steer", 0.0)?,
    };
    check(e, &cfg)?;
    Ok(cfg)
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`): the grid (Python's
/// `(k16 - off) % main16` raises on a grid that rounds to no 16th at all), the 16th counts `int()` takes and the
/// steering's number of 10-degree steps.
pub(crate) fn check(e: &Entry, w: &WreckingCfg) -> Result<(), RosterError> {
    e.grid_value("weapon", "every", w.every)?;
    if crate::py::round_half_even(4.0 * w.every) < 1.0 {
        return Err(e.err("weapon.every", format!("{} beats is less than a 16th note", w.every)));
    }
    sixteenths(e, "off16", w.off16)?;
    sixteenths(e, "skank_off16", w.skank_off16)?;
    if !w.steer.is_finite() || w.steer.abs() > 3600.0 {
        return Err(e.err("weapon.steer", format!("{} degrees is more than it can turn", w.steer)));
    }
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258): only `base`, the cable's length at the start
/// (`wreck_setup` sets `blade_len` to it again).
pub fn ball_start(w: &WreckingCfg) -> BallStart {
    BallStart {
        base: w.base,
        ..BallStart::default()
    }
}

/// The weapon's own 16th grid before any haste: `None`, no haste divides the wrecking ball's grid.
pub(crate) fn main16(_w: &WreckingCfg) -> Option<f64> {
    None
}
