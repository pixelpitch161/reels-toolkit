//! Weapon kind `wrecking`, BARI SAX's WRECKING BALL: `fight.py` 3594 to 3786 (`wreck_setup`, `_wreck_aim`,
//! `wreck_tick`, `_sails_over`, `wreck_step`) and the ability `offbeat` (SKANK, `trigger` 2911 to 2914), whose cycles
//! run in `wreck_tick`.
//!
//! Every `every` beats (from 16th `off16`; during SKANK every bar on 16th `skank_off16`) a crane cable drops for
//! `drop16` 16ths and hooks the ball: the swing is aimed (`_wreck_aim`: where the enemy will be at the swing's bottom,
//! where a hop lands, round an anchored whirl; `short` ball diameters short of it, turned by up to `jitter` degrees,
//! seeded; at most `blade_len * sin(theta)` away; the half swing that fits the ring, out over the ropes with
//! `overhang`, its plane turned by up to `steer` degrees near a corner), the ball is hoisted back for `lift16` 16ths
//! and swung through a pendulum arc for `swing16` 16ths: what is in front of it within `hit_r` beyond contact takes
//! `dmg` (from `dmg_min` of it at no speed, x (cable / base) ^ `len_pow`) and `knock` (unless it sails over it: up in a
//! hop past `air_edge` of it); a landed smash grows the cable by `grow` up to `max_reach`. At the far end it unhooks
//! and rolls on, or (out over the ropes) drops on them and is slung back in over `back16` 16ths. In the air it cannot
//! clash. Events `cable`, `swing`, `smash`, `wreck_out`, `wreck_in`, `skank`; recorded as `WRECK`.
//!
//! Python hangs `wreck`, `skank_until` and `skank_ab` on the ball; here they are [`State`], `Fight::wrecking`, indexed
//! by fighter (`Ball::air` and `Ball::blade_len` are the ball's). The generator is drawn once per swing aimed (the
//! jitter), nowhere else.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::ab_dur;
use crate::fight::{Fight, stunned};
use crate::geom::{self, V2};
use crate::kinds::mortar;
use crate::record::Frames;
use crate::roster::{AbilityCfg, WeaponCfg, WreckingCfg};

/// A cycle's phase (`WRECK_PHASES`: 1 drop, 2 lift, 3 swing, 4 return).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The cable drops.
    Drop,
    /// Hoisted back.
    Lift,
    /// The pendulum.
    Swing,
    /// #40 v2: dropped on the ropes, slung back in.
    Return,
}

impl Phase {
    /// `WRECK_PHASES[phase]`.
    pub fn code(self) -> f64 {
        match self {
            Phase::Drop => 1.0,
            Phase::Lift => 2.0,
            Phase::Swing => 3.0,
            Phase::Return => 4.0,
        }
    }
}

/// The swing fixed when the hook bites (`_wreck_aim`'s `W_.update(...)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aim {
    /// The swing's direction.
    pub u: V2,
    /// The cable's length.
    pub l: f64,
    /// The swing's widest angle.
    pub th: f64,
    /// Half the swing on the ground.
    pub half: f64,
    /// Where the hook bit.
    pub p0: V2,
    /// The swing's bottom.
    pub bottom: V2,
    /// Its start (hoisted back to).
    pub ps: V2,
    /// The height it is hoisted to.
    pub h0: f64,
}

/// Dropped on the ropes, slung back in (the `return` phase's keys).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Back {
    /// Where it went out.
    pub p_out: V2,
    /// Where it lands inside.
    pub q: V2,
    /// Its height when it went out.
    pub h_out: f64,
    /// The direction back in.
    pub vin: V2,
}

/// A cycle under way (`B.wreck`, the dict `wreck_tick` makes and the phases fill).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wreck {
    /// The phase.
    pub phase: Phase,
    /// The phase's start.
    pub t0: f64,
    /// The phase's end.
    pub t1: f64,
    /// 0 an ordinary swing, 1 a SKANK swing.
    pub kind: i64,
    /// Whether this swing has smashed.
    pub hit: bool,
    /// The swing (`None` until the hook bites).
    pub aim: Option<Aim>,
    /// The height (`W_['h']`: `None` until the lift starts).
    pub h: Option<f64>,
    /// The way back in (the `return` phase).
    pub back: Option<Back>,
}

/// Weapon kind `wrecking`'s per-fight state (`Fight::wrecking`), indexed by fighter.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// `B.wreck`: the cycle under way.
    pub wreck: [Option<Wreck>; 2],
    /// `B.skank_until`: SKANK lasts until then (-1.0: no SKANK yet).
    pub skank_until: [f64; 2],
}

impl Default for State {
    fn default() -> Self {
        State {
            wreck: [None, None],
            skank_until: [-1.0, -1.0],
        }
    }
}

/// `B.w['kind'] == 'wrecking'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Wrecking(_))
}

/// The wrecking ball's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a wrecking ball.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a WreckingCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Wrecking(w) => w,
        other => panic!("fighter {k} ({}) has no wrecking weapon: {other:?}", cfg.name),
    }
}

/// `B.w.get('air_edge', 0.0)` of any weapon: only a wrecking ball has one.
pub fn air_edge(f: &Fight, k: usize) -> f64 {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Wrecking(w) => w.air_edge,
        _ => 0.0,
    }
}

/// `wreck_setup(k)` (3594 to 3600), from `run()` 5842 to 5844.
pub fn setup(f: &mut Fight, k: usize) {
    let base = weapon(f, k).base;
    f.balls[k].blade_len = base;
    f.wrecking.wreck[k] = None;
    // B.skank_until = getattr(B, 'skank_until', -1.0): kept
    f.balls[k].air = false;
}

/// `np.clip(p, -lim, lim)` of a point.
fn clip2(p: V2, lo: f64, hi: f64) -> V2 {
    [geom::np_clip(p[0], lo, hi), geom::np_clip(p[1], lo, hi)]
}

/// `[u[0] * cos(a) - u[1] * sin(a), u[0] * sin(a) + u[1] * cos(a)]`: `u` turned by `a`.
fn turn(u: V2, a: f64) -> V2 {
    let (c, s) = (dmath::cos(a), dmath::sin(a));
    [u[0] * c - u[1] * s, u[0] * s + u[1] * c]
}

/// `span(sg)` of `_wreck_aim`: how far from `bottom` along `sg * u` before the ropes (out over them by `over`, but
/// never past a corner post).
fn span(u: V2, bottom: V2, sg: f64, lim: f64, over: f64, cl1: f64) -> f64 {
    let mut sp = 99.0;
    if over != 0.0 {
        let v_ = geom::scale(u, sg);
        let walls: [([f64; 2], f64); 8] = [
            ([1.0, 0.0], lim + over),
            ([-1.0, 0.0], lim + over),
            ([0.0, 1.0], lim + over),
            ([0.0, -1.0], lim + over),
            ([1.0, 1.0], cl1),
            ([1.0, -1.0], cl1),
            ([-1.0, 1.0], cl1),
            ([-1.0, -1.0], cl1),
        ];
        for (a_, c_) in walls {
            let av = a_[0] * v_[0] + a_[1] * v_[1];
            if av > 1e-9 {
                sp = py::py_min(sp, (c_ - (a_[0] * bottom[0] + a_[1] * bottom[1])) / av);
            }
        }
        return py::py_max(0.0, sp);
    }
    for ax in 0..2 {
        if u[ax].abs() > 1e-9 {
            sp = py::py_min(sp, (lim.copysign(sg * u[ax]) - bottom[ax]) / (sg * u[ax]));
        }
    }
    py::py_max(0.0, sp)
}

/// `_wreck_aim(k)` (3601 to 3664): fix the swing when the hook bites: the aim, the bottom point (under the pivot), the
/// angle that fits the ring. One draw (the jitter) when the weapon has one.
fn wreck_aim(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let s16 = f.rules.beat / 4.0;
    let t_bot = f.t + (w.lift16 + w.swing16 / 2.0) * s16;
    let o = 1 - k;
    let tgt = if let Some(h) = f.tiptoe.hop[o] {
        // a hopper mid-hop: it will stand where it lands
        h.p1
    } else if f.balls[o].anc.is_some() {
        // #40: a whirling (anchored) enemy: lead it round its whirl
        mortar::predict(f, o, (t_bot - f.t) * w.lead)
    } else {
        // O.p + O.v * (t_bot - self.t) * lead
        let (op, ov) = (f.balls[o].p, f.balls[o].v);
        let dt = t_bot - f.t;
        [op[0] + ov[0] * dt * w.lead, op[1] + ov[1] * dt * w.lead]
    };
    let bp = f.balls[k].p;
    let d = geom::sub(tgt, bp);
    let mut n = geom::hypot(d);
    let mut u = if n > 1e-6 {
        geom::div(d, n)
    } else {
        let bv = f.balls[k].v;
        geom::div(bv, geom::norm(bv) + 1e-12)
    };
    let kind = f.wrecking.wreck[k].expect("a cycle to aim").kind;
    let mut short = w.short;
    if kind == 1 {
        // a SKANK swing (its bottom on the off-beat) may aim apart
        short = w.skank_short;
    }
    n = py::py_max(0.0, n - short * 2.0 * r);
    if w.jitter > 0.0 {
        // the crane operator's aim: seeded, +- `jitter` deg
        let j = geom::radians(f.rng.uniform(-1.0, 1.0) * w.jitter);
        u = turn(u, j);
    }
    let l = f.balls[k].blade_len;
    let reach = l * dmath::sin(geom::radians(w.theta));
    let lim = f.rules.a - r;
    let mut bottom = clip2(geom::add(bp, geom::scale(u, py::py_min(n, reach))), -lim, lim);
    let over = w.overhang;
    let cl1 = w.corner_l1.unwrap_or(2.0 * lim + 0.4);
    let fits = |u: V2, bottom: V2| {
        py::py_max(
            0.15,
            py::py_min(
                py::py_min(reach, span(u, bottom, 1.0, lim, over, cl1)),
                span(u, bottom, -1.0, lim, over, cl1),
            ),
        )
    };
    let mut half = fits(u, bottom);
    let steer = w.steer;
    if steer != 0.0 && half < reach - 1e-6 {
        // #39 v4: no room for a full swing (a corner): turn the swing's plane (up to `steer` deg) to where it has
        // room, still bottoming out just short of the target
        let s2 = short * 2.0 * r;
        let aim = geom::add(bottom, geom::scale(u, s2));
        let (u0, b0) = (u, bottom);
        let mut best = (half, 0.0, u0, b0);
        let steps = py::floor_div(steer, 10.0) as i64;
        for j in 1..=steps {
            for sg in [1.0, -1.0] {
                let a = geom::radians(10.0 * j as f64 * sg);
                let uj = turn(u0, a);
                let bj = clip2(geom::sub(aim, geom::scale(uj, s2)), -lim, lim);
                let hv = fits(uj, bj);
                if hv > best.0 + 0.05 {
                    best = (hv, a, uj, bj);
                }
            }
        }
        (half, _, u, bottom) = best;
    }
    let th = dmath::asin(py::py_min(1.0, half / l));
    let p0 = f.balls[k].p;
    let wr = f.wrecking.wreck[k].as_mut().expect("a cycle to aim");
    wr.aim = Some(Aim {
        u,
        l,
        th,
        half,
        p0,
        bottom,
        ps: geom::sub(bottom, geom::scale(u, half)),
        h0: l * (1.0 - dmath::cos(th)),
    });
}

/// `wreck_tick(k, k16)` (3666 to 3684), from `run()` 6040 to 6044 (its own counter, `n16w`): on 16th `k16` of the
/// fight, unless a cycle or a CABLE SNAP is under way or it is stunned, a cable drops on the grid (during SKANK every
/// bar on `skank_off16`). Event `cable`.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    if f.wrecking.wreck[k].is_some() || f.snap.wsnap[k].is_some() || stunned(f, k) {
        return;
    }
    let s16 = f.rules.beat / 4.0;
    let skank = f.t < f.wrecking.skank_until[k];
    let start = if skank {
        py::imod_py(k16, 16) == w.skank_off16 as i64
    } else {
        let main16 = py::round_half_even(4.0 * w.every) as i64;
        let off = w.off16 as i64;
        k16 >= off && py::imod_py(k16 - off, main16) == 0
    };
    if !start {
        return;
    }
    let t_hook = f.t + w.drop16 * s16;
    f.wrecking.wreck[k] = Some(Wreck {
        phase: Phase::Drop,
        t0: f.t,
        t1: t_hook,
        kind: i64::from(skank),
        hit: false,
        aim: None,
        h: None,
        back: None,
    });
    f.ev("cable", k as i64, vec![Value::Float(t_hook)]);
}

/// `_sails_over(k, edge)` (3685 to 3695): is ball `k` up in the air, clear of a swinging wrecking ball? (Mid-hop,
/// past the first and before the last `edge` of the hop, where it is still low.)
pub fn sails_over(f: &Fight, k: usize, edge: f64) -> bool {
    if !f.balls[k].air {
        return false;
    }
    let Some(h) = f.tiptoe.hop[k] else {
        return true;
    };
    if edge <= 0.0 {
        return true;
    }
    let u = (f.t - h.t0) / py::py_max(1e-6, h.t1 - h.t0);
    edge < u && u < 1.0 - edge
}

/// `wreck_step(k, dt)` (3697 to 3786), from `run()` 6167 to 6168: the cycle's phase moves the ball.
pub fn step(f: &mut Fight, k: usize, _dt: f64) {
    let Some(mut wr) = f.wrecking.wreck[k] else {
        return;
    };
    let w = weapon(f, k);
    let r = f.rules.r;
    let s16 = f.rules.beat / 4.0;
    match wr.phase {
        Phase::Drop => {
            if f.t + 1e-9 >= wr.t1 {
                // hooked: aim the swing, hoist it back
                wreck_aim(f, k);
                let wr = f.wrecking.wreck[k].as_mut().expect("the cycle");
                wr.phase = Phase::Lift;
                wr.t0 = f.t;
                wr.t1 = f.t + w.lift16 * s16;
                f.balls[k].air = true;
            }
        }
        Phase::Return => {
            // #40 v2: dropped on the ropes, slung back into the ring
            let bk = wr.back.expect("the way back");
            let uu = py::py_min(1.0, (f.t - wr.t0) / py::py_max(1e-6, wr.t1 - wr.t0));
            let e = uu * uu * (3.0 - 2.0 * uu);
            f.balls[k].p = geom::add(bk.p_out, geom::scale(geom::sub(bk.q, bk.p_out), e));
            wr.h = Some(bk.h_out * (1.0 - uu) + 0.25 * dmath::sin(std::f64::consts::PI * uu));
            f.balls[k].v = geom::scale(bk.vin, py::py_max(0.3, 6.0 * uu * (1.0 - uu)));
            f.wrecking.wreck[k] = Some(wr);
            if uu >= 1.0 {
                let b = &mut f.balls[k];
                b.p = bk.q;
                b.v = geom::scale(bk.vin, f.rules.v);
                b.air = false;
                f.wrecking.wreck[k] = None;
                let p = f.balls[k].p;
                f.ev("wreck_in", k as i64, vec![Value::Float(p[0]), Value::Float(p[1])]);
            }
        }
        Phase::Lift => {
            let a = wr.aim.expect("the swing");
            let uu = py::py_min(1.0, (f.t - wr.t0) / py::py_max(1e-6, wr.t1 - wr.t0));
            let e = uu * uu * (3.0 - 2.0 * uu);
            f.balls[k].p = geom::add(a.p0, geom::scale(geom::sub(a.ps, a.p0), e));
            wr.h = Some(a.h0 * e);
            // hoisted: barely moving (what the enemy reads)
            f.balls[k].v = geom::scale(a.u, 0.3);
            if uu >= 1.0 {
                let t_ = w.swing16 * s16;
                wr.phase = Phase::Swing;
                wr.t0 = f.t;
                wr.t1 = f.t + t_;
                let p1 = geom::add(a.bottom, geom::scale(a.u, a.half));
                f.wrecking.wreck[k] = Some(wr);
                let data = vec![
                    Value::Float(a.ps[0]),
                    Value::Float(a.ps[1]),
                    Value::Float(p1[0]),
                    Value::Float(p1[1]),
                    Value::Float(a.bottom[0]),
                    Value::Float(a.bottom[1]),
                    Value::Float(a.l),
                    Value::Float(a.th),
                    Value::Float(wr.t0),
                    Value::Float(wr.t1),
                    Value::Int(wr.kind),
                ];
                f.ev("swing", k as i64, data);
            } else {
                f.wrecking.wreck[k] = Some(wr);
            }
        }
        Phase::Swing => swing(f, k, wr, w, r, s16),
    }
}

/// The swing phase of [`step`]: a pendulum's half period, fastest at the bottom; what is in front of it smashes
/// (`smash`); at the far end it unhooks (or, out over the ropes, drops on them: `wreck_out`).
fn swing(f: &mut Fight, k: usize, mut wr: Wreck, w: &WreckingCfg, r: f64, s16: f64) {
    let pi = std::f64::consts::PI;
    let a = wr.aim.expect("the swing");
    let t_ = wr.t1 - wr.t0;
    let uu = py::py_min(1.0, (f.t - wr.t0) / t_);
    let (l, th, u) = (a.l, a.th, a.u);
    let phi = -th * dmath::cos(pi * uu);
    f.balls[k].p = geom::add(a.bottom, geom::scale(u, l * dmath::sin(phi)));
    let h = l * (1.0 - dmath::cos(phi));
    wr.h = Some(h);
    // 0..1: the swing's speed
    let sp = dmath::sin(pi * uu);
    // ground speed
    let gv = l * dmath::cos(phi) * th * pi / t_ * sp;
    f.balls[k].v = geom::scale(u, py::py_max(0.3, gv));
    let o = 1 - k;
    if !wr.hit && !sails_over(f, o, w.air_edge) {
        let rel = geom::sub(f.balls[o].p, f.balls[k].p);
        let d3 = (geom::dot(rel, rel) + dmath::pow(h, 2.0)).sqrt();
        if d3 < 2.0 * r + w.hit_r && geom::dot(rel, u) > 0.0 {
            // only what is in front of the swing
            wr.hit = true;
            let dist = geom::hypot(rel);
            let n = if dist > 1e-6 { geom::div(rel, dist) } else { u };
            let at = geom::sub(f.balls[o].p, geom::scale(n, r));
            let dmg = w.dmg * (w.dmg_min + (1.0 - w.dmg_min) * sp) * dmath::pow(l / w.base, w.len_pow);
            f.wrecking.wreck[k] = Some(wr);
            let landed = f.damage(k, o, dmg, "wreck", at, 0);
            f.knock(o, geom::add(u, geom::scale(n, 0.6)), w.knock * (0.5 + 0.5 * sp));
            f.ev(
                "smash",
                k as i64,
                vec![Value::Float(at[0]), Value::Float(at[1]), Value::Float(sp)],
            );
            if landed {
                let b = &mut f.balls[k];
                b.blade_len = py::py_min(w.max_reach, b.blade_len + w.grow);
            }
        }
    }
    f.wrecking.wreck[k] = Some(wr);
    if uu >= 1.0 {
        // the far end: unhook and roll on along the swing
        f.balls[k].p = geom::add(a.bottom, geom::scale(u, a.half));
        let lim = f.rules.a - r;
        let p = f.balls[k].p;
        if w.overhang != 0.0 && (p[0].abs() > lim || p[1].abs() > lim) {
            // #40 v2: out over the ropes
            let back_in = w.back_in;
            let q = clip2(p, -(lim - back_in), lim - back_in);
            let d = geom::sub(q, p);
            let vin = geom::div(d, geom::hypot(d) + 1e-9);
            let t1 = f.t + w.back16 * s16;
            wr.phase = Phase::Return;
            wr.t0 = f.t;
            wr.t1 = t1;
            wr.back = Some(Back {
                p_out: p,
                q,
                h_out: h,
                vin,
            });
            f.wrecking.wreck[k] = Some(wr);
            let data = vec![
                Value::Float(p[0]),
                Value::Float(p[1]),
                Value::Float(q[0]),
                Value::Float(q[1]),
                Value::Float(f.t),
                Value::Float(t1),
            ];
            f.ev("wreck_out", k as i64, data);
            return;
        }
        let b = &mut f.balls[k];
        b.v = geom::scale(u, f.rules.v);
        b.air = false;
        f.wrecking.wreck[k] = None;
    }
}

/// `trigger`, kind `offbeat` (2911 to 2914): SKANK: for its duration a swing every bar on the off-beat. Event
/// `skank`.
pub fn start_offbeat(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + ab_dur(f, ab);
    f.wrecking.skank_until[k] = until;
    f.ev("skank", k as i64, vec![Value::Float(until)]);
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5846 to 5847: `WRECK`, when a
/// fighter's weapon is a wrecking ball.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) { &["WRECK"] } else { &[] }
}

/// This kind's rows of one frame: `snap`, `WRECK` (5577 to 5587): per ball the phase, the height, the cable's length,
/// the swing's bottom, where the hook bit, the phase's start and end (the cable's length alone between cycles).
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut row = [0.0; 18];
    for j in 0..2 {
        let b = &f.balls[j];
        let o = 9 * j;
        row[o + 2] = b.blade_len;
        if let Some(wr) = &f.wrecking.wreck[j] {
            let (bottom, p0, l) = match &wr.aim {
                Some(a) => (a.bottom, a.p0, a.l),
                None => (b.p, b.p, b.blade_len),
            };
            row[o..o + 9].copy_from_slice(&[
                wr.phase.code(),
                wr.h.unwrap_or(0.0),
                l,
                bottom[0],
                bottom[1],
                p0[0],
                p0[1],
                wr.t0,
                wr.t1,
            ]);
        }
    }
    // not one of the kit's array keys (`story.REC_KEYS`): a ragged list in a recording, each ball's row of nine floats
    let rows = row
        .chunks(9)
        .map(|r| Value::List(r.iter().copied().map(Value::Float).collect()));
    out.push_nested("WRECK", Value::List(rows.collect()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swing_spans_to_the_ropes() {
        // inside the ring: from the centre along +x the rope is at lim
        assert_eq!(span([1.0, 0.0], [0.0, 0.0], 1.0, 2.4, 0.0, 5.2), 2.4);
        assert_eq!(span([1.0, 0.0], [1.0, 0.0], -1.0, 2.4, 0.0, 5.2), 3.4);
        // out over the ropes by 1.3, never past a corner post
        assert_eq!(span([1.0, 0.0], [0.0, 0.0], 1.0, 2.4, 1.3, 5.2), 2.4 + 1.3);
        assert!(span([0.6, 0.8], [2.0, 2.0], 1.0, 2.4, 1.3, 5.2) < 1.0);
    }

    #[test]
    fn phases_have_the_kits_codes() {
        let codes: Vec<f64> = [Phase::Drop, Phase::Lift, Phase::Swing, Phase::Return]
            .iter()
            .map(|p| p.code())
            .collect();
        assert_eq!(codes, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn a_turn_is_a_rotation() {
        let u = turn([1.0, 0.0], std::f64::consts::FRAC_PI_2);
        assert!((u[0]).abs() < 1e-15 && (u[1] - 1.0).abs() < 1e-15);
    }
}
