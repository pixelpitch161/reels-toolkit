//! Weapon kind `roll_charge`, BASS DRUM's STEAMROLLER: `fight.py` 5284 to 5431 (`rollc_setup`, `rollc_blocked`,
//! `_roll_line`, `_start_roll`, `_end_roll`, `rollc_tick`, `rollc_step`), and the abilities `heartbeat`, `drum_roll`
//! and `cannons` (`trigger` 2983 to 2988; 5432 to 5494).
//!
//! The only roller. Every `every` beats on the 16th grid (`off16` later) the drum tips onto its side for `tip16`
//! 16ths: it holds still and the line is set, aimed at the enemy's spot `lead` x the roll ahead, turned by a seeded
//! jitter. Then it rolls along the line like a wheel at `speed` x the shared speed, up to `span`, stopping a radius
//! short of the ropes. From the tip on it is ROLLING: a walker (no knockback, no giving way in a clash). The first
//! contact of a roll is a BOOM; a landed one grows the roll's speed and the drum's size (`blade_len`). After a roll it
//! glides on along its line. A held, stunned, grabbed or smashed drum starts no roll, and a roll in progress ends
//! (`roll_off`): that is also how a TUBA's gulp (the pair rule `swallow_rollers`) ends a roll.
//!
//! HEARTBEAT sends two shockwaves `gap16` 16ths apart; DRUM ROLL rolls nonstop in zig-zag legs for `beats` beats;
//! 1812 OVERTURE calls `n` cannon slams, one every `gap_beats` beats, at the enemy's spot `tele` beats ahead.
//!
//! Python hangs the state on the ball: `rolling`, `walking`, `walker` and `blade_len` are on [`crate::ball::Ball`];
//! `rl`, `roll_v`, `roll_ang`, `droll`, `thumps` and `cannons` are in [`State`] (`Fight::roll_charge`), by fighter.
//!
//! Draws from the fight's generator, in Python's order: one `uniform(-1, 1)` per roll line tried (`_roll_line` 5320:
//! each weapon roll that passes the grid, and each side a DRUM ROLL leg tries), one `uniform()` per DRUM ROLL (its
//! first side, 5471).

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, Slam, Wave, stunned};
use crate::geom::{self, V2, py_max, py_min};
use crate::record::Frames;
use crate::roster::{AbilityCfg, RollChargeCfg, WeaponCfg};

/// A roll's phase (`L['phase']`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// `'tip'`: keeling over, held still at the start of the line.
    Tip,
    /// `'roll'`: rolling along the line.
    Roll,
}

/// The current roll (`B.rl`, `_start_roll` 5334 to 5335).
#[derive(Clone, Debug, PartialEq)]
pub struct Roll {
    /// Tipping or rolling.
    pub phase: Phase,
    /// Where the line starts (the ball when the roll started).
    pub p0: V2,
    /// Where it ends.
    pub p1: V2,
    /// The line's direction.
    pub u: V2,
    /// When the tip started.
    pub t_tip: f64,
    /// When the rolling starts.
    pub t0: f64,
    /// When it reaches the line's end.
    pub t1: f64,
    /// Whether it has hit (a weapon roll BOOMs once).
    pub hit: bool,
    /// 0 for the weapon's roll, 1 for a DRUM ROLL leg.
    pub kind: i64,
    /// The time of its last contact (-9.0 before the first).
    pub last: f64,
}

/// DRUM ROLL in progress (`B.droll`, `start_drum_roll` 5471).
#[derive(Clone, Debug, PartialEq)]
pub struct DrumRoll {
    /// When it ends.
    pub t1: f64,
    /// The side the next leg turns to first: 1.0 or -1.0.
    pub side: f64,
    /// The ability (`Dr['ab']`).
    pub ab: AbilityCfg,
}

/// 1812 OVERTURE in progress (`B.cannons`, `start_cannons` 5478).
#[derive(Clone, Debug, PartialEq)]
pub struct Cannons {
    /// The ability (`C['ab']`).
    pub ab: AbilityCfg,
    /// The cannons fired so far.
    pub i: i64,
    /// When the next one fires.
    pub t_next: f64,
}

/// Weapon kind `roll_charge`'s per-fight state (`Fight::roll_charge`): what `fight.py` hangs on the ball for this kind
/// besides `rolling`, `walking`, `walker` and `blade_len`, by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.rl`: the current roll.
    pub rl: [Option<Roll>; 2],
    /// `B.roll_v`: the roll's speed, a multiple of the shared speed.
    pub roll_v: [f64; 2],
    /// `B.roll_ang`: how far the wheel has turned (rad), for the picture.
    pub roll_ang: [f64; 2],
    /// `B.droll`: DRUM ROLL in progress.
    pub droll: [Option<DrumRoll>; 2],
    /// `B.thumps`: HEARTBEAT's second thump still to come, `(t, ab)`.
    pub thumps: [Vec<(f64, AbilityCfg)>; 2],
    /// `B.cannons`: 1812 OVERTURE in progress.
    pub cannons: [Option<Cannons>; 2],
}

/// `B.w['kind'] == 'roll_charge'`: fighter `k`'s weapon is this kind. It is also Python's
/// `getattr(B, 'roller', False)` (`snap` 5716): only `rollc_setup` gives a ball `roller`.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::RollCharge(_))
}

/// The roller's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k` is not a roller.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a RollChargeCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::RollCharge(w) => w,
        other => panic!("fighter {k} ({}) is not a roller: {other:?}", cfg.name),
    }
}

/// `ab.get(name, default)`.
fn get_or(ab: &AbilityCfg, name: &str, default: f64) -> f64 {
    ab.get(name).unwrap_or(default)
}

/// `rollc_setup(k)` (5294 to 5305), from `run()` 5906 to 5908: not rolling, no roll, the roll's speed `speed`, the
/// drum's size `size0`, the wheel at 0, no ability in progress.
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    f.balls[k].rolling = false;
    f.balls[k].blade_len = w.size0;
    let st = &mut f.roll_charge;
    st.rl[k] = None;
    st.roll_v[k] = w.speed;
    st.roll_ang[k] = 0.0;
    st.droll[k] = None;
    st.thumps[k] = Vec::new();
    st.cannons[k] = None;
}

/// `rollc_blocked(k)` (5306 to 5309): stunned (or sunk in shadow), held, grabbed or in pieces.
pub fn blocked(f: &Fight, k: usize) -> bool {
    let b = &f.balls[k];
    stunned(f, k) || b.held || b.grabbed || b.smashed > 0.0
}

/// `B.rolling = B.walking = B.walker = on`.
fn set_rolling(f: &mut Fight, k: usize, on: bool) {
    let b = &mut f.balls[k];
    b.rolling = on;
    b.walking = on;
    b.walker = on;
}

/// `_roll_line(k, t_ahead, jitter_deg, extra_deg, span)` (5311 to 5327): the roll's direction and length, at the
/// enemy's spot `lead` x `t_ahead` ahead (inside the ropes), turned by (a seeded jitter + `extra_deg`) degrees,
/// `span` long at most, stopping a radius short of the ropes. Draws one `uniform(-1, 1)`.
fn roll_line(f: &mut Fight, k: usize, t_ahead: f64, jitter_deg: f64, extra_deg: f64, span: f64) -> (V2, f64) {
    let lead = weapon(f, k).lead;
    let lim = f.rules.a - f.rules.r;
    let (bp, bv) = (f.balls[k].p, f.balls[k].v);
    let (op, ov) = (f.balls[1 - k].p, f.balls[1 - k].v);
    // tgt = np.clip(O.p + O.v * t_ahead * B.w.get('lead', 0.0), -lim, lim)
    let tgt = [
        geom::np_clip(op[0] + ov[0] * t_ahead * lead, -lim, lim),
        geom::np_clip(op[1] + ov[1] * t_ahead * lead, -lim, lim),
    ];
    let d = geom::sub(tgt, bp);
    let dist = geom::hypot(d);
    let u = if dist > 1e-6 {
        geom::div(d, dist)
    } else {
        geom::div(bv, geom::hypot(bv) + 1e-9)
    };
    let j = geom::radians(f.rng.uniform(-1.0, 1.0) * jitter_deg + extra_deg);
    let (cj, sj) = (dmath::cos(j), dmath::sin(j));
    let u = [u[0] * cj - u[1] * sj, u[0] * sj + u[1] * cj];
    let mut ln = span;
    for ax in 0..2 {
        if u[ax].abs() > 1e-9 {
            let room = ((if u[ax] > 0.0 { lim } else { -lim }) - bp[ax]) / u[ax];
            ln = py_min(ln, py_max(0.0, room));
        }
    }
    (u, ln)
}

/// `_start_roll(k, kind, t_tip, u, ln, speed)` (5329 to 5339): a roll along `u`, `ln` long, at `speed` x the shared
/// speed, after a tip of `t_tip` s (none for a DRUM ROLL leg). The ball is rolling from now on and faces the line.
/// Event `roll`: from, to, the tip's start, the roll's start and end, the kind (0 the weapon, 1 a DRUM ROLL leg).
fn start_roll(f: &mut Fight, k: usize, kind: i64, t_tip: f64, u: V2, ln: f64, speed: f64) {
    let v = speed * f.rules.v;
    let t = f.t;
    let t0 = t + t_tip;
    let p0 = f.balls[k].p;
    let p1 = [p0[0] + u[0] * ln, p0[1] + u[1] * ln];
    let t1 = t0 + ln / py_max(v, 1e-9);
    f.roll_charge.rl[k] = Some(Roll {
        phase: if t_tip > 0.0 { Phase::Tip } else { Phase::Roll },
        p0,
        p1,
        u,
        t_tip: t,
        t0,
        t1,
        hit: false,
        kind,
        last: -9.0,
    });
    set_rolling(f, k, true);
    let speed_v = f.rules.v;
    let b = &mut f.balls[k];
    b.v = geom::scale(u, speed_v);
    b.th = py::fmod_py(dmath::atan2(u[1], u[0]), 2.0 * dmath::PI);
    let data = vec![
        Value::Float(p0[0]),
        Value::Float(p0[1]),
        Value::Float(p1[0]),
        Value::Float(p1[1]),
        Value::Float(t),
        Value::Float(t0),
        Value::Float(t1),
        Value::Int(kind),
    ];
    f.ev("roll", k as i64, data);
}

/// `_end_roll(k, off=None)` (5341 to 5358): the roll ends, called off (`roll_off` with why) or at its line's end
/// (`roll_end` where the ball is). In a DRUM ROLL still running the next leg starts at once; a DRUM ROLL called off
/// or over ends too (`drum_roll_end`). With no DRUM ROLL left the ball stops rolling and, unless it is held, glides on
/// along its line.
fn end_roll(f: &mut Fight, k: usize, off: Option<&str>) {
    let l = f.roll_charge.rl[k].take();
    match off {
        Some(why) => f.ev("roll_off", k as i64, vec![Value::Str(why.to_string())]),
        None => {
            let p = f.balls[k].p;
            f.ev("roll_end", k as i64, vec![Value::Float(p[0]), Value::Float(p[1])]);
        }
    }
    if let Some(t1) = f.roll_charge.droll[k].as_ref().map(|dr| dr.t1) {
        if off.is_none() && f.t < t1 && next_leg(f, k) {
            return;
        }
        if off.is_some() || f.t >= t1 {
            f.roll_charge.droll[k] = None;
            f.ev("drum_roll_end", k as i64, vec![]);
        }
    }
    if f.roll_charge.droll[k].is_none() {
        set_rolling(f, k, false);
        if let Some(l) = l
            && !f.balls[k].held
        {
            // it glides on along its line
            f.balls[k].v = geom::scale(l.u, f.rules.v);
        }
    }
}

/// `rollc_tick(k, k16)` (5360 to 5372), from `run()` 6091 to 6095, on every 16th `k16` of the fight: on the roll
/// grid, a drum with no roll, no DRUM ROLL and nothing blocking it tips for a new roll, unless it is boxed in against
/// the ropes (a line shorter than `min_len`).
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    if f.roll_charge.rl[k].is_some() || f.roll_charge.droll[k].is_some() || blocked(f, k) {
        return;
    }
    // (k16 - int(w.get('off16', 0))) % max(1, int(round(4 * w['every'])))
    let grid = (py::round_half_even(4.0 * w.every) as i64).max(1);
    if py::imod_py(k16 - w.off16 as i64, grid) != 0 {
        return;
    }
    let tip = (w.tip16 as i64) as f64 * f.rules.beat / 4.0;
    let roll_v = f.roll_charge.roll_v[k];
    let t_ahead = tip + 0.5 * w.span / (roll_v * f.rules.v);
    let (u, ln) = roll_line(f, k, t_ahead, w.jitter, 0.0, w.span);
    if ln < w.min_len {
        // boxed in against the ropes: no room to roll this time
        return;
    }
    start_roll(f, k, 0, tip, u, ln, roll_v);
}

/// `rollc_step(k, dt)` (5373 to 5431), from `run()` 6189 to 6190, every step.
///
/// HEARTBEAT's second thump when due; 1812 OVERTURE's next cannon when due. Without a roll: a DRUM ROLL that is over
/// ends, and one waiting starts its next leg (or ends, with no room anywhere). With one: a blocked drum calls it off
/// (`roll_off`, `held` or `blocked`); a tipping drum is held at the line's start until the roll starts; a rolling one
/// is at its place on the line, the wheel turning by the distance, and the first contact (a DRUM ROLL leg: a contact
/// every `cool` s) is a BOOM (`roll` or `drumroll`; a landed weapon BOOM grows the roll's speed and the drum's size;
/// event `roll_boom` either way). At the line's end the roll ends.
pub fn step(f: &mut Fight, k: usize, _dt: f64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    // for item in list(B.thumps): HEARTBEAT's "dub"
    let thumps = f.roll_charge.thumps[k].clone();
    for item in thumps {
        if f.t + 1e-9 >= item.0 {
            let list = &mut f.roll_charge.thumps[k];
            if let Some(i) = list.iter().position(|x| *x == item) {
                list.remove(i);
            }
            thump(f, k, &item.1, 1);
        }
    }
    if let Some(c) = &f.roll_charge.cannons[k]
        && f.t + 1e-9 >= c.t_next
    {
        cannon(f, k);
    }
    let Some(mut l) = f.roll_charge.rl[k].clone() else {
        if let Some(t1) = f.roll_charge.droll[k].as_ref().map(|dr| dr.t1) {
            // DRUM ROLL waiting (blocked) or between legs
            if f.t >= t1 || (!blocked(f, k) && !next_leg(f, k)) {
                f.roll_charge.droll[k] = None;
                set_rolling(f, k, false);
                f.ev("drum_roll_end", k as i64, vec![]);
            }
        }
        return;
    };
    if blocked(f, k) {
        let why = if f.balls[k].held { "held" } else { "blocked" };
        end_roll(f, k, Some(why));
        return;
    }
    if l.phase == Phase::Tip {
        f.balls[k].p = l.p0;
        if f.t + 1e-9 < l.t0 {
            return;
        }
        l.phase = Phase::Roll;
    }
    let fr = py_min(1.0, (f.t - l.t0) / py_max(1e-9, l.t1 - l.t0));
    let q = [l.p0[0] + (l.p1[0] - l.p0[0]) * fr, l.p0[1] + (l.p1[1] - l.p0[1]) * fr];
    f.roll_charge.roll_ang[k] += geom::hypot(geom::sub(q, f.balls[k].p)) / r;
    f.balls[k].p = q;
    let rel = geom::sub(f.balls[1 - k].p, q);
    let dist = geom::hypot(rel);
    if dist < 2.0 * r + f.balls[k].blade_len && !f.balls[1 - k].held {
        let ab = if l.kind == 1 {
            let dr = f.roll_charge.droll[k]
                .as_ref()
                .expect("a DRUM ROLL leg rolls only while its DRUM ROLL runs");
            Some(dr.ab.clone())
        } else {
            None
        };
        let ready = match &ab {
            None => !l.hit,
            Some(ab) => f.t - l.last >= get_or(ab, "cool", 0.25),
        };
        if ready {
            l.hit = true;
            l.last = f.t;
            let mut side = [-l.u[1], l.u[0]];
            if geom::dot(side, rel) < 0.0 {
                side = [-side[0], -side[1]];
            }
            // hx, hy = (B.p + rel / max(dist, 1e-6) * R).tolist()
            let md = py_max(dist, 1e-6);
            let h = [q[0] + rel[0] / md * r, q[1] + rel[1] / md * r];
            let (dmg, kn, fw) = match &ab {
                None => (w.dmg, w.knock, w.fwd),
                Some(ab) => (param(ab, "dmg"), param(ab, "knock"), get_or(ab, "fwd", w.fwd)),
            };
            // the roll's state stands on the ball before the hit, as Python's dict does
            f.roll_charge.rl[k] = Some(l.clone());
            let how = if ab.is_none() { "roll" } else { "drumroll" };
            if f.damage(k, 1 - k, dmg, how, h, 0) {
                let away = [fw * l.u[0] + (1.0 - fw) * side[0], fw * l.u[1] + (1.0 - fw) * side[1]];
                f.knock(1 - k, away, kn);
                if ab.is_none() {
                    let st = &mut f.roll_charge;
                    let v = st.roll_v[k];
                    st.roll_v[k] = py_min(w.speed_max.unwrap_or(v), v + w.speed_grow);
                    let b = &mut f.balls[k];
                    b.blade_len = py_min(w.size_max.unwrap_or(b.blade_len), b.blade_len + w.size_grow);
                }
            }
            f.ev(
                "roll_boom",
                k as i64,
                vec![Value::Float(h[0]), Value::Float(h[1]), Value::Int(l.kind)],
            );
        }
    }
    f.roll_charge.rl[k] = Some(l);
    if fr >= 1.0 {
        end_roll(f, k, None);
    }
}

/// `_thump(k, ab, i)` (5440 to 5444): one of HEARTBEAT's shockwaves from the ball (`i` 0 the lub, 1 the dub). Event
/// `heartbeat`: `i` and where.
fn thump(f: &mut Fight, k: usize, ab: &AbilityCfg, i: i64) {
    let c = f.balls[k].p;
    let wave = Wave {
        owner: k,
        c,
        r: f.rules.r,
        speed: param(ab, "speed"),
        rmax: param(ab, "radius"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        kind: "shockwave".to_string(),
        hit: false,
        name: ab.name.clone(),
        t0: f.t,
        // slow=None, and none of a ring's options
        slow: None,
        stun: None,
        detune: None,
        grow: false,
        clears: true,
    };
    f.waves.push(wave);
    f.ev("heartbeat", k as i64, vec![Value::Int(i), Value::Float(c[0]), Value::Float(c[1])]);
}

/// `trigger`, kind `heartbeat` (2983 to 2984): `start_heartbeat(k, ab)` (5446 to 5450): the lub now, the dub
/// `int(gap16)` 16ths later (from [`step`]). Only a roller steps its thumps (Python's `B.thumps` is the roller's).
pub fn start_heartbeat(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    thump(f, k, ab, 0);
    let gap = get_or(ab, "gap16", 2.0) as i64;
    let at = f.t + gap as f64 * f.rules.beat / 4.0;
    f.roll_charge.thumps[k].push((at, ab.clone()));
}

/// `_next_leg(k)` (5451 to 5464): DRUM ROLL's next leg, straight away: `int(leg16)` 16ths long at `speed_k` x the
/// roll's speed, aimed at the enemy turned `zig` degrees to the side due, else to the other; false when neither side
/// has room (a line of at least 0.25).
fn next_leg(f: &mut Fight, k: usize) -> bool {
    let dr = f.roll_charge.droll[k]
        .clone()
        .expect("a leg is asked for only while a DRUM ROLL runs");
    let ab = &dr.ab;
    let v = get_or(ab, "speed_k", 1.0) * f.roll_charge.roll_v[k];
    let leg = get_or(ab, "leg16", 3.0) as i64;
    let span = v * f.rules.v * leg as f64 * f.rules.beat / 4.0;
    let zig = get_or(ab, "zig", 30.0);
    for side in [dr.side, -dr.side] {
        let (u, ln) = roll_line(f, k, 0.5 * span / (v * f.rules.v), 0.0, side * zig, span);
        if ln >= 0.25 {
            f.roll_charge.droll[k].as_mut().expect("still running").side = -side;
            start_roll(f, k, 1, 0.0, u, ln, v);
            return true;
        }
    }
    false
}

/// `trigger`, kind `drum_roll` (2985 to 2986): `start_drum_roll(k, ab)` (5466 to 5476): a roll in progress gives way
/// (`roll_off`, `drum_roll`); for `beats` beats the drum is rolling, its first side drawn. Event `drum_roll`: the end
/// time. Unless blocked, the first leg starts at once.
pub fn start_drum_roll(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    if f.roll_charge.rl[k].is_some() {
        // the weapon's roll gives way to the drum roll
        f.roll_charge.rl[k] = None;
        f.ev("roll_off", k as i64, vec![Value::Str("drum_roll".to_string())]);
    }
    let t1 = f.t + param(ab, "beats") * f.rules.beat;
    let side = if f.rng.uniform(0.0, 1.0) < 0.5 { 1.0 } else { -1.0 };
    f.roll_charge.droll[k] = Some(DrumRoll {
        t1,
        side,
        ab: ab.clone(),
    });
    set_rolling(f, k, true);
    f.ev("drum_roll", k as i64, vec![Value::Float(t1)]);
    if !blocked(f, k) {
        next_leg(f, k);
    }
}

/// `trigger`, kind `cannons` (2987 to 2988): `start_cannons(k, ab)` (5477 to 5478): the first cannon fires at the
/// next step (from [`step`]), the rest `gap_beats` beats apart.
pub fn start_cannons(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    f.roll_charge.cannons[k] = Some(Cannons {
        ab: ab.clone(),
        i: 0,
        t_next: f.t,
    });
}

/// `_cannon(k)` (5480 to 5493): one cannon: a slam (`how = 'cannon'`) at the enemy's spot 0.6 x `tele` beats ahead
/// (inside the ropes), landing `tele` beats from now. Event `cannon`: its number, where, when it lands. After the
/// `int(n)`th, 1812 OVERTURE is over.
fn cannon(f: &mut Fight, k: usize) {
    let c = f.roll_charge.cannons[k].clone().expect("checked by the caller");
    let ab = &c.ab;
    let lead = param(ab, "tele") * f.rules.beat;
    let lim = f.rules.a - f.rules.r;
    let (op, ov) = (f.balls[1 - k].p, f.balls[1 - k].v);
    // tgt = np.clip(O.p + O.v * lead * 0.6, -lim, lim)
    let tgt = [
        geom::np_clip(op[0] + ov[0] * lead * 0.6, -lim, lim),
        geom::np_clip(op[1] + ov[1] * lead * 0.6, -lim, lim),
    ];
    let slam = Slam {
        owner: k,
        c: tgt,
        t0: f.t,
        t_slam: f.t + lead,
        radius: param(ab, "radius"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        done: false,
        name: ab.name.clone(),
        how: "cannon".to_string(),
    };
    f.slams.push(slam);
    let data = vec![
        Value::Int(c.i),
        Value::Float(tgt[0]),
        Value::Float(tgt[1]),
        Value::Float(f.t + lead),
    ];
    f.ev("cannon", k as i64, data);
    let i = c.i + 1;
    let t_next = c.t_next + get_or(ab, "gap_beats", 1.0) * f.rules.beat;
    f.roll_charge.cannons[k] = if i >= param(ab, "n") as i64 {
        None
    } else {
        Some(Cannons { ab: c.ab, i, t_next })
    };
}

/// The record keys this kind adds (Python's `rec` gets them at setup, `run()` 5910 to 5911): `RCHG`, when a fighter
/// is a roller.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if is(f, 0) || is(f, 1) { &["RCHG"] } else { &[] }
}

/// This kind's rows of one frame: `snap`, `RCHG` (5713 to 5732), a `(2, 9)` float row, NaN where nothing is live
/// (and all NaN for a fighter that is not a roller). `studio/docs/recording-format.md` 6.4 lists the columns.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut ro = [f64::NAN; 18];
    for j in 0..2 {
        if !is(f, j) {
            continue;
        }
        let row = &mut ro[j * 9..(j + 1) * 9];
        let st = &f.roll_charge;
        let l = &st.rl[j];
        row[0] = match l {
            None => 0.0,
            Some(l) if l.phase == Phase::Tip => 1.0,
            Some(_) => 2.0,
        };
        if let Some(l) = l {
            row[1] = if l.phase == Phase::Tip {
                (f.t - l.t_tip) / py_max(1e-9, l.t0 - l.t_tip)
            } else {
                (f.t - l.t0) / py_max(1e-9, l.t1 - l.t0)
            };
            row[2] = dmath::atan2(l.u[1], l.u[0]);
            row[8] = l.kind as f64;
        }
        row[3] = st.roll_ang[j];
        row[4] = f.balls[j].blade_len;
        row[5] = st.roll_v[j];
        row[6] = if st.droll[j].is_some() { 1.0 } else { 0.0 };
        row[7] = if f.balls[j].rolling { 1.0 } else { 0.0 };
    }
    out.push_f64("RCHG", &[2, 9], &ro);
}

#[cfg(test)]
mod tests {
    use bf_core::recording::Array;

    use super::*;
    use crate::fight::{Setup, run};
    use crate::roster::FighterCfg;
    use crate::testing::{episode_rules, fight, fighter, roster};

    /// BASS DRUM (left) against FLUTE under the example episode's rules.
    fn rolling(roster: &[FighterCfg], seed: u64) -> Setup<'_> {
        Setup {
            fighters: [fighter(roster, "BASS DRUM"), fighter(roster, "FLUTE")],
            rules: episode_rules(),
            seed,
        }
    }

    #[test]
    fn a_roll_tips_on_the_bar_then_rolls_along_its_line() {
        let roster = roster();
        let setup = rolling(&roster, 1);
        let (beat, dt) = (setup.rules.beat, setup.rules.dt);
        let rec = run(&setup, false).expect("the fight runs");
        let rolls: Vec<usize> = (0..rec.events.len())
            .filter(|&i| {
                let e = &rec.events[i];
                e.kind == "roll" && e.who() == Some(0) && e.i(10) == 0
            })
            .collect();
        assert!(rolls.len() >= 3, "a roll a bar");
        let first = &rec.events[rolls[0]];
        // the first roll starts with the fight, on the bar line, after a tip of two 16ths
        assert!((first.t - setup.rules.t_fight).abs() <= dt + 1e-5, "{}", first.t);
        assert!((first.f(8) - (first.f(7) + 2.0 * beat / 4.0)).abs() < 1e-9);
        // its line is at most `span` long, and it reaches the end at the roll's speed
        let len = geom::hypot([first.f(5) - first.f(3), first.f(6) - first.f(4)]);
        assert!((0.6..=5.2 + 1e-9).contains(&len), "{len}");
        assert!((first.f(9) - first.f(8) - len / (2.4 * setup.rules.v)).abs() < 1e-9);
        // a weapon roll ends (or is called off) before the next one starts
        let end = rec.events[rolls[0] + 1..]
            .iter()
            .find(|e| e.who() == Some(0) && matches!(e.kind.as_str(), "roll_end" | "roll_off"))
            .expect("the roll ends");
        assert!(end.t <= rec.events[rolls[1]].t);
    }

    #[test]
    fn the_record_has_a_rchg_row_for_the_roller_alone() {
        let roster = roster();
        let rec = run(&rolling(&roster, 2), true).expect("the fight runs");
        let Some(Array::F64 { shape, data }) = rec.arrays.get("RCHG") else {
            panic!("a roll_charge fight records RCHG as floats")
        };
        assert_eq!(shape[1..], [2, 9]);
        let mut seen = [false; 3];
        for frame in data.chunks(18) {
            let (mine, other) = frame.split_at(9);
            assert!(other.iter().all(|v| v.is_nan()), "FLUTE is no roller");
            let phase = mine[0];
            seen[phase as usize] = true;
            assert_eq!(mine[1].is_nan(), phase == 0.0);
            assert_eq!(mine[8].is_nan(), phase == 0.0);
            // the drum's size and the roll's speed grow from their starts to their caps
            assert!((0.0..=0.25).contains(&mine[4]));
            assert!((2.4..=3.4).contains(&mine[5]));
            // rolling from the tip on (and through DRUM ROLL)
            assert!(phase == 0.0 || mine[7] == 1.0);
        }
        assert!(seen.iter().all(|&s| s), "it stands, tips and rolls: {seen:?}");
        let other = Setup {
            fighters: [fighter(&roster, "FLUTE"), fighter(&roster, "TROMBONE")],
            rules: episode_rules(),
            seed: 2,
        };
        assert!(!run(&other, true).expect("runs").arrays.contains_key("RCHG"));
    }

    #[test]
    fn a_rolling_drum_is_a_walker_and_a_blocked_one_calls_its_roll_off() {
        let roster = roster();
        let mut f = fight(&rolling(&roster, 3), false);
        setup(&mut f, 0);
        assert!(!f.balls[0].rolling && f.roll_charge.rl[0].is_none());
        assert_eq!((f.balls[0].blade_len, f.roll_charge.roll_v[0]), (0.0, 2.4));
        f.t = f.rules.t_fight;
        tick(&mut f, 0, 0);
        let l = f.roll_charge.rl[0].clone().expect("a roll starts on the first 16th");
        assert_eq!((l.phase, l.kind, l.hit, l.last), (Phase::Tip, 0, false, -9.0));
        assert!(f.balls[0].rolling && f.balls[0].walking && f.balls[0].walker);
        // rolling: a knock does not turn it
        let v = f.balls[0].v;
        f.knock(0, [0.0, 1.0], 2.0);
        assert_eq!((f.balls[0].v, f.balls[0].boost), (v, 0.0));
        // the tip holds it at the line's start
        f.balls[0].p = [l.p0[0] + 0.3, l.p0[1]];
        step(&mut f, 0, 0.0);
        assert_eq!(f.balls[0].p, l.p0);
        // held (a gulp): the roll is off, nothing rolls, and it does not glide on
        f.balls[0].held = true;
        f.balls[0].v = [0.0, 0.0];
        step(&mut f, 0, 0.0);
        assert!(f.roll_charge.rl[0].is_none() && !f.balls[0].rolling && !f.balls[0].walker);
        assert_eq!(f.balls[0].v, [0.0, 0.0]);
        let last = f.events.last().expect("an event");
        assert_eq!(
            (last.kind.as_str(), &last.fields[1]),
            ("roll_off", &Value::Str("held".into()))
        );
        // and starts none while held
        tick(&mut f, 0, 16);
        assert!(f.roll_charge.rl[0].is_none());
    }

    #[test]
    fn the_abilities_thump_twice_roll_in_legs_and_fire_five_cannons() {
        let roster = roster();
        let mut f = fight(&rolling(&roster, 4), false);
        setup(&mut f, 0);
        f.t = f.rules.t_fight + 1.0;
        let (beat, dt) = (f.rules.beat, f.rules.dt);
        let cfg = f.balls[0].cfg;
        // HEARTBEAT: the lub at once, the dub two 16ths later
        start_heartbeat(&mut f, 0, &cfg.unlock1);
        assert_eq!(f.waves.len(), 1);
        assert_eq!((f.waves[0].kind.as_str(), f.waves[0].rmax), ("shockwave", 2.0));
        let due = f.roll_charge.thumps[0][0].0;
        assert!((due - (f.t + 2.0 * beat / 4.0)).abs() < 1e-12);
        f.t = due;
        step(&mut f, 0, dt);
        assert_eq!(f.waves.len(), 2);
        assert!(f.roll_charge.thumps[0].is_empty());
        let beats: Vec<i64> = f.events.iter().filter(|e| e.kind == "heartbeat").map(|e| e.i(3)).collect();
        assert_eq!(beats, [0, 1]);
        // 1812 OVERTURE: five cannon slams a beat apart, the first at the next step
        start_cannons(&mut f, 0, &cfg.low);
        for i in 0..5 {
            f.t += beat;
            step(&mut f, 0, dt);
            assert_eq!(f.slams.len(), i + 1);
            assert_eq!(f.slams[i].how, "cannon");
        }
        assert!(f.roll_charge.cannons[0].is_none());
        // DRUM ROLL: rolling at once, legs of DRUM ROLL's kind
        start_drum_roll(&mut f, 0, &cfg.unlock2);
        assert!(f.roll_charge.droll[0].is_some() && f.balls[0].rolling);
        let l = f.roll_charge.rl[0].clone().expect("the first leg");
        assert_eq!((l.phase, l.kind), (Phase::Roll, 1));
        assert!(f.events.iter().any(|e| e.kind == "drum_roll"));
    }
}
