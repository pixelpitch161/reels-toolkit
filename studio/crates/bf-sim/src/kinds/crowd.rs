//! Weapon kind `crowd`, TENOR SAX's CROWD PLEASER: `fight.py` 4513 to 4757 (`crowd_setup`, `_crowd_volley`,
//! `_launch_throw`, `crowd_tick`, `crowd_step`, `start_runway`, `strut_step`, `start_dive`, `dive_step`), and the
//! abilities `runway`, `subtone` and `stagedive` (`trigger` 2966 to 2976).
//!
//! Every `every` beats (`off16` 16ths later; haste divides it) the tenor strikes a pose for `pose16` 16ths (it all
//! but stops: its speed x `pose_v`) and the crowd hurls `int(hype)` things at the enemy from the stands, one every
//! `stagger16` 16ths from a 16th after the pose. Each throw leaves the stands beyond the rope nearest the enemy's
//! predicted spot, flies `flight16` 16ths and lands on that spot, but no further into the ring from that rope than
//! the reach. A landing within `splash` (+ 0.6 R) of the enemy hits it and knocks it along the throw; the first
//! landed throw of a volley grows the hype, the reach and the damage.
//!
//! WALK THE BAR (`runway`): a strut from the tenor through the enemy to the far ropes, bumping the enemy aside and
//! honking a ring on every beat. SUBTONE (`subtone`): a hush, then one volley twice as hard. STAGE DIVE
//! (`stagedive`): a leap beyond the nearest rope, a surf along the stands, then a cannonball at the enemy's
//! predicted spot and a ring where it lands.
//!
//! The state Python hangs on the fight (`throws`, `pend_throws`, `next_throw`, `next_cvol`, `cvol_landed`) and on a
//! ball (`throw_dmg`, `pose_until`, `pose_t0`, `hush`) is [`State`]; `B.strut` and `B.dive`, which the shared engine
//! reads, are `Ball::strut` and `Ball::dive`; the hype and the reach are the ball's own `n_vol` and `blade_len`.
//! Three draws from the fight's generator per throw launched, in Python's order: `uniform(-1, 1, 2)` for the
//! scatter, then `uniform(-1, 1)` for the spread along the stands.
//!
//! The three ability starts run on whichever ball fires them, as `trigger` does (a ball that copies one need not be
//! a crowd pleaser): Python sets `B.hush`, `B.strut` or `B.dive` on it all the same. Only a crowd pleaser's are
//! stepped (`crowd_tick` and `crowd_step` run for the fighters whose weapon is the kind), so on another ball a
//! hush never ends and a strut or a dive never moves on, exactly as in the kit.

use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, Wave, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::kinds::crowd::CrowdCfg;
use crate::roster::{AbilityCfg, WeaponCfg};

/// One throw waiting for its launch: an entry of `self.pend_throws` (`_crowd_volley` 4561 to 4562).
#[derive(Clone, Debug, PartialEq)]
pub struct Pending {
    /// The crowd pleaser it is thrown for.
    pub owner: usize,
    /// When it leaves the stands.
    pub t_launch: f64,
    /// Its volley's number.
    pub vol: i64,
    /// Its place in the volley.
    pub j: i64,
    /// Its damage (`B.throw_dmg * dmg_k` when the volley was called).
    pub dmg: f64,
    /// Its reach into the ring (`B.blade_len * reach_k` when the volley was called).
    pub reach: f64,
}

/// One throw in the air or just landed: an entry of `self.throws` (`_launch_throw` 4582 to 4583).
#[derive(Clone, Debug, PartialEq)]
pub struct Throw {
    /// The crowd pleaser it is thrown for.
    pub owner: usize,
    /// Its number, in launch order over the fight.
    pub id: i64,
    /// Where it leaves the stands.
    pub p0: V2,
    /// Where it lands.
    pub p1: V2,
    /// When it left.
    pub t0: f64,
    /// When it lands.
    pub t1: f64,
    /// 0 a tomato, 1 a rose, 2 a foam finger (`(vol + j) % 3`; the picture's).
    pub item: i64,
    /// 0 flying, 1 landed and hit, 2 landed and missed.
    pub landed: i64,
    /// Its damage.
    pub dmg: f64,
    /// Its volley's number.
    pub vol: i64,
}

/// `B.hush`: `dict(ab=ab, t_throw=...)`, SUBTONE's hush.
#[derive(Clone, Debug, PartialEq)]
pub struct Hush {
    /// The ability (its `n_k`, `mult`, `reach_k`).
    pub ab: AbilityCfg,
    /// When the hush ends in its volley.
    pub t_throw: f64,
}

/// What `crowd_setup` and `trigger` hang on one ball besides `strut` and `dive`.
#[derive(Clone, Debug, PartialEq)]
pub struct BallCrowd {
    /// `hasattr(B, 'throw_dmg')`: `crowd_setup` ran for this ball (`snap` writes its `CROWD` row only then).
    pub set_up: bool,
    /// `B.throw_dmg`: the damage of its next throws.
    pub throw_dmg: f64,
    /// `B.pose_until`: when its pose ends (-1.0 when it is not posing).
    pub pose_until: f64,
    /// `B.pose_t0`: when its last pose began.
    pub pose_t0: f64,
    /// `B.hush`.
    pub hush: Option<Hush>,
}

impl Default for BallCrowd {
    fn default() -> BallCrowd {
        BallCrowd {
            set_up: false,
            throw_dmg: 0.0,
            pose_until: -1.0,
            pose_t0: -1.0,
            hush: None,
        }
    }
}

/// Weapon kind `crowd`'s per-fight state (`Fight::crowd`): what `fight.py` hangs on the fight and on each ball for
/// this kind.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// By ball.
    pub balls: [BallCrowd; 2],
    /// `self.throws`: the throws in the air, and the landed ones for 0.7 s more.
    pub throws: Vec<Throw>,
    /// `self.pend_throws`: the throws called and not launched yet.
    pub pend_throws: Vec<Pending>,
    /// `self.next_throw`: the next throw's id.
    pub next_throw: i64,
    /// `self.next_cvol`: the next volley's number.
    pub next_cvol: i64,
    /// `self.cvol_landed`: the volleys that have landed a throw (each grows the crowd pleaser once).
    pub cvol_landed: Vec<i64>,
}

/// `B.strut` while WALK THE BAR runs (`start_runway` 4648 to 4664): `Ball::strut`.
#[derive(Clone, Debug, PartialEq)]
pub struct Strut {
    /// The ability (its `dmg`, `knock`, `cool` and the honk's numbers).
    pub ab: AbilityCfg,
    /// Where the strut began.
    pub p0: V2,
    /// Where it ends.
    pub p1: V2,
    /// When it began.
    pub t0: f64,
    /// When it ends.
    pub t1: f64,
    /// When the next honk is due.
    pub next: f64,
    /// When it last bumped the enemy (-9.0 before the first).
    pub last: f64,
}

/// `B.dive` while STAGE DIVE runs (`start_dive` 4692 to 4709): `Ball::dive`.
#[derive(Clone, Debug, PartialEq)]
pub struct Dive {
    /// The ability.
    pub ab: AbilityCfg,
    /// 1 the leap, 2 the surf, 3 the cannonball.
    pub phase: i64,
    /// Where the leap began.
    pub p0: V2,
    /// Where the leap ends, in the stands.
    pub out: V2,
    /// The axis of the rope it leapt over.
    pub ax: usize,
    /// That rope's side, 1.0 or -1.0.
    pub sg: f64,
    /// When the leap began.
    pub t0: f64,
    /// When the surf begins.
    pub t_surf: f64,
    /// When the cannonball is fired (the planned time, then the step it happened in).
    pub t_launch: f64,
    /// When the cannonball lands (`None` before the launch).
    pub t_land: Option<f64>,
    /// Whether the cannonball has hit.
    pub hit: bool,
    /// Where the surf began (`Dv['s0']`, set at the end of the leap).
    pub s0: V2,
    /// Where the cannonball is fired from (`Dv['l0']`).
    pub l0: V2,
    /// Where it lands (`Dv['l1']`).
    pub l1: V2,
}

/// `B.w['kind'] == 'crowd'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Crowd(_))
}

/// The crowd pleaser's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k` is no crowd pleaser.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a CrowdCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Crowd(w) => w,
        other => panic!("fighter {k} ({}) has no crowd weapon: {other:?}", cfg.name),
    }
}

/// `dd, ax, sg = min(dists)` with `dists = [(A - p[0], 0, 1.0), (A + p[0], 0, -1.0), (A - p[1], 1, 1.0),
/// (A + p[1], 1, -1.0)]`: the rope nearest `p`, as Python's `min` of tuples picks it (the smallest distance; of
/// equal distances the lower axis, then the lower sign; of equal tuples the first).
fn nearest_rope(a: f64, p: V2) -> (f64, usize, f64) {
    let dists = [
        (a - p[0], 0usize, 1.0f64),
        (a + p[0], 0, -1.0),
        (a - p[1], 1, 1.0),
        (a + p[1], 1, -1.0),
    ];
    let mut best = dists[0];
    for cand in &dists[1..] {
        // tuple `<`: the first element that differs decides
        let less = if cand.0 != best.0 {
            cand.0 < best.0
        } else if cand.1 != best.1 {
            cand.1 < best.1
        } else {
            cand.2 < best.2
        };
        if less {
            best = *cand;
        }
    }
    best
}

/// `u * u * (3 - 2 * u)`.
fn smooth(u: f64) -> f64 {
    u * u * (3.0 - 2.0 * u)
}

/// `crowd_setup(k)` (4538 to 4551), from `run()` 5894 to 5896
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let b = &mut f.balls[k];
    b.n_vol = w.n0;
    b.blade_len = w.reach;
    b.strut = None;
    b.dive = None;
    f.crowd.balls[k] = BallCrowd {
        set_up: true,
        throw_dmg: w.dmg,
        pose_until: -1.0,
        pose_t0: -1.0,
        hush: None,
    };
    // `if not hasattr(self, 'throws')`: the fight's lists start empty and the counters at 0 (`State::default`)
}

/// `_crowd_volley(k, n, dmg_k=1.0, reach_k=1.0)` (4553 to 4562): `n` throws wait for their launches, one every
/// `stagger16` 16ths from a 16th after now.
fn volley(f: &mut Fight, k: usize, n: i64, dmg_k: f64, reach_k: f64) {
    let w = weapon(f, k);
    let s16 = f.rules.beat / 4.0;
    let vol = f.crowd.next_cvol;
    f.crowd.next_cvol += 1;
    let (dmg, reach) = (f.crowd.balls[k].throw_dmg * dmg_k, f.balls[k].blade_len * reach_k);
    for j in 0..n {
        // tl = self.t + (1 + j * w.get('stagger16', 1)) * s16
        let t_launch = f.t + (1.0 + j as f64 * w.stagger16) * s16;
        f.crowd.pend_throws.push(Pending {
            owner: k,
            t_launch,
            vol,
            j,
            dmg,
            reach,
        });
    }
}

/// `_launch_throw(P_)` (4564 to 4586): the throw leaves the stands. Three draws. Event `throw`: its id, where it
/// leaves, where it lands, when it left, when it lands, its item and its volley.
fn launch_throw(f: &mut Fight, pending: &Pending) {
    let k = pending.owner;
    let w = weapon(f, k);
    let a = f.rules.a;
    let lim = a - f.rules.r;
    let fl = w.flight16 * f.rules.beat / 4.0;
    let (o_p, o_v) = (f.balls[1 - k].p, f.balls[1 - k].v);
    // tgt = O.p + O.v * fl * w.get('lead', 0.7) + self.rng.uniform(-1, 1, 2) * w.get('scatter', 0.0)
    let mut draws = [0.0; 2];
    f.rng.fill_uniform(-1.0, 1.0, &mut draws);
    let ahead = geom::scale(geom::scale(o_v, fl), w.lead);
    let tgt = geom::add(geom::add(o_p, ahead), geom::scale(draws, w.scatter));
    // tgt = np.clip(tgt, -lim, lim)
    let mut tgt = [geom::np_clip(tgt[0], -lim, lim), geom::np_clip(tgt[1], -lim, lim)];
    let (dd, ax, sg) = nearest_rope(a, tgt);
    let mut o = tgt;
    o[ax] = sg * (a + w.stand);
    // o[1 - ax] = float(np.clip(tgt[1 - ax] + self.rng.uniform(-1, 1) * w.get('stand_spread', 0.8), -A, A))
    o[1 - ax] = geom::np_clip(tgt[1 - ax] + f.rng.uniform(-1.0, 1.0) * w.stand_spread, -a, a);
    if dd > pending.reach {
        // out of range: it falls short, `reach` into the ring
        tgt[ax] = sg * (a - pending.reach);
    }
    let tid = f.crowd.next_throw;
    f.crowd.next_throw += 1;
    let item = py::imod_py(pending.vol + pending.j, 3);
    let (t0, t1) = (f.t, f.t + fl);
    f.crowd.throws.push(Throw {
        owner: k,
        id: tid,
        p0: o,
        p1: tgt,
        t0,
        t1,
        item,
        landed: 0,
        dmg: pending.dmg,
        vol: pending.vol,
    });
    let data = vec![
        Value::Int(tid),
        Value::Float(o[0]),
        Value::Float(o[1]),
        Value::Float(tgt[0]),
        Value::Float(tgt[1]),
        Value::Float(t0),
        Value::Float(t1),
        Value::Int(item),
        Value::Int(pending.vol),
    ];
    f.ev("throw", k as i64, data);
}

/// `crowd_tick(k, k16)` (4588 to 4609), from `run()` 6081 to 6085: a hush that is over ends in its volley (event
/// `subtone`); on its grid a crowd pleaser that is free poses (event `pose`: when the pose ends and the hype) and the
/// crowd throws.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    // Hs = B.hush: what the tests below read is the hush as it stood at the start of the tick
    let hushed = f.crowd.balls[k].hush.is_some();
    if let Some(hs) = f.crowd.balls[k].hush.clone()
        && f.t + 1e-9 >= hs.t_throw
    {
        // SUBTONE: the hush ends in one huge volley
        f.crowd.balls[k].hush = None;
        // max(1, int(round(int(B.n_vol + 1e-9) * ab['n_k'])))
        let whole = (f.balls[k].n_vol + 1e-9) as i64;
        let n = (py::round_half_even(whole as f64 * param(&hs.ab, "n_k")) as i64).max(1);
        volley(f, k, n, param(&hs.ab, "mult"), param(&hs.ab, "reach_k"));
        f.ev("subtone", k as i64, vec![]);
    }
    let b = &f.balls[k];
    if hushed || b.dive.is_some() || b.strut.is_some() || stunned(f, k) || b.smashed > 0.0 {
        return;
    }
    let m = f.ab_mult(k);
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    // (k16 - int(w.get('off16', 0))) % max(1, int(round(main16 / m)))
    let every16 = (py::round_half_even(main16 as f64 / m) as i64).max(1);
    if py::imod_py(k16 - w.off16 as i64, every16) != 0 {
        return;
    }
    let pose_until = f.t + w.pose16 * f.rules.beat / 4.0;
    let c = &mut f.crowd.balls[k];
    c.pose_t0 = f.t;
    c.pose_until = pose_until;
    f.balls[k].vmul = Some(w.pose_v);
    let n_vol = f.balls[k].n_vol;
    f.ev("pose", k as i64, vec![Value::Float(pose_until), Value::Float(n_vol)]);
    volley(f, k, (n_vol + 1e-9) as i64, 1.0, 1.0);
}

/// `crowd_step(k, dt)` (4610 to 4647), from `run()` 6185 to 6186: the pose ends; the throws that are due leave the
/// stands; the throws that are down land (a hit: `throw`, a knock along the throw, and the growth once a volley;
/// event `splat`: the throw's id, where, and whether it landed a hit); the landed ones go after 0.7 s; then the
/// strut and the dive.
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let t = f.t;
    let c = &mut f.crowd.balls[k];
    if c.pose_until >= 0.0 && t >= c.pose_until {
        c.pose_until = -1.0;
        f.balls[k].vmul = None;
    }
    let is_due = |p: &Pending| p.owner == k && t + 1e-9 >= p.t_launch;
    let due: Vec<Pending> = f.crowd.pend_throws.iter().filter(|p| is_due(p)).cloned().collect();
    if !due.is_empty() {
        f.crowd.pend_throws.retain(|p| !is_due(p));
        for pending in &due {
            launch_throw(f, pending);
        }
    }
    // Python walks `self.throws`; nothing in the loop adds or removes a throw, so an index walk is the same
    for i in 0..f.crowd.throws.len() {
        let th = f.crowd.throws[i].clone();
        if th.owner != k || th.landed != 0 || t + 1e-9 < th.t1 {
            continue;
        }
        let rel = geom::sub(f.balls[1 - k].p, th.p1);
        let dist = geom::hypot(rel);
        let hit = dist < w.splash + 0.6 * r;
        let mut landed = false;
        if hit {
            landed = f.damage(k, 1 - k, th.dmg, "throw", th.p1, 0);
            if landed {
                let fly = geom::sub(th.p1, th.p0);
                // fly if dist < 1e-3 else 0.5 * fly / (np.hypot(*fly) + 1e-9) + rel / dist
                let away = if dist < 1e-3 {
                    fly
                } else {
                    let along = geom::div(geom::scale(fly, 0.5), geom::hypot(fly) + 1e-9);
                    geom::add(along, geom::div(rel, dist))
                };
                f.knock(1 - k, away, w.knock);
                if !f.crowd.cvol_landed.contains(&th.vol) {
                    f.crowd.cvol_landed.push(th.vol);
                    let b = &mut f.balls[k];
                    b.n_vol = py::py_min(w.n_max, b.n_vol + w.grow);
                    b.blade_len = py::py_min(w.reach_max, b.blade_len + w.reach_grow);
                    let c = &mut f.crowd.balls[k];
                    c.throw_dmg = py::py_min(w.dmg_max, c.throw_dmg + w.dmg_grow);
                }
            }
        }
        f.crowd.throws[i].landed = if landed { 1 } else { 2 };
        let data = vec![
            Value::Int(th.id),
            Value::Float(th.p1[0]),
            Value::Float(th.p1[1]),
            Value::Int(i64::from(landed)),
        ];
        f.ev("splat", k as i64, data);
    }
    f.crowd.throws.retain(|th| !(th.landed != 0 && t > th.t1 + 0.7));
    if f.balls[k].strut.is_some() {
        strut_step(f, k, dt);
    }
    if f.balls[k].dive.is_some() {
        dive_step(f, k, dt);
    }
}

/// `trigger`, kind `runway` (2966 to 2968): `if B.dive is None: start_runway(k, ab)` (4648 to 4664): the runway from
/// the ball through the enemy to the far ropes (at least 0.5 long), the pose ended, the ball headed down it. Event
/// `runway`: where it starts, where it ends, when it starts, when it ends.
pub fn start_runway(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    if f.balls[k].dive.is_some() {
        return;
    }
    let lim = f.rules.a - f.rules.r;
    let (b_p, o_p) = (f.balls[k].p, f.balls[1 - k].p);
    let d = geom::sub(o_p, b_p);
    let n = geom::hypot(d);
    let u = if n > 1e-6 { geom::div(d, n) } else { [1.0, 0.0] };
    // ts = [(lim - B.p[ax] * math.copysign(1, u[ax])) / abs(u[ax]) for ax in (0, 1) if abs(u[ax]) > 1e-6]
    // L = max(0.5, min(ts))
    let mut least: Option<f64> = None;
    for ax in 0..2 {
        if u[ax].abs() > 1e-6 {
            let ts = (lim - b_p[ax] * 1.0f64.copysign(u[ax])) / u[ax].abs();
            least = Some(match least {
                Some(m) => py::py_min(m, ts),
                None => ts,
            });
        }
    }
    let len = py::py_max(0.5, least.expect("a unit heading has a component"));
    // p1 = np.clip(B.p + u * L, -lim, lim)
    let p1 = geom::add(b_p, geom::scale(u, len));
    let p1 = [geom::np_clip(p1[0], -lim, lim), geom::np_clip(p1[1], -lim, lim)];
    let t1 = f.t + param(ab, "beats") * f.rules.beat;
    let (t, v) = (f.t, f.rules.v);
    f.crowd.balls[k].pose_until = -1.0;
    let b = &mut f.balls[k];
    b.vmul = None;
    b.strut = Some(Strut {
        ab: ab.clone(),
        p0: b_p,
        p1,
        t0: t,
        t1,
        next: t,
        last: -9.0,
    });
    b.v = geom::scale(u, v);
    b.boost = 0.0;
    let data = vec![
        Value::Float(b_p[0]),
        Value::Float(b_p[1]),
        Value::Float(p1[0]),
        Value::Float(p1[1]),
        Value::Float(t),
        Value::Float(t1),
    ];
    f.ev("runway", k as i64, data);
}

/// `strut_step(k, dt)` (4665 to 4691): the strut down the runway. At its time it ends where the runway ends (event
/// `strut_end`). On contact, once per `cool` seconds, it bumps the enemy (`strut`, a knock straight away; event
/// `strut_bump`: where). On every beat it honks: a ring of kind `honk` that clears no projectile.
fn strut_step(f: &mut Fight, k: usize, _dt: f64) {
    let Some(st) = f.balls[k].strut.clone() else {
        return;
    };
    let r = f.rules.r;
    let t = f.t;
    if t >= st.t1 {
        f.balls[k].p = st.p1;
        f.balls[k].strut = None;
        f.ev("strut_end", k as i64, vec![]);
        return;
    }
    let u = (t - st.t0) / py::py_max(1e-9, st.t1 - st.t0);
    // B.p = St['p0'] + (St['p1'] - St['p0']) * u
    let p = geom::add(st.p0, geom::scale(geom::sub(st.p1, st.p0), u));
    f.balls[k].p = p;
    let rel = geom::sub(f.balls[1 - k].p, p);
    let dist = geom::hypot(rel);
    if dist < 2.0 * r + 0.04 && t - st.last >= param(&st.ab, "cool") {
        // n = rel / dist if dist > 1e-6 else B.v / (np.hypot(*B.v) + 1e-9)
        let n = if dist > 1e-6 {
            geom::div(rel, dist)
        } else {
            let v = f.balls[k].v;
            geom::div(v, geom::hypot(v) + 1e-9)
        };
        // hx, hy = (B.p + n * R).tolist()
        let h = geom::add(p, geom::scale(n, r));
        if let Some(strut) = &mut f.balls[k].strut {
            strut.last = t;
        }
        if f.damage(k, 1 - k, param(&st.ab, "dmg"), "strut", h, 0) {
            f.knock(1 - k, n, param(&st.ab, "knock"));
        }
        f.ev("strut_bump", k as i64, vec![Value::Float(h[0]), Value::Float(h[1])]);
    }
    if t + 1e-9 >= st.next {
        let wave = Wave {
            owner: k,
            c: p,
            r,
            speed: st.ab.get("honk_speed").unwrap_or(8.0),
            rmax: r + param(&st.ab, "honk_r"),
            dmg: param(&st.ab, "honk_dmg"),
            knock: param(&st.ab, "honk_knock"),
            kind: "honk".to_string(),
            hit: false,
            name: st.ab.name.clone(),
            t0: t,
            slow: None,
            stun: None,
            detune: None,
            grow: false,
            clears: false,
        };
        f.waves.push(wave);
        let beat = f.rules.beat;
        if let Some(strut) = &mut f.balls[k].strut {
            strut.next += beat;
        }
    }
}

/// `trigger`, kind `subtone` (2969 to 2972): `hush`, the pending throws of `k` dropped, event `hush`: when the hush
/// ends.
pub fn start_subtone(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let t_throw = f.t + param(ab, "hush_beats") * f.rules.beat;
    f.crowd.balls[k].hush = Some(Hush {
        ab: ab.clone(),
        t_throw,
    });
    f.crowd.pend_throws.retain(|p| p.owner != k);
    f.ev("hush", k as i64, vec![Value::Float(t_throw)]);
}

/// `trigger`, kind `stagedive` (2973 to 2976): `B.strut = None`, `B.hush = None`, then `start_dive(k, ab)` (4692
/// to 4709): the leap to the stands beyond the nearest rope. Event `dive`: where the leap ends, when the surf
/// begins, when the cannonball is due.
pub fn start_dive(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    f.balls[k].strut = None;
    f.crowd.balls[k].hush = None;
    let a = f.rules.a;
    let bt = f.rules.beat;
    let p = f.balls[k].p;
    let (_, ax, sg) = nearest_rope(a, p);
    let mut out = p;
    out[ax] = sg * (a + ab.get("stand").unwrap_or(0.9));
    let t_surf = f.t + param(ab, "leap_beats") * bt;
    let t_launch = t_surf + param(ab, "surf_beats") * bt;
    f.crowd.balls[k].pose_until = -1.0;
    let t = f.t;
    let b = &mut f.balls[k];
    b.vmul = None;
    b.boost = 0.0;
    b.dive = Some(Dive {
        ab: ab.clone(),
        phase: 1,
        p0: p,
        out,
        ax,
        sg,
        t0: t,
        t_surf,
        t_launch,
        t_land: None,
        hit: false,
        s0: p,
        l0: p,
        l1: p,
    });
    let data = vec![
        Value::Float(out[0]),
        Value::Float(out[1]),
        Value::Float(t_surf),
        Value::Float(t_launch),
    ];
    f.ev("dive", k as i64, data);
}

/// `dive_step(k, dt)` (4710 to 4757): the leap over the ropes (eased), the surf along the stands toward the enemy
/// (eased), then the cannonball. At the launch, event `dive_launch`: from where, to where, when it lands. A
/// cannonball passing within 2 R + `hit_r` of the enemy hits once (`dive`; event `dive_hit`: where the enemy is).
/// Where it lands: a ring of kind `dive`, and event `dive_land`: where.
fn dive_step(f: &mut Fight, k: usize, _dt: f64) {
    let Some(dv) = f.balls[k].dive.clone() else {
        return;
    };
    let ab = &dv.ab;
    let (a, r, v_shared, t) = (f.rules.a, f.rules.r, f.rules.v, f.t);
    let lim = a - r;
    let (o_p, o_v) = (f.balls[1 - k].p, f.balls[1 - k].v);
    if dv.phase == 1 {
        // the leap over the ropes into the crowd
        let u = py::py_min(1.0, (t - dv.t0) / py::py_max(1e-9, dv.t_surf - dv.t0));
        let p = geom::add(dv.p0, geom::scale(geom::sub(dv.out, dv.p0), smooth(u)));
        f.balls[k].p = p;
        if t >= dv.t_surf
            && let Some(d) = &mut f.balls[k].dive
        {
            d.phase = 2;
            d.s0 = p;
        }
    } else if dv.phase == 2 {
        // crowd-surfing along the stands toward the enemy
        let ax = dv.ax;
        let mut goal = dv.out;
        goal[1 - ax] = geom::np_clip(o_p[1 - ax], -a + 0.4, a - 0.4);
        let u = py::py_min(1.0, (t - dv.t_surf) / py::py_max(1e-9, dv.t_launch - dv.t_surf));
        let p = geom::add(dv.s0, geom::scale(geom::sub(goal, dv.s0), smooth(u)));
        f.balls[k].p = p;
        if t >= dv.t_launch {
            let speed = param(ab, "speed") * v_shared;
            let fl_guess = geom::hypot(geom::sub(o_p, p)) / speed;
            // tgt = np.clip(O.p + O.v * fl_guess * ab.get('lead', 0.6), -lim, lim)
            let ahead = geom::scale(geom::scale(o_v, fl_guess), ab.get("lead").unwrap_or(0.6));
            let tgt = geom::add(o_p, ahead);
            let tgt = [geom::np_clip(tgt[0], -lim, lim), geom::np_clip(tgt[1], -lim, lim)];
            let t_land = t + geom::hypot(geom::sub(tgt, p)) / speed;
            if let Some(d) = &mut f.balls[k].dive {
                d.phase = 3;
                d.l0 = p;
                d.l1 = tgt;
                d.t_launch = t;
                d.t_land = Some(t_land);
            }
            let data = vec![
                Value::Float(p[0]),
                Value::Float(p[1]),
                Value::Float(tgt[0]),
                Value::Float(tgt[1]),
                Value::Float(t_land),
            ];
            f.ev("dive_launch", k as i64, data);
        }
    } else {
        // the cannonball
        let t_land = dv.t_land.expect("a launched dive has its landing time");
        let u = py::py_min(1.0, (t - dv.t_launch) / py::py_max(1e-9, t_land - dv.t_launch));
        let fly = geom::sub(dv.l1, dv.l0);
        let p = geom::add(dv.l0, geom::scale(fly, u));
        f.balls[k].p = p;
        let rel = geom::sub(o_p, p);
        let dist = geom::hypot(rel);
        if !dv.hit && dist < 2.0 * r + ab.get("hit_r").unwrap_or(0.1) {
            if let Some(d) = &mut f.balls[k].dive {
                d.hit = true;
            }
            if f.damage(k, 1 - k, param(ab, "dmg"), "dive", o_p, 0) {
                // 0.6 * fly / (np.hypot(*fly) + 1e-9) + 0.4 * rel / max(dist, 1e-6)
                let along = geom::div(geom::scale(fly, 0.6), geom::hypot(fly) + 1e-9);
                let away = geom::div(geom::scale(rel, 0.4), py::py_max(dist, 1e-6));
                f.knock(1 - k, geom::add(along, away), param(ab, "knock"));
            }
            f.ev("dive_hit", k as i64, vec![Value::Float(o_p[0]), Value::Float(o_p[1])]);
        }
        if u >= 1.0 {
            // B.v = fly / (np.hypot(*fly) + 1e-9) * self.R.V
            let b = &mut f.balls[k];
            b.v = geom::scale(geom::div(fly, geom::hypot(fly) + 1e-9), v_shared);
            b.dive = None;
            let wave = Wave {
                owner: k,
                c: p,
                r,
                speed: ab.get("ring_speed").unwrap_or(9.0),
                rmax: r + param(ab, "radius"),
                dmg: param(ab, "ring_dmg"),
                knock: param(ab, "ring_knock"),
                kind: "dive".to_string(),
                hit: false,
                name: ab.name.clone(),
                t0: t,
                slow: None,
                stun: None,
                detune: None,
                grow: false,
                clears: true,
            };
            f.waves.push(wave);
            f.ev("dive_land", k as i64, vec![Value::Float(p[0]), Value::Float(p[1])]);
        }
    }
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5898 to 5899: `CROWD` and `THROWS`,
/// when a fighter is a crowd pleaser.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) {
        &["CROWD", "THROWS"]
    } else {
        &[]
    }
}

/// One ball's row of `CROWD`: (hype, pose 0..1 or -1, hush 0 / 1, strut 0..1 or -1, dive phase 0 none / 1 leap /
/// 2 surf / 3 cannonball, the phase's start, the phase's end); all -1 for a ball `crowd_setup` never ran for.
fn crowd_row(f: &Fight, j: usize) -> [f64; 7] {
    let mut row = [-1.0; 7];
    let c = &f.crowd.balls[j];
    if !c.set_up {
        return row;
    }
    let b = &f.balls[j];
    row[0] = b.n_vol;
    if c.pose_until >= 0.0 {
        row[1] = (f.t - c.pose_t0) / py::py_max(1e-9, c.pose_until - c.pose_t0);
    }
    row[2] = if c.hush.is_some() { 1.0 } else { 0.0 };
    if let Some(st) = &b.strut {
        row[3] = (f.t - st.t0) / py::py_max(1e-9, st.t1 - st.t0);
    }
    (row[4], row[5], row[6]) = (0.0, 0.0, 0.0);
    if let Some(dv) = &b.dive {
        let (from, to) = match dv.phase {
            1 => (dv.t0, dv.t_surf),
            2 => (dv.t_surf, dv.t_launch),
            _ => (dv.t_launch, dv.t_land.expect("a launched dive has its landing time")),
        };
        (row[4], row[5], row[6]) = (dv.phase as f64, from, to);
    }
    row
}

/// This kind's rows of one frame: `snap`, `CROWD` and `THROWS` (5682 to 5704), into `out` (only the keys
/// [`record_keys`] gave). Neither key is one of the kit's array keys (`story.REC_KEYS`), so both are ragged lists in
/// a recording: `CROWD` each ball's row of seven floats ([`crowd_row`]), `THROWS` one entry per live throw (owner,
/// x0, y0, x1, y1, t0, t1, item, landed 0 flying / 1 hit / 2 miss, id).
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let rows = (0..2).map(|j| Value::List(crowd_row(f, j).into_iter().map(Value::Float).collect()));
    out.push_nested("CROWD", Value::List(rows.collect()));
    let throws = f.crowd.throws.iter().map(|th| {
        Value::List(vec![
            Value::Int(th.owner as i64),
            Value::Float(th.p0[0]),
            Value::Float(th.p0[1]),
            Value::Float(th.p1[0]),
            Value::Float(th.p1[1]),
            Value::Float(th.t0),
            Value::Float(th.t1),
            Value::Int(th.item),
            Value::Int(th.landed),
            Value::Int(th.id),
        ])
    });
    out.push_nested("THROWS", Value::List(throws.collect()));
}
