//! Ability kind `slam` (PEDAL TONE): `fight.py` 2859 to 2865 (`trigger`) and 3088 to 3098 (`effects_step`).
//!
//! The slam is called on the spot the enemy is heading for (60 % of the way it would fly in `tele` beats, held
//! inside the ropes) and lands `tele` beats later. An enemy within `radius` plus 0.6 of a ball radius of that spot
//! then takes `dmg` and is knocked away from it.

use bf_core::event::Value;

use crate::abilities::param;
use crate::fight::{Fight, Slam};
use crate::geom;
use crate::roster::AbilityCfg;

/// `trigger`, kind `slam`: `lead = tele * beat`; the target is `O.p + O.v * lead * 0.6`, each coordinate clipped to
/// the ring less a ball radius; it lands at `t + lead`.
pub fn fire(fight: &mut Fight, k: usize, ab: &AbilityCfg) {
    let o = &fight.balls[1 - k];
    let lead = param(ab, "tele") * fight.rules.beat;
    // tgt = O.p + O.v * lead * 0.6
    let tgt = [o.p[0] + o.v[0] * lead * 0.6, o.p[1] + o.v[1] * lead * 0.6];
    // A = self.R.A - self.R.R; tgt = np.clip(tgt, -A, A)
    let a = fight.rules.a - fight.rules.r;
    let slam = Slam {
        owner: k,
        c: [geom::np_clip(tgt[0], -a, a), geom::np_clip(tgt[1], -a, a)],
        t0: fight.t,
        t_slam: fight.t + lead,
        radius: param(ab, "radius"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        done: false,
        name: ab.name.clone(),
        // `S.get('how', 'slam')`: only 1812 OVERTURE's cannons (`roll_charge::start_cannons`) say otherwise
        how: "slam".to_string(),
    };
    fight.slams.push(slam);
}

/// `effects_step`, the slams: each lands once, in the first step at or after its time. Event `slam`: where, and 1 if
/// the enemy is inside, else 0. An enemy inside takes the damage (a hit of kind `slam` at its own centre) and is
/// knocked away from the slam's centre (along its own heading when it stands on the centre). A slam that has landed
/// stays in the list: the record shows it for 0.6 s more.
pub fn step(fight: &mut Fight) {
    let r = fight.rules.r;
    // Python walks `self.slams` and nothing in the loop adds or removes one, so an index walk is the same.
    for i in 0..fight.slams.len() {
        let s = &mut fight.slams[i];
        // `if not S['done'] and self.t >= S['t_slam']`
        if s.done || fight.t < s.t_slam {
            continue;
        }
        s.done = true;
        let (owner, c, radius, dmg, knock) = (s.owner, s.c, s.radius, s.dmg, s.knock);
        let how = s.how.clone();
        let o = 1 - owner;
        let (o_p, o_v) = (fight.balls[o].p, fight.balls[o].v);
        let rel = geom::sub(o_p, c);
        let dist = geom::hypot(rel);
        let inside = dist < radius + r * 0.6;
        let data = vec![Value::Float(c[0]), Value::Float(c[1]), Value::Int(i64::from(inside))];
        fight.ev("slam", owner as i64, data);
        if inside {
            // fight.py 3097: `S.get('how', 'slam')`, `cannon` for 1812 OVERTURE's (#56)
            fight.socket("effects.slam.how");
            fight.damage(owner, o, dmg, &how, o_p, 0);
            fight.knock(o, if dist > 1e-3 { rel } else { o_v }, knock);
        }
    }
}
