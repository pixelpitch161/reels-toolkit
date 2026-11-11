//! Weapon kind `drill`, SOUSAPHONE's DRILL SET: its roster settings. `fight.py` 4758 to 5005 (`drill_setup`,
//! `drill_blocked`, `_drill_grow`, `drill_tick`, `_drill_blast`, `_capsule_hit`, `_trample_baskets`, `drill_step`,
//! `start_flash`, `start_front`), and the abilities `flash_spin` and `company_front` (`trigger` 2977 to 2982).
//!
//! A stub until SOUSAPHONE's port: [`PORTED`] is false, so a `drill` weapon loads as `WeaponCfg::Unsupported("drill")`
//! and no fight can have it. The port fills [`DrillCfg`] with the fields `fight.py` reads (`w[name]`, and `w.get(name,
//! default)` with its default), [`parse`] and [`check`] with the roster checks (every number the kind cannot run is
//! refused, naming the fighter and the field), [`ball_start`] and [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `drill`. While false the roster reads it as `WeaponCfg::Unsupported("drill")`,
/// which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `drill`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrillCfg {}

/// Reads a `drill` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<DrillCfg, RosterError> {
    Ok(DrillCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &DrillCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &DrillCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &DrillCfg) -> Option<f64> {
    None
}
