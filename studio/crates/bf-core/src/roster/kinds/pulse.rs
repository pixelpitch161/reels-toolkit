//! Weapon kind `pulse`, DRUMS's the drumhead: its roster settings. `fight.py` 411 to 424 (`fire_pulse`), the pulse
//! counter of `run()` (5946 to 5963) and its rings in `effects_step` (kind `pulse`, with `grow` and `clears`).
//!
//! A stub until DRUMS's port (milestone 4): [`PORTED`] is false, so a weapon of kind `pulse` loads as
//! `WeaponCfg::Unsupported("pulse")` and no fight can have it. The port fills [`PulseCfg`] with the fields `fight.py`
//! reads (`w[name]`, and `w.get(name, default)` with its default), [`parse`] and [`check`] with the roster checks
//! (every number the kind cannot run is refused, naming the fighter and the field), [`ball_start`] and [`main16`], and
//! then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `pulse`. While false the roster reads it as
/// `WeaponCfg::Unsupported("pulse")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `pulse`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PulseCfg {}

/// Reads a `pulse` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<PulseCfg, RosterError> {
    Ok(PulseCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &PulseCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &PulseCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &PulseCfg) -> Option<f64> {
    None
}
