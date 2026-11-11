//! Weapon kind `roll_charge`, BASS DRUM's STEAMROLLER: its roster settings. `fight.py` 5284 to 5431 (`rollc_setup`,
//! `rollc_blocked`, `_roll_line`, `_start_roll`, `_end_roll`, `rollc_tick`, `rollc_step`), and the abilities
//! `heartbeat`, `drum_roll` and `cannons` (`trigger` 2983 to 2988; 5432 to 5494).
//!
//! A stub until BASS DRUM's port: [`PORTED`] is false, so a `roll_charge` weapon loads as
//! `WeaponCfg::Unsupported("roll_charge")` and no fight can have it. The port fills [`RollChargeCfg`] with the fields
//! `fight.py` reads (`w[name]`, and `w.get(name, default)` with its default), [`parse`] and [`check`] with the roster
//! checks (every number the kind cannot run is refused, naming the fighter and the field), [`ball_start`] and
//! [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `roll_charge`. While false the roster reads it as
/// `WeaponCfg::Unsupported("roll_charge")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `roll_charge`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RollChargeCfg {}

/// Reads a `roll_charge` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<RollChargeCfg, RosterError> {
    Ok(RollChargeCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &RollChargeCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &RollChargeCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &RollChargeCfg) -> Option<f64> {
    None
}
