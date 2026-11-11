//! Weapon kind `tune`, OBOE's TUNING A: its roster settings. `fight.py` 2425 to 2525 (`tune_setup`, `_detune`,
//! `tune_tick`, `tune_step`), and the ability `endless` (`trigger` 2884 to 2888).
//!
//! A stub until OBOE's port: [`PORTED`] is false, so a `tune` weapon loads as `WeaponCfg::Unsupported("tune")` and no
//! fight can have it. The port fills [`TuneCfg`] with the fields `fight.py` reads (`w[name]`, and `w.get(name,
//! default)` with its default), [`parse`] and [`check`] with the roster checks (every number the kind cannot run is
//! refused, naming the fighter and the field), [`ball_start`] and [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `tune`. While false the roster reads it as `WeaponCfg::Unsupported("tune")`,
/// which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `tune`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TuneCfg {}

/// Reads a `tune` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<TuneCfg, RosterError> {
    Ok(TuneCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &TuneCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &TuneCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &TuneCfg) -> Option<f64> {
    None
}
