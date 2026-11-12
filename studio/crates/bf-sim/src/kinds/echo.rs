//! Weapon kind `echo`, HORN's the echo call: `fight.py` 2196 to 2281 (`echo_setup`, `echo_pos`, `_echo_blast`,
//! `echo_tick`, `echo_step`).
//!
//! On its grid (`every` beats) the ball sounds a round blast around itself: a ring from its surface out to `blade_len`
//! beyond it (`base`, + `reach_grow` per landed blast, up to `max_reach`). Up to `n_max` echo ghosts trail behind
//! along the ball's own past path, ghost j where the ball was j spacings ago (the spacing eases toward a beat over the
//! haste: HORN RIP's half-beat echoes), and ghost j repeats each call j spacings after it (`max(1, int(round(4 /
//! ab_mult)))` 16ths), from where it is then: a delay in space. Each repeat deals `decay` x the one before (damage
//! and knockback) and reaches `echo_reach` x as far. Every landed blast (the call or an echo) adds `grow` ghosts.
//! A ring hits once, when it reaches the enemy's near surface. Events `echo` (ghost j, the call's id, where, the
//! ring's full radius), `echo_call` (the id, the ghosts, the spacing in 16ths), `echo_grow`; hits of kind `echo` with
//! the ghost as their sub-number. Recorded as `ECHO_G`, `ECHO_N` and `ECHO_R`.
//!
//! Python hangs `n_echo`, `echo_hist`, `echo_sp`, `echo_due` and `echo_calls` on the ball and keeps the live rings
//! in the fight's `echoes`; here they are [`State`], `Fight::echo`, indexed by fighter (`Ball::blade_len` is the
//! ball's). None of this code draws from the generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{EchoCfg, WeaponCfg};

/// `Fight.EMAX`: the ghost slots `ECHO_G` holds.
pub const EMAX: usize = bf_core::roster::kinds::echo::EMAX;

/// A live blast ring (one entry of Python's `self.echoes`).
#[derive(Clone, Debug, PartialEq)]
pub struct Ring {
    /// The fighter that sounded it.
    pub owner: usize,
    /// Its centre.
    pub c: V2,
    /// Its radius now.
    pub r: f64,
    /// Its full radius.
    pub rmax: f64,
    /// Units a second it spreads.
    pub speed: f64,
    /// Its damage.
    pub dmg: f64,
    /// Its knockback.
    pub knock: f64,
    /// 0 the call, j the j-th ghost's repeat.
    pub j: i64,
    /// The call it sounds or repeats.
    pub call: i64,
    /// When it was sounded.
    pub t0: f64,
    /// When it is gone.
    pub t_end: f64,
    /// Whether it has reached the enemy.
    pub hit: bool,
}

/// What `echo_setup` hangs on one ball.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ghosts {
    /// Whether `echo_setup` ran for the ball (Python's `hasattr(B, 'echo_hist')`).
    pub set_up: bool,
    /// `B.n_echo`: the ghosts earned (a float; its whole part counts).
    pub n_echo: f64,
    /// `B.echo_hist`: the ball's position every substep, newest last, trimmed.
    pub hist: Vec<V2>,
    /// `B.echo_sp`: the ghosts' spacing along the path, in seconds.
    pub sp: f64,
    /// `B.echo_due`: the pending repeats (16th index, ghost j, call id), in the order they were added.
    pub due: Vec<(i64, i64, i64)>,
    /// `B.echo_calls`: the calls sounded.
    pub calls: i64,
}

/// Weapon kind `echo`'s per-fight state (`Fight::echo`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// Each fighter's ghosts.
    pub balls: [Ghosts; 2],
    /// `self.echoes`: the live rings of both fighters, in the order they were sounded.
    pub rings: Vec<Ring>,
}

/// `B.w['kind'] == 'echo'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Echo(_))
}

/// The echo's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not an echo.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a EchoCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Echo(w) => w,
        other => panic!("fighter {k} ({}) has no echo weapon: {other:?}", cfg.name),
    }
}

/// `echo_setup(k)` (2205 to 2214), from `run()` 5787 to 5789
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let p = f.balls[k].p;
    f.echo.balls[k] = Ghosts {
        set_up: true,
        n_echo: w.ghosts0(),
        hist: vec![p],
        sp: f.rules.beat,
        due: Vec::new(),
        calls: 0,
    };
}

/// `echo_pos(B, lag)` (2215 to 2223): where fighter `k` was `lag` s ago (the history holds one sample per substep;
/// before the fight: its start).
pub fn echo_pos(f: &Fight, k: usize, lag: f64) -> V2 {
    let h = &f.echo.balls[k].hist;
    let m = py::py_max(0.0, lag) / f.rules.dt;
    let i = m as usize; // int(m): m is never negative
    if i + 1 >= h.len() {
        return h[0];
    }
    let (a, b) = (h[h.len() - 1 - i], h[h.len() - 2 - i]);
    let x = m - i as f64;
    // a + (b - a) * (m - i)
    [a[0] + (b[0] - a[0]) * x, a[1] + (b[1] - a[1]) * x]
}

/// `_echo_blast(k, c, j, call)` (2225 to 2234): a ring sounds at `c`. Event `echo`.
fn blast(f: &mut Fight, k: usize, c: V2, j: i64, call: i64) {
    let w = weapon(f, k);
    // w.get('decay', 0.8) ** j: a float to an int power
    let fj = dmath::pow(w.decay, j as f64);
    let rr = f.rules.r;
    let r0 = if j == 0 { rr } else { rr * 0.5 };
    let rmax = rr + f.balls[k].blade_len * (if j != 0 { w.echo_reach } else { 1.0 });
    let ring = Ring {
        owner: k,
        c,
        r: r0,
        rmax,
        speed: w.speed,
        dmg: w.dmg * fj,
        knock: w.knock * fj,
        j,
        call,
        t0: f.t,
        t_end: f.t + (rmax - r0) / w.speed + w.linger,
        hit: false,
    };
    f.echo.rings.push(ring);
    let data = vec![
        Value::Int(j),
        Value::Int(call),
        Value::Float(c[0]),
        Value::Float(c[1]),
        Value::Float(rmax),
    ];
    f.ev("echo", k as i64, data);
}

/// `echo_tick(k, k16)` (2236 to 2252), from `run()` 6009 to 6014 (a stunned fighter is skipped by the caller): due
/// repeats fire from their ghosts, and the call sounds on the weapon's grid. Event `echo_call`.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let due: Vec<(i64, i64, i64)> = f.echo.balls[k].due.iter().copied().filter(|d| d.0 <= k16).collect();
    if !due.is_empty() {
        f.echo.balls[k].due.retain(|d| d.0 > k16);
        for (_, j, call) in due {
            let lag = j as f64 * f.echo.balls[k].sp;
            let c = echo_pos(f, k, lag);
            blast(f, k, c, j, call);
        }
    }
    let w = weapon(f, k);
    let every16 = (py::round_half_even(4.0 * w.every) as i64).max(1);
    if py::imod_py(k16, every16) == 0 {
        f.echo.balls[k].calls += 1;
        let calls = f.echo.balls[k].calls;
        let p = f.balls[k].p;
        blast(f, k, p, 0, calls);
        let n = (f.echo.balls[k].n_echo + 1e-9) as i64;
        // haste: the echoes come back faster
        let sp16 = (py::round_half_even(4.0 / f.ab_mult(k)) as i64).max(1);
        for j in 1..=n {
            f.echo.balls[k].due.push((k16 + j * sp16, j, calls));
        }
        let data = vec![Value::Int(calls), Value::Int(n), Value::Int(sp16)];
        f.ev("echo_call", k as i64, data);
    }
}

/// `echo_step(k, dt)` (2253 to 2281), from `run()` 6146 to 6147: the path history, the spacing, and fighter `k`'s
/// rings spreading; a ring that reaches the enemy's near surface hits (event `hit` of kind `echo`, the knock, and on a
/// landed hit the ghosts and the reach grow: event `echo_grow` when a ghost is earned).
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let p = f.balls[k].p;
    // L = int(self.EMAX * self.R.beat * 1.1 / self.R.DT) + 4
    let lmax = (EMAX as f64 * f.rules.beat * 1.1 / f.rules.dt) as usize + 4;
    {
        let g = &mut f.echo.balls[k];
        g.hist.push(p);
        if g.hist.len() > 2 * lmax {
            g.hist.drain(..g.hist.len() - lmax);
        }
    }
    let tgt = f.rules.beat / f.ab_mult(k);
    {
        let g = &mut f.echo.balls[k];
        g.sp += (tgt - g.sp) * py::py_min(1.0, dt / w.ease);
    }
    // `for E in [E for E in self.echoes if E['owner'] == k]`: no ring is added or removed in the loop
    for idx in 0..f.echo.rings.len() {
        let e = &mut f.echo.rings[idx];
        if e.owner != k || e.r >= e.rmax {
            continue;
        }
        e.r = py::py_min(e.rmax, e.r + e.speed * dt);
        let (c, er, hit, dmg, knock, j) = (e.c, e.r, e.hit, e.dmg, e.knock, e.j);
        let (o_p, o_v) = (f.balls[1 - k].p, f.balls[1 - k].v);
        let rel = geom::sub(o_p, c);
        let dist = geom::hypot(rel);
        if !hit && er >= dist - r {
            // the ring reaches the enemy's near surface
            f.echo.rings[idx].hit = true;
            let u = if dist > 1e-6 {
                geom::div(rel, dist)
            } else {
                // O.v / (np.linalg.norm(O.v) + 1e-12)
                geom::div(o_v, geom::norm(o_v) + 1e-12)
            };
            // hx, hy = (E['c'] + u * max(0.0, dist - R)).tolist()
            let at = geom::add(c, geom::scale(u, py::py_max(0.0, dist - r)));
            let n0 = (f.echo.balls[k].n_echo + 1e-9) as i64;
            let landed = f.damage(k, 1 - k, dmg, "echo", at, j);
            f.knock(1 - k, u, knock);
            if landed {
                let g = &mut f.echo.balls[k];
                g.n_echo = py::py_min(w.n_max, g.n_echo + w.grow);
                let b = &mut f.balls[k];
                // min(w.get('max_reach', B.blade_len), B.blade_len + w.get('reach_grow', 0.0))
                b.blade_len = py::py_min(w.max_reach.unwrap_or(b.blade_len), b.blade_len + w.reach_grow);
                let n = (f.echo.balls[k].n_echo + 1e-9) as i64;
                if n > n0 {
                    f.ev("echo_grow", k as i64, vec![Value::Int(n)]);
                }
            }
        }
    }
    let t = f.t;
    f.echo.rings.retain(|e| e.owner != k || t < e.t_end);
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5791 to 5792: `ECHO_G`, `ECHO_N`,
/// `ECHO_R`, when a fighter's weapon is an echo.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) {
        &["ECHO_G", "ECHO_N", "ECHO_R"]
    } else {
        &[]
    }
}

/// This kind's rows of one frame: `snap`, `ECHO_G`, `ECHO_N`, `ECHO_R` (5539 to 5549), into `out` (only the keys
/// [`record_keys`] gave). `ECHO_G`: per ball, every ghost slot's place on its past path and its lag (NaN for a ball
/// with no echo); `ECHO_N`: the ghosts earned; `ECHO_R`, ragged: every live ring (owner, x, y, r, rmax, j, t0, call,
/// hit 0 / 1).
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut g = [f64::NAN; 2 * EMAX * 3];
    for j in 0..2 {
        if !f.echo.balls[j].set_up {
            continue;
        }
        let sp = f.echo.balls[j].sp;
        for i in 0..EMAX {
            let lag = (i + 1) as f64 * sp;
            let q = echo_pos(f, j, lag);
            let at = (j * EMAX + i) * 3;
            g[at] = q[0];
            g[at + 1] = q[1];
            g[at + 2] = lag;
        }
    }
    out.push_f64("ECHO_G", &[2, EMAX, 3], &g);
    // getattr(B, 'n_echo', 0.0)
    let n = [0, 1].map(|j| if f.echo.balls[j].set_up { f.echo.balls[j].n_echo } else { 0.0 });
    out.push_f64("ECHO_N", &[2], &n);
    let rings = f.echo.rings.iter().map(|e| {
        Value::List(vec![
            Value::Int(e.owner as i64),
            Value::Float(e.c[0]),
            Value::Float(e.c[1]),
            Value::Float(e.r),
            Value::Float(e.rmax),
            Value::Int(e.j),
            Value::Float(e.t0),
            Value::Int(e.call),
            Value::Int(i64::from(e.hit)),
        ])
    });
    out.push_nested("ECHO_R", Value::List(rings.collect()));
}
