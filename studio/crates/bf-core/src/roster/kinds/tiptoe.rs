//! Weapon kind `tiptoe`, BASSOON's TIPTOE: its roster settings. `fight.py` 3280 to 3429 (`tiptoe_setup`, `_behind`,
//! `_hop_launch`, `_hop_land`, `tiptoe_tick`, `tiptoe_step`, `start_mountain`), and the ability `mountain` (`trigger`
//! 2917 to 2918).
//!
//! A stub until BASSOON's port: [`PORTED`] is false, so a `tiptoe` weapon loads as `WeaponCfg::Unsupported("tiptoe")`
//! and no fight can have it. The port fills [`TiptoeCfg`] with the fields `fight.py` reads (`w[name]`, and `w.get(name,
//! default)` with its default), [`parse`] and [`check`] with the roster checks (every number the kind cannot run is
//! refused, naming the fighter and the field), [`ball_start`] and [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `tiptoe`. While false the roster reads it as `WeaponCfg::Unsupported("tiptoe")`,
/// which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `tiptoe`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TiptoeCfg {}

/// Reads a `tiptoe` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<TiptoeCfg, RosterError> {
    Ok(TiptoeCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &TiptoeCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &TiptoeCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &TiptoeCfg) -> Option<f64> {
    None
}
