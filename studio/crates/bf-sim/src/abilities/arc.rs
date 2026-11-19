//! Ability kind `arc` (PIANO's GLISSANDO): `fight.py` `trigger` 2852 to 2858; the arcs of `effects_step` 3099 to
//! 3113.
//!
//! The ball throws `n` projectiles one after another over `beats` beats, in a fan `sweep` degrees wide centred on
//! where the enemy stood when the ability fired; each leaves the ball's surface along its own angle at `speed` and
//! does `dmg`. The whole arc is one volley; a shot's index in it is its step along the fan. The projectiles
//! themselves are the roll's (`kinds::roll`: `spawn_key`, their flight and hits).
//!
//! A copied ability (IDENTITY CRISIS) runs this code for the copier's ball: nothing here assumes which side has the
//! kind, or that the ball has a roll. None of this code draws from the generator.

use bf_core::dmath;

use crate::abilities::param;
use crate::fight::Fight;
use crate::geom;
use crate::kinds::roll::{self, Shot};
use crate::roster::AbilityCfg;

/// One arc in progress: an entry of `self.arcs` (`trigger` 2855 to 2857).
#[derive(Clone, Debug, PartialEq)]
pub struct Arc {
    /// The fighter throwing it.
    pub owner: usize,
    /// When it started.
    pub t0: f64,
    /// When its last projectile leaves.
    pub t1: f64,
    /// The angle to the enemy when it started.
    pub aim: f64,
    /// The fan's width in radians.
    pub sweep: f64,
    /// How many projectiles (`ab['n']`).
    pub n: f64,
    /// How many have left.
    pub emitted: i64,
    /// Units a second each flies.
    pub speed: f64,
    /// The damage of each.
    pub dmg: f64,
    /// The arc's volley.
    pub vol: i64,
}

/// This ability's per-fight state (`Fight::arc`): the arcs in progress (`self.arcs`), in the order they started.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.arcs`.
    pub arcs: Vec<Arc>,
}

/// `trigger`, kind `arc`: `trigger` 2852 to 2858.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let aim = dmath::atan2(d[1], d[0]);
    let vol = f.roll.next_vol;
    f.arc.arcs.push(Arc {
        owner: k,
        t0: f.t,
        t1: f.t + param(ab, "beats") * f.rules.beat,
        aim,
        sweep: geom::radians(param(ab, "sweep")),
        n: param(ab, "n"),
        emitted: 0,
        speed: param(ab, "speed"),
        dmg: param(ab, "dmg"),
        vol,
    });
    f.roll.next_vol += 1;
}

/// Whether `self.arcs` holds anything: `effects_step` walks the arcs (3099 to 3113), and only then is [`step`]
/// called.
pub fn live(f: &Fight) -> bool {
    !f.arc.arcs.is_empty()
}

/// The arcs of `effects_step` (3099 to 3113): each arc emits its projectiles on time, after the slams; spent arcs are
/// dropped.
pub fn step(f: &mut Fight) {
    let r = f.rules.r;
    for a in 0..f.arc.arcs.len() {
        let ar = f.arc.arcs[a].clone();
        // max(1, n - 1)
        let steps = geom::py_max(1.0, ar.n - 1.0);
        let mut emitted = ar.emitted;
        while (emitted as f64) < ar.n {
            let j = emitted as f64;
            let te = ar.t0 + (ar.t1 - ar.t0) * j / steps;
            if f.t + 1e-9 < te {
                break;
            }
            let b_p = f.balls[ar.owner].p;
            let ang = ar.aim - ar.sweep / 2.0 + ar.sweep * j / steps;
            // pos = B.p + (R + 0.06) * np.array([math.cos(a), math.sin(a)])
            let pos = [
                b_p[0] + (r + 0.06) * dmath::cos(ang),
                b_p[1] + (r + 0.06) * dmath::sin(ang),
            ];
            let shot = Shot {
                pos,
                ang,
                speed: ar.speed,
                dmg: ar.dmg,
                vol: ar.vol,
                idx: emitted,
            };
            roll::spawn_key(f, ar.owner, shot);
            // 3110 to 3111, `curve`: an arc of curving projectiles, an older kind's (the roster reads none)
            emitted += 1;
        }
        f.arc.arcs[a].emitted = emitted;
    }
    f.arc.arcs.retain(|ar| (ar.emitted as f64) < ar.n);
}
