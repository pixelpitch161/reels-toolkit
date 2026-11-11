//! Weapon kind `spotlight`, ALTO SAX's SPOTLIGHT SOLO: its roster settings. `fight.py` 4369 to 4512 (`spot_setup`,
//! `_spot_line`, `spot_tick`, `spot_pos`, `spot_step`, `start_encore`), and the abilities `octave`, `altissimo` and
//! `encore` (`trigger` 2958 to 2965).
//!
//! A stub until ALTO SAX's port: [`PORTED`] is false, so a `spotlight` weapon loads as
//! `WeaponCfg::Unsupported("spotlight")` and no fight can have it. The port fills [`SpotlightCfg`] with the fields
//! `fight.py` reads (`w[name]`, and `w.get(name, default)` with its default), [`parse`] and [`check`] with the roster
//! checks (every number the kind cannot run is refused, naming the fighter and the field), [`ball_start`] and
//! [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `spotlight`. While false the roster reads it as
/// `WeaponCfg::Unsupported("spotlight")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `spotlight`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpotlightCfg {}

/// Reads a `spotlight` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<SpotlightCfg, RosterError> {
    Ok(SpotlightCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &SpotlightCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &SpotlightCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &SpotlightCfg) -> Option<f64> {
    None
}
