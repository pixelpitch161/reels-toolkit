//! Weapon kind `surf`, UKULELE's SURF'S UP: its roster settings. `fight.py` 502 to 527 (`surf_dist`, `fire_surf`), the
//! `surfs` of `surf_rake_step` (588 to 609), on the `dc` 16th counter of `run()` (5964 to 5974, shared with the rake).
//!
//! A stub until UKULELE's port (milestone 4): [`PORTED`] is false, so a weapon of kind `surf` loads as
//! `WeaponCfg::Unsupported("surf")` and no fight can have it. The port fills [`SurfCfg`] with the fields `fight.py`
//! reads (`w[name]`, and `w.get(name, default)` with its default), [`parse`] and [`check`] with the roster checks
//! (every number the kind cannot run is refused, naming the fighter and the field), [`ball_start`] and [`main16`], and
//! then sets [`PORTED`].

use serde_json::Value as Json;

use crate::roster::{BallStart, Entry, RosterError};

/// Whether the engine runs weapon kind `surf`. While false the roster reads it as
/// `WeaponCfg::Unsupported("surf")`, which the engine refuses in words.
pub const PORTED: bool = false;

/// Weapon kind `surf`'s typed settings (empty until the port).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfCfg {}

/// Reads a `surf` weapon entry (the roster's `weapon`); called only when [`PORTED`] is true.
pub(crate) fn parse(_e: &Entry, _w: &Json) -> Result<SurfCfg, RosterError> {
    Ok(SurfCfg {})
}

/// [`parse`]'s checks made again on typed settings changed in code (`FighterCfg::check`).
pub(crate) fn check(_e: &Entry, _w: &SurfCfg) -> Result<(), RosterError> {
    Ok(())
}

/// What `Ball.__init__` reads from this weapon (`fight.py` 231 to 258: `base`, `frets0`, `rest`, `n0`), each at its
/// Python default until the port.
pub fn ball_start(_w: &SurfCfg) -> BallStart {
    BallStart::default()
}

/// The weapon's own 16th grid before any haste (`int(round(4 * every))`, as a float), when a haste divides it: the
/// roster then checks the haste's factor against it. `None` for a kind without one.
pub(crate) fn main16(_w: &SurfCfg) -> Option<f64> {
    None
}
