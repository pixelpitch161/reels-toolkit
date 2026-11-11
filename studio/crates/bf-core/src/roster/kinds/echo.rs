//! Weapon kind `echo`, HORN's the echo call: its roster settings. `fight.py` 2196 to 2281 (`echo_setup`, `echo_pos`,
//! `_echo_blast`, `echo_tick`, `echo_step`).
//!
//! A stub until HORN's port: [`PORTED`] is false, so a `echo` weapon loads as `WeaponCfg::Unsupported("echo")` and no
//! fight can have it. The port fills [`EchoCfg`] with the fields `fight.py` reads (`w[name]`, and `w.get(name,
//! default)` with its default), [`parse`] and [`check`] with the roster checks (every number the kind cannot run is
//! refused, naming the fighter and the field), [`ball_start`] and [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `echo`. While false the roster reads it as `WeaponCfg::Unsupported("echo")`,
/// which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `echo`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EchoCfg {}

/// Reads a `echo` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<EchoCfg, RosterError> {
    Ok(EchoCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &EchoCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &EchoCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &EchoCfg) -> Option<f64> {
    None
}
