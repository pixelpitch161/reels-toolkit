//! Weapon kind `walk`, CONTRABASS's WALKING BASS: its roster settings. `fight.py` 3965 to 4040 (`walk_setup`,
//! `walk_blocked`, `walk_tick`, `walk_step`, `_step_land`), and the walker's side of the clash in `move` (3190 to 3208,
//! 3227 to 3231).
//!
//! A stub until CONTRABASS's port: [`PORTED`] is false, so a `walk` weapon loads as `WeaponCfg::Unsupported("walk")`
//! and no fight can have it. The port fills [`WalkCfg`] with the fields `fight.py` reads (`w[name]`, and `w.get(name,
//! default)` with its default), [`parse`] and [`check`] with the roster checks (every number the kind cannot run is
//! refused, naming the fighter and the field), [`ball_start`] and [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `walk`. While false the roster reads it as `WeaponCfg::Unsupported("walk")`,
/// which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `walk`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WalkCfg {}

/// Reads a `walk` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<WalkCfg, RosterError> {
    Ok(WalkCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &WalkCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &WalkCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &WalkCfg) -> Option<f64> {
    None
}
