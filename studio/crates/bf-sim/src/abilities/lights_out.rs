//! Ability kind `lights_out` (BASS CLARINET's LIGHTS OUT): `fight.py` `trigger` 2895 to 2896; 2767 to 2800
//! (`start_lights_out`, `lights_out_step`).
//!
//! A stub until BASS CLARINET's port. [`start`] is already called from `trigger`'s dispatch (`abilities/mod.rs`), and
//! every other function here where `Fight.run()` calls its Python namesake, behind a test that is false until the
//! roster reads the kind (`roster::abilities::lights_out::PORTED`). So a stub body is never reached in a fight; with
//! `Fight::trace_sockets` on, one that is reached logs its name. The port fills the bodies with the Python (every draw
//! from the fight's generator in Python's order and number) and touches no other engine file. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here may assume which side has the kind.

use crate::fight::Fight;
use crate::record::Frames;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::lights_out`): what `fight.py` hangs on the ball or the fight for it, indexed
/// by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `lights_out`: `trigger` 2895 to 2896.
pub fn start(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:abilities::lights_out::start");
}

/// `lights_out_step(k, dt)` (2776 to 2800), from `run()` 6110 to 6112, for both balls (a copied one too) while any
/// fighter has the kind
pub fn lights_out_step(f: &mut Fight, _k: usize, _dt: f64) {
    f.socket("stub:abilities::lights_out::lights_out_step");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5806 to 5808: `LO`. None until the port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `LO` (5562 to 5563), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:abilities::lights_out::snap");
}
