//! Weapon kind `scratch_floor`, TURNTABLES's THE RING IS THE RECORD: its roster settings. `fight.py` 1713 to 1762
//! (`scratch_beat`, `scratch_step`).
//!
//! A stub until TURNTABLES's port (milestone 4): [`PORTED`] is false, so a weapon of kind `scratch_floor` loads as
//! `WeaponCfg::Unsupported("scratch_floor")` and no fight can have it. The port fills [`ScratchFloorCfg`] with the
//! fields `fight.py` reads (`w[name]`, and `w.get(name, default)` with its default), [`parse`] and [`check`] with the
//! roster checks (every number the kind cannot run is refused, naming the fighter and the field), [`ball_start`] and
//! [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `scratch_floor`. While false the roster reads it as
/// `WeaponCfg::Unsupported("scratch_floor")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `scratch_floor`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScratchFloorCfg {}

/// Reads a `scratch_floor` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<ScratchFloorCfg, RosterError> {
    Ok(ScratchFloorCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &ScratchFloorCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &ScratchFloorCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &ScratchFloorCfg) -> Option<f64> {
    None
}
