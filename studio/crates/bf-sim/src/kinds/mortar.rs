//! Weapon kind `mortar`, EUPHONIUM's BELL MORTAR: `fight.py` 2659 to 2766 (`mortar_setup`, `predict`, `_shell`,
//! `mortar_fire`, `mortar_tick`, `start_barrage`, `shells_step`), the shells a lance swats (`shell_height`,
//! `swat_shells` 1243 to 1271), and the ability `barrage` (`trigger` 2900 to 2901).
//!
//! A stub until EUPHONIUM's port. Every function here is already called where `Fight.run()` calls its Python namesake
//! (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind [`is`], which is false until the roster reads
//! the kind (`roster::kinds::mortar::PORTED`). So a stub body is never reached in a fight; with `Fight::trace_sockets`
//! on, one that is reached logs its name. The port fills the bodies with the Python, every name and argument as
//! `fight.py` has it, every draw from the fight's generator in Python's order and number, and touches no other engine
//! file.

use crate::fight::Fight;
use crate::geom::V2;
use crate::record::Frames;
use crate::roster::{AbilityCfg, MortarCfg, WeaponCfg};

/// Weapon kind `mortar`'s per-fight state (`Fight::mortar`): what `fight.py` hangs on the ball or the fight for this
/// kind, indexed by fighter where it is per ball. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// One shell in the air or landed (`_shell` 2686 to 2690): an entry of `Fight::shells`. Empty until the port.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shell {}

/// `B.w['kind'] == 'mortar'`: fighter `k`'s weapon is this kind (false for every fighter until the roster reads the
/// kind).
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Mortar(_))
}

/// `shelling` (`run()` 5810): a fighter is a mortar, or either fighter has a `barrage` ability. Its shells then land
/// (`shells_step`) and the record has `SHELLS`.
pub fn shelling(f: &Fight) -> bool {
    (0..2).any(|k| is(f, k) || crate::abilities::has(f, k, &crate::roster::AbilityKind::Barrage))
}

/// `mortar_setup(k)` (2668 to 2671), from `run()` 5813 to 5814 (after 5810 to 5812 made `shells`, `next_salvo`,
/// `salvo_landed`: `Fight::shells` and its neighbours, empty by default)
pub fn setup(f: &mut Fight, _k: usize) {
    f.socket("stub:kinds::mortar::setup");
}

/// `mortar_tick(k, k16)` (2712 to 2720), from `run()` 6025 to 6029
pub fn tick(f: &mut Fight, _k: usize, _k16: i64) {
    f.socket("stub:kinds::mortar::tick");
}

/// `shells_step()` (2741 to 2766), from `run()` 6157 to 6158 (`if shelling`: a mortar or a `barrage`; see [`shelling`])
pub fn shells_step(f: &mut Fight) {
    f.socket("stub:kinds::mortar::shells_step");
}

/// `swat_shells(k, a, d, L_)` (1251 to 1271), from `thrust_step` 755 to 756 (a live lance, the rule `swat_shells`,
/// shells in the air)
pub fn swat_shells(f: &mut Fight, _k: usize, _a: V2, _d: V2, _l: f64) {
    f.socket("stub:kinds::mortar::swat_shells");
}

/// `mortar_fire(k, w=None, n=None)` (2691 to 2711); IDENTITY CRISIS's signature copy fires one with the enemy's weapon
/// and volley (2820 to 2821)
pub fn mortar_fire(f: &mut Fight, _k: usize, _w: Option<&MortarCfg>, _n: Option<i64>) {
    f.socket("stub:kinds::mortar::mortar_fire");
}

/// `trigger`, kind `barrage` (2900 to 2901): `start_barrage(k, ab)` (2721 to 2740)
pub fn start_barrage(f: &mut Fight, _k: usize, _ab: &AbilityCfg) {
    f.socket("stub:kinds::mortar::start_barrage");
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5816 to 5817: `SHELLS` when shelling.
/// None until the port.
pub fn record_keys(_f: &Fight) -> &'static [&'static str] {
    &[]
}

/// This kind's rows of one frame: `snap`, `SHELLS` (5564 to 5568), into `out` (only the keys [`record_keys`] gave).
pub(crate) fn snap(f: &Fight, _out: &mut Frames) {
    f.socket("stub:kinds::mortar::snap");
}
