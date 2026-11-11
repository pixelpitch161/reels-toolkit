//! Weapon kind `rake`, ACOUSTIC's SIX-STRING RAKE: its roster settings. `fight.py` 497 to 700: `fire_rake` (528 to
//! 545), `aim_blend`, `rake_lateral`, `sweep_front`, the rake branches of `dash_cone_step` (457 to 501) and
//! `surf_rake_step` (585 to 700); the `dash`, `cone`, `surf` and `spout` branches belong to older kinds.
//!
//! A stub until ACOUSTIC's port: [`PORTED`] is false, so a `rake` weapon loads as `WeaponCfg::Unsupported("rake")` and
//! no fight can have it. The port fills [`RakeCfg`] with the fields `fight.py` reads (`w[name]`, and `w.get(name,
//! default)` with its default), [`parse`] and [`check`] with the roster checks (every number the kind cannot run is
//! refused, naming the fighter and the field), [`ball_start`] and [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `rake`. While false the roster reads it as `WeaponCfg::Unsupported("rake")`,
/// which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `rake`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RakeCfg {}

/// Reads a `rake` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<RakeCfg, RosterError> {
    Ok(RakeCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &RakeCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &RakeCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &RakeCfg) -> Option<f64> {
    None
}
