//! Ability kind `wave` (PICCOLO), and the ring both `wave` and `shockwave` make: `fight.py` 2841 to 2844 (`trigger`)
//! and 3057 to 3087 (`effects_step`).
//!
//! A ring starts at the fighter with the ball's radius and spreads at `speed` units a second to `radius`. When its
//! front passes the enemy (half a ball radius short of the enemy's centre) it hits once: `dmg`, a knock straight away
//! from the ring's centre, and its slow if it has one ([`crate::abilities::shockwave::slow`]). On its way it clears
//! the enemy's projectiles.

use crate::abilities::{self, param, shockwave};
use crate::fight::{Fight, Wave};
use crate::geom;
use crate::kinds::{self, tune};
use crate::roster::AbilityCfg;

/// `trigger`, kind `wave`: a ring of kind `wave` from where fighter `k` is.
pub fn fire(fight: &mut Fight, k: usize, ab: &AbilityCfg) {
    ring(fight, k, ab, "wave");
}

/// `trigger`, the branch the kinds `shockwave` and `wave` share: appends a ring centred where fighter `k` is now (a
/// copy: the ring does not follow the fighter), as wide as the ball, with the ability's `speed`, `radius`, `dmg`,
/// `knock` and `slow`. `kind` is what a hit by it is called.
pub fn ring(fight: &mut Fight, k: usize, ab: &AbilityCfg, kind: &str) {
    let wave = Wave {
        owner: k,
        c: fight.balls[k].p,
        r: fight.rules.r,
        speed: param(ab, "speed"),
        rmax: param(ab, "radius"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        kind: kind.to_string(),
        hit: false,
        name: ab.name.clone(),
        t0: fight.t,
        slow: ab.slow,
        stun: None,
        // fight.py 2845 to 2846: `if ab.get('detune'): self.waves[-1]['detune'] = ab['detune']` (ILL WIND; the roster
        // reads it only once the option is ported)
        detune: ab.detune.clone(),
        // an ability's ring never grows its owner's reach and always clears projectiles (`W.get('grow')`,
        // `W.get('clears', True)`); the walk's step ring and the stomps and honks of the kit set their own
        grow: false,
        clears: true,
    };
    fight.waves.push(wave);
}

/// `effects_step`, the rings: each spreads by `speed * dt` up to its final radius, hits the enemy its front passes
/// in this step, and clears the enemy projectiles in the band its front swept; a ring at its final radius is gone.
pub fn step(fight: &mut Fight, dt: f64) {
    let r = fight.rules.r;
    // Python walks `self.waves` and nothing in the loop adds or removes a ring, so an index walk is the same.
    for i in 0..fight.waves.len() {
        let w = &mut fight.waves[i];
        let r0 = w.r;
        w.r = geom::py_min(w.rmax, r0 + w.speed * dt);
        let (owner, c, front) = (w.owner, w.c, w.r);
        let o = 1 - owner;
        let rel = geom::sub(fight.balls[o].p, c);
        let dist = geom::hypot(rel);
        // `r0 < dist - R * 0.5 <= W['r'] + 1e-9`: the front reaches the enemy, half a ball radius short of its centre,
        // in this step
        let reach = dist - r * 0.5;
        if !w.hit && r0 < reach && reach <= front + 1e-9 {
            w.hit = true;
            let (dmg, knock, slow, stun, grow) = (w.dmg, w.knock, w.slow, w.stun, w.grow);
            let detune = w.detune.clone();
            let kind = w.kind.clone();
            // hx, hy = W['c'] + rel / max(dist, 1e-6) * (dist - R): the near side of the enemy's ball
            let m = geom::py_max(dist, 1e-6);
            let h = [c[0] + rel[0] / m * (dist - r), c[1] + rel[1] / m * (dist - r)];
            let landed = fight.damage(owner, o, dmg, &kind, h, 0);
            fight.knock(o, rel, knock);
            fight.socket("effects.wave.detune");
            if let Some(detune) = detune.filter(|_| landed) {
                // #37, fight.py 3069 to 3071: out of tune (ILL WIND); `_detune` is the tune's (OBOE's) code
                tune::wave_detune(fight, o, owner, &detune);
            }
            fight.socket("effects.wave.stun");
            if let Some(stun) = stun.filter(|s| landed && *s != 0.0) {
                // #37, fight.py 3072 to 3074: stunned (can't move or start an attack); SQUEAK's (CLARINET's) code
                abilities::stun::hit(fight, o, owner, stun);
            }
            if let Some(slow) = slow.filter(|_| landed) {
                // #34: the enemy it hits is slowed for a moment
                shockwave::slow(fight, o, owner, slow);
            }
            fight.socket("effects.wave.grow");
            if landed && grow {
                // fight.py 3078 to 3082: a ring with `grow` (the walk's step ring, CONTRABASS; and the older pulse
                // weapon's) grows its owner's reach: the walk's code
                kinds::walk::ring_grow(fight, owner);
            }
        }
        // fight.py 3083 to 3086, `if len(self.kp) and W.get('clears', True)`: the walk's step ring, the stampede's
        // stomps and the crowd's honks carry `clears = False`
        let clears = fight.waves[i].clears;
        if !fight.keys.is_empty() && clears {
            let mask: Vec<bool> = fight
                .keys
                .iter()
                .map(|key| {
                    let kr = geom::hypot(geom::sub(key.p, c));
                    key.own == o && kr > r0 - 0.15 && kr <= front + 0.05
                })
                .collect();
            fight.kill_keys(&mask, "cleared", owner as i64);
        }
    }
    fight.waves.retain(|w| w.r < w.rmax - 1e-9);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fight::Key;
    use crate::testing;

    fn ring(grow: bool, clears: bool) -> Wave {
        Wave {
            owner: 0,
            c: [0.0, 0.0],
            r: 0.6,
            speed: 6.0,
            rmax: 3.0,
            dmg: 1.0,
            knock: 0.5,
            kind: "step".into(),
            hit: false,
            name: "WALK".into(),
            t0: 0.0,
            slow: None,
            stun: None,
            detune: None,
            grow,
            clears,
        }
    }

    /// `effects_step` 3078 to 3086: a ring with `clears` false leaves the enemy's projectiles alone (the walk's step
    /// ring, the stampede's stomps, the crowd's honks), and a landed ring with `grow` calls its owner's growth (the
    /// walk's code: proven in `kinds::walk::kit_vectors`).
    #[test]
    fn a_rings_clears_and_grow_follow_python() {
        let bare = testing::bare_pair();
        for (grow, clears) in [(false, true), (false, false)] {
            let mut f = testing::fight(&testing::bare_setup(&bare, 1), false);
            f.trace_sockets = Some(std::cell::RefCell::new(Vec::new()));
            f.t = 5.0;
            f.balls[0].p = [0.0, 0.0];
            f.balls[1].p = [0.95, 0.0];
            f.keys.push(Key {
                p: [0.0, 0.64],
                v: [0.0, 1.0],
                own: 1,
                id: 7,
                vol: 0,
                idx: 0,
                dmg: 0.5,
            });
            f.waves.push(ring(grow, clears));
            let dt = f.rules.dt;
            step(&mut f, dt);
            let trace = f.trace_sockets.take().expect("on").into_inner();
            let kinds: Vec<&str> = f.events.iter().map(|e| e.kind.as_str()).collect();
            assert!(kinds.contains(&"hit"), "{grow} {clears}: {kinds:?}");
            assert_eq!(f.keys.is_empty(), clears, "{grow} {clears}");
            assert_eq!(kinds.contains(&"cleared"), clears);
            assert!(
                !trace.iter().any(|s| s.starts_with("stub:")),
                "{grow} {clears}: {trace:?}"
            );
        }
    }
}
