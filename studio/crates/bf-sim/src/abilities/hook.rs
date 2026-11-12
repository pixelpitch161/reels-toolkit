//! Ability kind `hook` (BASSOON's BOCAL HOOK): `fight.py` `trigger` 2915 to 2916; 3430 to 3476 (`hook_ready`,
//! `start_hook`, `hook_step`); the gate in `abilities_step` 3040 to 3042.
//!
//! The crook swings out at a close enemy, turning to it at `track` rad/s while it winds up over `open16` 16ths, and
//! hooks: an enemy within `reach` beyond contact and `half` degrees of the aim takes `dmg`, is spun round to face
//! away from the fighter (its heading turned, its speed kept) and nudged (`knock`), and a hopper gets a follow-up hop
//! straight behind it on the next 16th. It waits for its moment: it only starts when the enemy is within `ready`
//! beyond contact and the fighter is on the ground. Events `hook_open` (the time it hooks, the aim) and `hook` (hit
//! 0 / 1, the aim, the reach); recorded as `HOOK`.
//!
//! Python hangs `hook` on the ball; here it is [`State`], `Fight::hook`, indexed by fighter. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here assumes which side has the kind. None of
//! this code draws from the generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::Fight;
use crate::geom;
use crate::kinds::tiptoe;
use crate::record::Frames;
use crate::roster::{AbilityCfg, AbilityKind};

/// A hook under way (`B.hook`, the dict `start_hook` makes; `ab`'s numbers are copied in).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hook {
    /// `ab['track']`: rad/s the aim turns to the enemy while it winds up.
    pub track: f64,
    /// `ab['reach']`: the hook reaches this far beyond contact.
    pub reach: f64,
    /// `ab['half']`: and this many degrees either side of the aim.
    pub half: f64,
    /// `ab['dmg']`.
    pub dmg: f64,
    /// `ab['knock']`.
    pub knock: f64,
    /// The aim, an angle.
    pub aim: f64,
    /// When it hooks.
    pub t_hook: f64,
    /// 1 swinging out, 2 hooked (hit or not).
    pub state: i64,
    /// When it is put away (`None` while it swings out).
    pub t_end: Option<f64>,
}

/// This ability's per-fight state (`Fight::hook`): each fighter's `B.hook`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.hook`, by fighter.
    pub hook: [Option<Hook>; 2],
}

/// `trigger`, kind `hook`: `trigger` 2915 to 2916, `start_hook(k, ab)` (3442 to 3446): the crook swings out, aimed at
/// the enemy as it stands. Event `hook_open`.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let aim = dmath::atan2(d[1], d[0]);
    let t_hook = f.t + param(ab, "open16") * f.rules.beat / 4.0;
    f.hook.hook[k] = Some(Hook {
        track: param(ab, "track"),
        reach: param(ab, "reach"),
        half: param(ab, "half"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        aim,
        t_hook,
        state: 1,
        t_end: None,
    });
    f.ev("hook_open", k as i64, vec![Value::Float(t_hook), Value::Float(aim)]);
}

/// `hook_ready(k, ab)` (3437 to 3440): the gate in `abilities_step` 3040 to 3042: the fighter is on the ground and
/// the enemy within `ready` beyond contact.
pub fn ready(f: &Fight, k: usize, ab: &AbilityCfg) -> bool {
    let (b, o) = (&f.balls[k], &f.balls[1 - k]);
    !b.air && geom::hypot(geom::sub(o.p, b.p)) - 2.0 * f.rules.r <= param(ab, "ready")
}

/// `hook_step(k, dt)` (3448 to 3476), from `run()` 6163 to 6164: while it swings out the aim turns to the enemy, at
/// most `track * dt` a step; at its time it hooks (event `hook`) and stays out 0.3 s more.
pub fn hook_step(f: &mut Fight, k: usize, dt: f64) {
    let Some(mut hk) = f.hook.hook[k] else {
        return;
    };
    let r = f.rules.r;
    let o_p = f.balls[1 - k].p;
    let rel = geom::sub(o_p, f.balls[k].p);
    if hk.state == 1 {
        let want = dmath::atan2(rel[1], rel[0]);
        let step = hk.track * dt;
        // _wrap(Hk['aim'] + max(-step, min(step, _wrap(want - Hk['aim']))))
        hk.aim = geom::wrap(hk.aim + py::py_max(-step, py::py_min(step, geom::wrap(want - hk.aim))));
        if f.t + 1e-9 >= hk.t_hook {
            let dist = geom::hypot(rel);
            let off = geom::wrap(dmath::atan2(rel[1], rel[0]) - hk.aim).abs();
            // math.degrees(off): off * (180.0 / pi), the constant formed first, as CPython does
            let hit = dist - 2.0 * r <= hk.reach && off * (180.0 / dmath::PI) <= hk.half;
            if hit {
                let n = if dist > 1e-6 {
                    geom::div(rel, dist)
                } else {
                    [dmath::cos(hk.aim), dmath::sin(hk.aim)]
                };
                // hx, hy = (O.p - n * self.R.R).tolist()
                let at = geom::sub(o_p, geom::scale(n, r));
                if f.damage(k, 1 - k, hk.dmg, "hook", at, 0) {
                    let o = &mut f.balls[1 - k];
                    let sp = geom::hypot(o.v);
                    // spun round: it now faces away from the fighter
                    o.v = geom::scale(n, py::py_max(sp, 1e-6));
                    o.boost = py::py_max(o.boost, hk.knock);
                    if tiptoe::is(f, k) {
                        // the follow-up hop: straight behind it
                        f.tiptoe.hop_bonus[k] = true;
                    }
                }
            }
            let data = vec![Value::Int(i64::from(hit)), Value::Float(hk.aim), Value::Float(hk.reach)];
            f.ev("hook", k as i64, data);
            hk.state = 2;
            hk.t_end = Some(f.t + 0.3);
        }
        f.hook.hook[k] = Some(hk);
    } else if hk.t_end.is_some_and(|t_end| f.t >= t_end) {
        f.hook.hook[k] = None;
    }
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5851 to 5853: `HOOK`, when a fighter
/// has a hook among its abilities.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| crate::abilities::has(f, k, &AbilityKind::Hook)) {
        &["HOOK"]
    } else {
        &[]
    }
}

/// This kind's rows of one frame: `snap`, `HOOK` (5602 to 5608): per ball, the hook's state, aim and time, or three
/// NaN when it has none out.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut row = [f64::NAN; 6];
    for (j, hook) in f.hook.hook.iter().enumerate() {
        if let Some(hk) = hook {
            row[3 * j] = hk.state as f64;
            row[3 * j + 1] = hk.aim;
            row[3 * j + 2] = hk.t_hook;
        }
    }
    out.push_f64("HOOK", &[2, 3], &row);
}
