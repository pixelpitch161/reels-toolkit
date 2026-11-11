//! Weapon kind `lightning`, ELECTRIC's LIGHTNING RIFF: its roster settings. `fight.py` 3845 to 3922 (`POSTS`,
//! `lightning_setup`, `lightning_tick`, `bolt_source`, `bolt`), and the abilities `fork` and `tapping` (`trigger` 2933
//! to 2938).
//!
//! A stub until ELECTRIC's port: [`PORTED`] is false, so a `lightning` weapon loads as
//! `WeaponCfg::Unsupported("lightning")` and no fight can have it. The port fills [`LightningCfg`] with the fields
//! `fight.py` reads (`w[name]`, and `w.get(name, default)` with its default), [`parse`] and [`check`] with the roster
//! checks (every number the kind cannot run is refused, naming the fighter and the field), [`ball_start`] and
//! [`main16`], and then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `lightning`. While false the roster reads it as
/// `WeaponCfg::Unsupported("lightning")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `lightning`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LightningCfg {}

/// Reads a `lightning` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<LightningCfg, RosterError> {
    Ok(LightningCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &LightningCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &LightningCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &LightningCfg) -> Option<f64> {
    None
}
