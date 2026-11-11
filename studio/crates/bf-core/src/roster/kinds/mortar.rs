//! Weapon kind `mortar`, EUPHONIUM's BELL MORTAR: its roster settings. `fight.py` 2659 to 2766 (`mortar_setup`,
//! `predict`, `_shell`, `mortar_fire`, `mortar_tick`, `start_barrage`, `shells_step`), the shells a lance swats
//! (`shell_height`, `swat_shells` 1243 to 1271), and the ability `barrage` (`trigger` 2900 to 2901).
//!
//! A stub until EUPHONIUM's port: [`PORTED`] is false, so a `mortar` weapon loads as `WeaponCfg::Unsupported("mortar")`
//! and no fight can have it. The port fills [`MortarCfg`] with the fields `fight.py` reads (`w[name]`, and `w.get(name,
//! default)` with its default), [`parse`] and [`check`] with the roster checks (every number the kind cannot run is
//! refused, naming the fighter and the field), [`ball_start`] and [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `mortar`. While false the roster reads it as `WeaponCfg::Unsupported("mortar")`,
/// which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `mortar`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MortarCfg {}

/// Reads a `mortar` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<MortarCfg, RosterError> {
    Ok(MortarCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &MortarCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &MortarCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &MortarCfg) -> Option<f64> {
    None
}
