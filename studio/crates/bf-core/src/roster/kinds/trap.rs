//! Weapon kind `trap`, HARP's the strings: its roster settings. `fight.py` 1324 to 1332 (`hazard_beat`, shared with
//! `line`), 1363 to 1392 (`lay_string`) and the strings' half of `hazards_step` (1417 to 1434).
//!
//! A stub until HARP's port (milestone 4): [`PORTED`] is false, so a weapon of kind `trap` loads as
//! `WeaponCfg::Unsupported("trap")` and no fight can have it. The port fills [`TrapCfg`] with the fields `fight.py`
//! reads (`w[name]`, and `w.get(name, default)` with its default), [`parse`] and [`check`] with the roster checks
//! (every number the kind cannot run is refused, naming the fighter and the field), [`ball_start`] and [`main16`], and
//! then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `trap`. While false the roster reads it as
/// `WeaponCfg::Unsupported("trap")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `trap`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrapCfg {}

/// Reads a `trap` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<TrapCfg, RosterError> {
    Ok(TrapCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &TrapCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &TrapCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &TrapCfg) -> Option<f64> {
    None
}
