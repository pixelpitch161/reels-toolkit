//! Weapon kind `grapple`, CELLO's THE EMBRACE: its roster settings. `fight.py` 4041 to 4201 (`GRAB_STATES`,
//! `grapple_setup`, `grapple_busy`, `grapple_tick`, `_end_rush`, `_nearest_rope_angle`, `grapple_release`,
//! `grapple_step`).
//!
//! A stub until CELLO's port: [`PORTED`] is false, so a `grapple` weapon loads as `WeaponCfg::Unsupported("grapple")`
//! and no fight can have it. The port fills [`GrappleCfg`] with the fields `fight.py` reads (`w[name]`, and
//! `w.get(name, default)` with its default), [`parse`] and [`check`] with the roster checks (every number the kind
//! cannot run is refused, naming the fighter and the field), [`ball_start`] and [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `grapple`. While false the roster reads it as
/// `WeaponCfg::Unsupported("grapple")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `grapple`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GrappleCfg {}

/// Reads a `grapple` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<GrappleCfg, RosterError> {
    Ok(GrappleCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &GrappleCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &GrappleCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &GrappleCfg) -> Option<f64> {
    None
}
