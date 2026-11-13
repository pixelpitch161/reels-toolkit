//! Weapon kind `wrecking`, BARI SAX's WRECKING BALL: its roster settings. `fight.py` 3594 to 3786 (`wreck_setup`,
//! `_wreck_aim`, `wreck_tick`, `_sails_over`, `wreck_step`) and the ability `offbeat` (SKANK, `trigger` 2911 to 2914),
//! whose cycles run in `wreck_tick`.
//!
//! A stub until BARI SAX's port (milestone 4): [`PORTED`] is false, so a weapon of kind `wrecking` loads as
//! `WeaponCfg::Unsupported("wrecking")` and no fight can have it. The port fills [`WreckingCfg`] with the fields
//! `fight.py` reads (`w[name]`, and `w.get(name, default)` with its default), [`parse`] and [`check`] with the roster
//! checks (every number the kind cannot run is refused, naming the fighter and the field), [`ball_start`] and
//! [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `wrecking`. While false the roster reads it as
/// `WeaponCfg::Unsupported("wrecking")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `wrecking`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WreckingCfg {}

/// Reads a `wrecking` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<WreckingCfg, RosterError> {
    Ok(WreckingCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &WreckingCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &WreckingCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &WreckingCfg) -> Option<f64> {
    None
}
