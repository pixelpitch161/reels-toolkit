//! Weapon kind `rests`, VIOLA's COUNTING RESTS: its roster settings. `fight.py` 5021 to 5107 (`rests_setup`,
//! `fire_note`, `rests_tick`, `_crush`), 5137 to 5181 (`notes_step`), the resting `guard` in `damage` (342 to 343), and
//! the abilities `double_stop` (5108 to 5117) and `melody` (`trigger` 2923 to 2927), which run in this kind's code.
//!
//! A stub until VIOLA's port (milestone 4): [`PORTED`] is false, so a weapon of kind `rests` loads as
//! `WeaponCfg::Unsupported("rests")` and no fight can have it. The port fills [`RestsCfg`] with the fields `fight.py`
//! reads (`w[name]`, and `w.get(name, default)` with its default), [`parse`] and [`check`] with the roster checks
//! (every number the kind cannot run is refused, naming the fighter and the field), [`ball_start`] and [`main16`], and
//! then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `rests`. While false the roster reads it as
/// `WeaponCfg::Unsupported("rests")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `rests`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RestsCfg {}

/// Reads a `rests` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<RestsCfg, RosterError> {
    Ok(RestsCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &RestsCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &RestsCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &RestsCfg) -> Option<f64> {
    None
}
