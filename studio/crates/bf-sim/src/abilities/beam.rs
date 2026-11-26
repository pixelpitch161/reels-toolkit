//! Ability kind `beam` (TRUMPET's HIGH C): `fight.py` `trigger` 2870 to 2871; `ab_beam_charge` 2990 to 2999,
//! `ab_beam_fire` 3001 to 3010, `abilities_step` 3013 to 3016, the ability beams of `beams_step` 857 to 897.
//!
//! The fighter charges for `charge_beats` beats with its aim locked on the spot the enemy is predicted to reach
//! (`lead` x the charge ahead), then fires one giant blast: a capsule `len` long and `width` wide from just outside
//! the ball, live for `hold` seconds. It hits the enemy once (`dmg`, `knock`), pierces enemy projectiles in its path
//! (`cleared`) and grows no weapon.
//!
//! The beam *weapon* (`beam_beat`, `beam_fire`, the bell tracking at the top of `beams_step`) is an older kind and
//! is not ported: every beam here is an ability's. A copied ability (IDENTITY CRISIS) runs this code for the
//! copier's ball, so nothing here assumes which side has the kind. None of this code draws from the generator.

use bf_core::dmath;
use bf_core::event::Value;

use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::roster::AbilityCfg;

/// A charge in progress (`B.ab_charge`): the locked aim, when it fires, and the ability's numbers.
#[derive(Clone, Debug, PartialEq)]
pub struct Charge {
    /// The locked aim, radians (`aim`).
    pub aim: f64,
    /// When the blast fires (`t_fire`).
    pub t_fire: f64,
    /// `ab['len']`.
    pub len: f64,
    /// `ab['width']`.
    pub width: f64,
    /// `ab.get('hold', 0.3)`: seconds the blast is live.
    pub hold: f64,
    /// `ab['dmg']`.
    pub dmg: f64,
    /// `ab.get('knock', 1.5)`.
    pub knock: f64,
}

/// A live blast (one of `self.beams`; an ability's has `grow=False`).
#[derive(Clone, Debug, PartialEq)]
pub struct Beam {
    /// The fighter that fired it.
    pub owner: usize,
    /// Where it starts (`o`).
    pub o: V2,
    /// Its direction, a unit vector (`d`).
    pub d: V2,
    /// Its length.
    pub len: f64,
    /// Its width (`w`).
    pub w: f64,
    /// When it ends (`t1`).
    pub t1: f64,
    /// Whether it has hit the enemy.
    pub hit: bool,
    /// Its damage.
    pub dmg: f64,
    /// Its knockback.
    pub knock: f64,
}

/// This ability's per-fight state (`Fight::beam`): each ball's charge and the fight's live beams.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.ab_charge`, per fighter.
    pub charge: [Option<Charge>; 2],
    /// `self.beams`.
    pub beams: Vec<Beam>,
}

/// `trigger`, kind `beam` (2870 to 2871): `ab_beam_charge(k, ab)` (2990 to 2999). Event `charge`: the aim, the
/// seconds of charge, the blast's length and width.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let (b, o) = (&f.balls[k], &f.balls[1 - k]);
    // tc = ab.get('charge_beats', 1) * self.R.beat
    let tc = ab.get("charge_beats").unwrap_or(1.0) * f.rules.beat;
    let lead = ab.get("lead").unwrap_or(0.7);
    // d = (O.p + O.v * tc * ab.get('lead', 0.7)) - (B.p + B.v * tc)
    let d = [
        (o.p[0] + o.v[0] * tc * lead) - (b.p[0] + b.v[0] * tc),
        (o.p[1] + o.v[1] * tc * lead) - (b.p[1] + b.v[1] * tc),
    ];
    let aim = dmath::atan2(d[1], d[0]);
    let (len, width) = (crate::abilities::param(ab, "len"), crate::abilities::param(ab, "width"));
    f.beam.charge[k] = Some(Charge {
        aim,
        t_fire: f.t + tc,
        len,
        width,
        hold: ab.get("hold").unwrap_or(0.3),
        dmg: crate::abilities::param(ab, "dmg"),
        knock: ab.get("knock").unwrap_or(1.5),
    });
    let data = vec![
        Value::Float(aim),
        Value::Float(tc),
        Value::Float(len),
        Value::Float(width),
    ];
    f.ev("charge", k as i64, data);
}

/// Whether `self.beams` holds anything: half of the test before `beams_step` (`run()` 6139; the other half, a `beam`
/// weapon, is an older kind).
pub fn live(f: &Fight) -> bool {
    !f.beam.beams.is_empty()
}

/// `abilities_step` 3013 to 3016: a charged ability beam fires when its beat comes (`ab_beam_fire(k)`, 3001 to 3010),
/// before any fighter's levels. Event `beam`: where the blast starts, its aim, length and width.
pub fn check_charge(f: &mut Fight, k: usize) {
    // if ch is not None and self.t + 1e-9 >= ch['t_fire']
    let Some(ch) = f.beam.charge[k].take_if(|ch| f.t + 1e-9 >= ch.t_fire) else {
        return;
    };
    let a = ch.aim;
    let d = [dmath::cos(a), dmath::sin(a)];
    // o = B.p + d * (self.R.R + 0.04)
    let reach = f.rules.r + 0.04;
    let p = f.balls[k].p;
    let o = [p[0] + d[0] * reach, p[1] + d[1] * reach];
    f.beam.beams.push(Beam {
        owner: k,
        o,
        d,
        len: ch.len,
        w: ch.width,
        t1: f.t + ch.hold,
        hit: false,
        dmg: ch.dmg,
        knock: ch.knock,
    });
    let data = vec![
        Value::Float(o[0]),
        Value::Float(o[1]),
        Value::Float(a),
        Value::Float(ch.len),
        Value::Float(ch.width),
    ];
    f.ev("beam", k as i64, data);
}

/// `beams_step(dt)` (857 to 897), from `run()` 6139 to 6140: each live beam hits the enemy once (a capsule test) and
/// pierces the enemy's projectiles in its path; the ended beams leave. (861 to 871, a beam weapon's bell, and the
/// growth of a landed weapon beam, 886 to 889, are the older `beam` weapon: an ability's beam has `grow=False`.)
pub fn beams_step(f: &mut Fight, _dt: f64) {
    let r = f.rules.r;
    // Python walks `self.beams`; nothing in the loop adds or removes one, so an index walk is the same.
    for i in 0..f.beam.beams.len() {
        let bm = f.beam.beams[i].clone();
        let k = bm.owner;
        let o = 1 - k;
        let o_p = f.balls[o].p;
        let rel = geom::sub(o_p, bm.o);
        // s_ = min(Bm['len'], max(0.0, float(rel @ Bm['d'])))
        let s = geom::py_min(bm.len, geom::py_max(0.0, geom::dot(rel, bm.d)));
        let off = [rel[0] - bm.d[0] * s, rel[1] - bm.d[1] * s];
        let dist = geom::hypot(off);
        if !bm.hit && dist < r + bm.w / 2.0 {
            f.beam.beams[i].hit = true;
            // n = off / dist if dist > 1e-6 else -Bm['d']
            let n = if dist > 1e-6 {
                geom::div(off, dist)
            } else {
                [-bm.d[0], -bm.d[1]]
            };
            // hx, hy = (O.p - n * R)
            let h = [o_p[0] - n[0] * r, o_p[1] - n[1] * r];
            f.damage(k, o, bm.dmg, "beam", h, 0);
            // self.knock(o, Bm['d'] + 0.3 * n, Bm['knock'])
            f.knock(o, [bm.d[0] + 0.3 * n[0], bm.d[1] + 0.3 * n[1]], bm.knock);
        }
        // if len(self.kp) and _clears(self.balls[k].w): no ported weapon kind has `clears`, so it is the default,
        // True (`roster` refuses the key on a ported weapon)
        if !f.keys.is_empty() {
            let rk = f.rules.key_r;
            let rows = f.keys.len();
            let mask: Vec<bool> = f
                .keys
                .iter()
                .map(|key| {
                    let relk = geom::sub(key.p, bm.o);
                    // sk = np.clip(relk @ Bm['d'], 0.0, Bm['len']): `relk` is the array of every key, so the
                    // product is a matrix times a vector
                    let sk = geom::np_clip(geom::dot_mv(relk, bm.d, rows), 0.0, bm.len);
                    let dk = geom::hypot([relk[0] - sk * bm.d[0], relk[1] - sk * bm.d[1]]);
                    key.own == o && dk < bm.w / 2.0 + rk
                })
                .collect();
            f.kill_keys(&mask, "cleared", k as i64);
        }
    }
    // self.beams = [Bm for Bm in self.beams if self.t < Bm['t1'] - 1e-9]
    let t = f.t;
    f.beam.beams.retain(|bm| t < bm.t1 - 1e-9);
}
