//! Ability kind `slap` (CONTRABASS's SLAP BASS): `fight.py` `trigger` 2946 to 2947; 4202 to 4219 (`slap_ready`,
//! `start_slap`); the gate in `abilities_step` 3043 to 3045.
//!
//! The string is pulled and let go so it snaps onto the fingerboard: a sharp ring of kind `slap` (`speed`, `radius`,
//! `dmg`, `knock`) that clears enemy projectiles. It waits for its moment: the enemy within `ready` beyond contact,
//! or the fighter held in a grapple; then it fires and breaks the grab (the holder lets go first). Event (`slap`,
//! broke 0 / 1). It draws nothing. A copied slap (IDENTITY CRISIS) runs the same code for the copier's ball.

use bf_core::event::Value;

use crate::abilities::param;
use crate::fight::{Fight, Wave};
use crate::geom;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::slap`): a slap keeps none (its ring is one of `Fight::waves`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `grapple_release(k, why)` (`fight.py` 4102 to 4111): the grappler `k` lets go of the ball it holds. The grapple is
/// CELLO's kind (Task 24) and not ported: no fight can have a grappler yet, so no ball is ever `grabbed` and this is
/// never reached. With `Fight::trace_sockets` on it logs the stub.
pub(crate) fn grapple_release(f: &mut Fight, _k: usize, _why: &str) {
    f.socket("stub:kinds::grapple::grapple_release");
}

/// `trigger`, kind `slap`: `trigger` 2946 to 2947, `start_slap(k, ab)` (4211 to 4218).
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let broke = i64::from(f.balls[k].grabbed);
    if broke != 0 {
        grapple_release(f, 1 - k, "slap");
    }
    let wave = Wave {
        owner: k,
        c: f.balls[k].p,
        r: f.rules.r,
        speed: param(ab, "speed"),
        rmax: param(ab, "radius"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        kind: "slap".to_string(),
        hit: false,
        name: ab.name.clone(),
        t0: f.t,
        slow: None,
        stun: None,
        detune: None,
        // no `grow`; `clears` takes its default (True)
        grow: false,
        clears: true,
    };
    f.waves.push(wave);
    f.ev("slap", k as i64, vec![Value::Int(broke)]);
}

/// `slap_ready(k, ab)` (4207 to 4209): the gate in `abilities_step` 3043 to 3045. Held, or the enemy within `ready`
/// beyond contact (`float(np.hypot(*(O.p - B.p))) - 2 * R <= ab['ready']`).
pub fn ready(f: &Fight, k: usize, ab: &AbilityCfg) -> bool {
    f.balls[k].grabbed || geom::hypot(geom::sub(f.balls[1 - k].p, f.balls[k].p)) - 2.0 * f.rules.r <= param(ab, "ready")
}
