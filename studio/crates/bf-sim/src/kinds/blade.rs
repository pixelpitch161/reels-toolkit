//! Weapon kind `blade` in thrust mode, the TROMBONE's slide lance: `fight.py`'s `start_thrust` and `thrust_step`
//! (lines 701 to 773) and the thrust branch of `blade_step` (775 to 779).
//!
//! Idle, the closed lance (`rest` long beyond the ball) turns toward the enemy at `aim_rate` at most. On its grid
//! (every `every` beats; haste divides it) the lance locks on where the enemy is and shoots out to its reach in
//! `t_out` seconds, holds for `t_hold` and pulls back over `t_back`; haste runs the whole thrust faster. While it
//! goes out or is held the lance is live: it kills enemy projectiles it touches (`parry`), knocks risen enemy cobras
//! flat (the rule `swat_cobras`), swats falling enemy shells (the rule `swat_shells`, the mortar's code) and hits
//! the enemy once (`dmg`, `knock`; a landed hit grows the reach by `grow` up to `max_len` and adds a fret). A thrust
//! that meets a shield is blocked and pulls straight back.
//!
//! The lance's state is on the ball, where `Ball.__init__` puts it: `th` (its angle), `ext` (its length beyond the
//! ball now), `thr` (when the thrust in progress started), `thr_hit`, `thr_k` (the thrust's time scale), with the
//! reach in `blade_len` and the growth count in `frets`. None of this code draws from the generator.
//!
//! A blade without `mode = 'thrust'` spins (the rest of `blade_step`, 780 to 825: VIOLIN's bow, milestone 4):
//! [`spin_step`], a stub until VIOLIN's port, which the roster reads as unsupported until then
//! (`roster::blade::SPIN_PORTED`).

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::Fight;
use crate::geom;
use crate::kinds::{mortar, striker};
use crate::roster::{BladeCfg, BladeMode, WeaponCfg};

/// `B.w['kind'] == 'blade'`.
pub fn is_blade(fight: &Fight, k: usize) -> bool {
    matches!(fight.balls[k].cfg.weapon, WeaponCfg::Blade(_))
}

/// The blade's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k` has no blade.
fn weapon<'a>(fight: &Fight<'a>, k: usize) -> &'a BladeCfg {
    let cfg = fight.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Blade(w) => w,
        other => panic!("fighter {k} ({}) has no blade: {other:?}", cfg.name),
    }
}

/// `int(round(4 * w['every']))`: the lance's own thrust grid in 16ths (4 for `every = 1` beat). The fight loop
/// divides it by the haste factor (`fight.py` 5998 to 5999).
///
/// # Panics
/// When fighter `k` has no blade.
pub fn main16(fight: &Fight, k: usize) -> i64 {
    py::round_half_even(4.0 * weapon(fight, k).every) as i64
}

/// `start_thrust(k)` (#34): the lance locks on where the enemy is now and shoots out; haste speeds the whole thrust
/// up (`thr_k` is 1 over the haste factor). Event `thrust`: the reach of this thrust.
pub fn start_thrust(fight: &mut Fight, k: usize) {
    let rel = geom::sub(fight.balls[1 - k].p, fight.balls[k].p);
    let (t, mult) = (fight.t, fight.ab_mult(k));
    let b = &mut fight.balls[k];
    b.th = py::fmod_py(dmath::atan2(rel[1], rel[0]), 2.0 * dmath::PI);
    b.thr = Some(t);
    b.thr_hit = false;
    b.thr_k = 1.0 / mult;
    let reach = b.blade_len;
    fight.ev("thrust", k as i64, vec![Value::Float(reach)]);
}

/// `thrust_step(k, dt)` (#34). Idle, the closed lance tracks the enemy (turn-rate limited). A thrust shoots the
/// lance out to the reach (`t_out`), holds it (`t_hold`) and pulls it back (`t_back`); it hits once per thrust while
/// going out or held: the enemy ball against the lance segment, tested every step (the tip moves less than a ball
/// per step).
pub fn thrust_step(fight: &mut Fight, k: usize, dt: f64) {
    let w = weapon(fight, k);
    let r = fight.rules.r;
    let rest = w.rest_idle();
    let (t, o_p) = (fight.t, fight.balls[1 - k].p);
    let b = &mut fight.balls[k];
    let Some(thr) = b.thr else {
        let rel = geom::sub(o_p, b.p);
        let dth = geom::wrap(dmath::atan2(rel[1], rel[0]) - b.th);
        let lim = w.aim_rate * dt;
        // B.th = (B.th + max(-lim, min(lim, dth))) % (2 * math.pi)
        b.th = py::fmod_py(b.th + geom::py_max(-lim, geom::py_min(lim, dth)), 2.0 * dmath::PI);
        b.ext = rest;
        return;
    };
    let u = (t - thr) / b.thr_k;
    let (t_out, t_hold, t_back) = (w.t_out, w.t_hold, w.t_back);
    // `live`: the lance is going out or held
    let live = if u < t_out {
        let e = u / t_out;
        // rest + (B.blade_len - rest) * (1 - (1 - e) ** 2)
        // a Python float squared with `**` is the C runtime's pow(x, 2.0), one step from the product now and then
        b.ext = rest + (b.blade_len - rest) * (1.0 - dmath::pow(1.0 - e, 2.0));
        true
    } else if u < t_out + t_hold {
        b.ext = b.blade_len;
        true
    } else if u < t_out + t_hold + t_back {
        let e = (u - t_out - t_hold) / t_back;
        b.ext = b.blade_len + (rest - b.blade_len) * e * e * (3.0 - 2.0 * e);
        false
    } else {
        b.thr = None;
        b.ext = rest;
        return;
    };
    // the lance: from `a`, 0.9 R out of the ball's centre, `l` along `d`
    let d = [dmath::cos(b.th), dmath::sin(b.th)];
    // a = B.p + d * R * 0.9
    let a = [b.p[0] + d[0] * r * 0.9, b.p[1] + d[1] * r * 0.9];
    let l = r * 0.1 + b.ext;
    if live && w.parry && !fight.keys.is_empty() {
        // 746 to 752: the lance swats enemy projectiles it passes through
        let reach = fight.rules.key_r + 0.04;
        let enemy = |own: usize| own != k;
        if fight.keys.iter().any(|key| enemy(key.own)) {
            let rows = fight.keys.len();
            let hit: Vec<bool> = fight
                .keys
                .iter()
                .map(|key| {
                    let rel = geom::sub(key.p, a);
                    // s_ = np.clip(rel @ d, 0, L_); dd = np.hypot(*(rel - s_[:, None] * d[None]).T): `rel` is the
                    // array of every key, so the product is a matrix times a vector
                    let s = geom::np_clip(geom::dot_mv(rel, d, rows), 0.0, l);
                    let dd = geom::hypot([rel[0] - s * d[0], rel[1] - s * d[1]]);
                    enemy(key.own) && dd < reach
                })
                .collect();
            fight.kill_keys(&hit, "parry", k as i64);
        }
    }
    if live && fight.rules.swat_cobras == Some(true) {
        // 753 to 754, opt-in rule (#41): the lance swats enemy cobras too
        striker::swat_cobras(fight, k, a, d, l);
    }
    fight.socket("lance.swat_shells");
    if live && fight.rules.swat_shells == Some(true) && !fight.shells.is_empty() {
        // 755 to 756, opt-in rule (#48): the lance swats enemy mortar shells too (the mortar's code)
        mortar::swat_shells(fight, k, a, d, l);
    }
    if live && !fight.balls[k].thr_hit {
        let rel = geom::sub(o_p, a);
        // s_ = min(max(float(rel @ d), 0.0), L_)
        let s = geom::py_min(geom::py_max(geom::dot(rel, d), 0.0), l);
        let c = [a[0] + d[0] * s, a[1] + d[1] * s];
        let dist = geom::hypot(geom::sub(o_p, c));
        if dist < r {
            fight.balls[k].thr_hit = true;
            // hx, hy = O.p + (c - O.p) / max(dist, 1e-6) * R
            let m = geom::py_max(dist, 1e-6);
            let h = [o_p[0] + (c[0] - o_p[0]) / m * r, o_p[1] + (c[1] - o_p[1]) / m * r];
            if fight.balls[1 - k].shield > 0.0 {
                let data = vec![
                    Value::Int(k as i64),
                    Value::Float(h[0]),
                    Value::Float(h[1]),
                    Value::Str("blade".to_string()),
                ];
                fight.ev("block", (1 - k) as i64, data);
                // blocked: the lance pulls straight back
                let b = &mut fight.balls[k];
                b.thr = Some(t - (t_out + t_hold) * b.thr_k);
            } else {
                let frets = fight.balls[k].frets;
                let landed = fight.damage(k, 1 - k, w.dmg, "blade", h, frets);
                fight.knock(1 - k, d, w.knock);
                let b = &mut fight.balls[k];
                if landed && b.blade_len + w.grow <= w.max_len + 1e-9 {
                    b.blade_len += w.grow;
                    b.frets += 1;
                }
            }
        }
    }
}

/// `blade_step(k, dt)`: the blade's step in the fight loop. A thrust blade's is [`thrust_step`] (`fight.py` 778 to
/// 779); the rest of `blade_step` is the spinning blade's ([`spin_step`]).
///
/// # Panics
/// When fighter `k` has no blade.
pub fn step(fight: &mut Fight, k: usize, dt: f64) {
    match weapon(fight, k).mode {
        BladeMode::Thrust => thrust_step(fight, k, dt),
        BladeMode::Spin => spin_step(fight, k, dt),
    }
}

/// The spinning branch of `blade_step(k, dt)` (780 to 825, with `spin_grow` and `spin_max`): VIOLIN's bow. A stub
/// until VIOLIN's port (milestone 4): no fighter has a spinning blade before `roster::blade::SPIN_PORTED`.
pub fn spin_step(fight: &mut Fight, _k: usize, _dt: f64) {
    fight.socket("stub:kinds::blade::spin_step");
}

#[cfg(test)]
mod tests;
