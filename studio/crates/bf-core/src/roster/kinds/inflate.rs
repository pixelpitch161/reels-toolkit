//! Weapon kind `inflate`, BAGPIPES's BAG INFLATE: its roster settings. `fight.py` 1763 to 1867 (`inflate_setup`,
//! `inflate_start`, `inflate_step`, `inflate_clash`), its branch of `grid16_fire` (1487 to 1507), and the per-ball
//! radius, mass and speed it gives `move` (3165, 3179, 3197, 3209 to 3216, 3233 to 3234).
//!
//! A stub until BAGPIPES's port (milestone 4): [`PORTED`] is false, so a weapon of kind `inflate` loads as
//! `WeaponCfg::Unsupported("inflate")` and no fight can have it. The port fills [`InflateCfg`] with the fields
//! `fight.py` reads (`w[name]`, and `w.get(name, default)` with its default), [`parse`] and [`check`] with the roster
//! checks (every number the kind cannot run is refused, naming the fighter and the field), [`ball_start`] and
//! [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `inflate`. While false the roster reads it as
/// `WeaponCfg::Unsupported("inflate")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `inflate`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InflateCfg {}

/// Reads a `inflate` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<InflateCfg, RosterError> {
    Ok(InflateCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &InflateCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &InflateCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &InflateCfg) -> Option<f64> {
    None
}
