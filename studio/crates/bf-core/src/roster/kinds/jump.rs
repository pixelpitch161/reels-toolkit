//! Weapon kind `jump`, CLARINET's TWELFTH JUMP: its roster settings. `fight.py` 2338 to 2424 (`jump_setup`, `_jump`,
//! `jump_tick`), and the ability `rhapsody` (`trigger` 2930 to 2932; its chain runs in `jump_tick`).
//!
//! A stub until CLARINET's port: [`PORTED`] is false, so a `jump` weapon loads as `WeaponCfg::Unsupported("jump")` and
//! no fight can have it. The port fills [`JumpCfg`] with the fields `fight.py` reads (`w[name]`, and `w.get(name,
//! default)` with its default), [`parse`] and [`check`] with the roster checks (every number the kind cannot run is
//! refused, naming the fighter and the field), [`ball_start`] and [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `jump`. While false the roster reads it as `WeaponCfg::Unsupported("jump")`,
/// which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `jump`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JumpCfg {}

/// Reads a `jump` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<JumpCfg, RosterError> {
    Ok(JumpCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &JumpCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &JumpCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &JumpCfg) -> Option<f64> {
    None
}
