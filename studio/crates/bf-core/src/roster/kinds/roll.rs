//! Weapon kind `roll`, PIANO's PIANO ROLL: its roster settings. `fight.py` 5164 to 5283 (`ROLL_DEFAULT`, `roll_setup`,
//! `roll_lane`, `roll_tick`, `fire_row`, `start_runoff`, `hold_step`), the projectiles (`spawn_key` 370 to 378, their
//! flight and hits in `move` 3235 to 3278) and their `K` rows in `snap` (5509 to 5514).
//!
//! A stub until PIANO's port: [`PORTED`] is false, so a `roll` weapon loads as `WeaponCfg::Unsupported("roll")` and no
//! fight can have it. The port fills [`RollCfg`] with the fields `fight.py` reads (`w[name]`, and `w.get(name,
//! default)` with its default), [`parse`] and [`check`] with the roster checks (every number the kind cannot run is
//! refused, naming the fighter and the field), [`ball_start`] and [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `roll`. While false the roster reads it as `WeaponCfg::Unsupported("roll")`,
/// which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `roll`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RollCfg {}

/// Reads a `roll` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<RollCfg, RosterError> {
    Ok(RollCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &RollCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &RollCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &RollCfg) -> Option<f64> {
    None
}
