//! What BARI SAX's look and its part both derive from the recording (the part recomputes its look's state from the
//! run, ruling M3-R15): the WRECKING BALL cycles (`_cycles`, `cycle_at`, `wreck_pose`), CABLE SNAP (`snap_at`), the
//! ball's height off the canvas (`air_pose`) and the ropes it presses and that spring back when it lifts off
//! (`bari_over`, `_rope_wobbles`: the rope events of speed 3.2 its part twangs).

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{py_max, py_min, round_half_even};
use bf_core::recording::{Array, Recording};
use bf_core::roster::FighterCfg;

/// `VERT`: the crane's pendulum drawn 1.6 x taller.
pub const VERT: f64 = 1.6;
/// The ball's radius (`scene.R_BALL`).
const R: f64 = 0.6;
/// The ropes' limit (`scene.A`).
const A: f64 = 3.0;
/// The top rope's height (`scene.ROPE_Y[-1]`).
const ROPE_TOP: f64 = 1.18;
/// #39 `_rope_wobbles`: `(axis, sign)` of rope sides 0 to 3.
const SIDES: [(usize, f64); 4] = [(1, 1.0), (0, 1.0), (1, -1.0), (0, -1.0)];

/// What the derivations read of the run: the events, the timeline's moments and the recorded arrays.
pub trait Src {
    /// `EV`.
    fn events(&self) -> &[Event];
    /// `T.T_FIGHT`.
    fn t_fight(&self) -> f64;
    /// `T_KO`.
    fn t_ko(&self) -> f64;
    /// `T_HS`.
    fn t_hs(&self) -> f64;
    /// `T.S16`.
    fn s16(&self) -> f64;
    /// `sim_i(t)`.
    fn sim_i(&self, t: f64) -> usize;
    /// `RUN['P'][i, k]`.
    fn p(&self, i: usize, k: usize) -> [f64; 2];
    /// `RUN[key][i, k]` as a row of floats (`None` without the key).
    fn row(&self, key: &str, i: usize, k: usize) -> Option<Vec<f64>>;
}

/// Ball `k`'s row of one frame of a ragged per-frame key (`WRECK`, `SNAP`: `[[row of ball 0], [row of ball 1]]`).
pub fn nested_row(frame: &Value, k: usize) -> Option<Vec<f64>> {
    let Value::List(balls) = frame else {
        return None;
    };
    let Value::List(row) = balls.get(k)? else {
        return None;
    };
    row.iter()
        .map(|v| match v {
            Value::Float(x) => Some(*x),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        })
        .collect()
}

/// The recorded arrays straight from a recording (the part's view; the look reads the picture's `Run`).
pub struct RecSrc<'a> {
    /// The recording.
    pub rec: &'a Recording,
    /// `T.T_FIGHT`.
    pub t_fight: f64,
    /// `T_KO`.
    pub t_ko: f64,
    /// `T_HS`.
    pub t_hs: f64,
    /// `T.S16`.
    pub s16: f64,
}

impl RecSrc<'_> {
    /// `NSIM = len(RUN['HP'])`.
    fn nsim(&self) -> usize {
        match self.rec.arrays.get("HP") {
            Some(Array::F64 { shape, .. } | Array::I64 { shape, .. }) => shape.first().copied().unwrap_or(1),
            _ => 1,
        }
    }
}

impl Src for RecSrc<'_> {
    fn events(&self) -> &[Event] {
        &self.rec.events
    }
    fn t_fight(&self) -> f64 {
        self.t_fight
    }
    fn t_ko(&self) -> f64 {
        self.t_ko
    }
    fn t_hs(&self) -> f64 {
        self.t_hs
    }
    fn s16(&self) -> f64 {
        self.s16
    }
    fn sim_i(&self, t: f64) -> usize {
        // int(min(NSIM - 1, max(0, round((t - T.T_FIGHT) * FPS))))
        let n = self.nsim() as f64;
        py_min(n - 1.0, py_max(0.0, round_half_even((t - self.t_fight) * 30.0))) as usize
    }
    fn p(&self, i: usize, k: usize) -> [f64; 2] {
        let i = i.min(self.nsim() - 1);
        match self.rec.arrays.get("P") {
            Some(Array::F64 { data, .. }) => [data[(i * 2 + k) * 2], data[(i * 2 + k) * 2 + 1]],
            _ => [0.0, 0.0],
        }
    }
    fn row(&self, key: &str, i: usize, k: usize) -> Option<Vec<f64>> {
        if let Some(Array::Nested(Value::List(frames))) = self.rec.arrays.get(key) {
            return nested_row(frames.get(i)?, k);
        }
        let (shape, len) = match self.rec.arrays.get(key)? {
            Array::F64 { shape, data } => (shape, data.len()),
            Array::I64 { shape, data } => (shape, data.len()),
            Array::Nested(_) => return None,
        };
        if shape.len() < 2 || shape[1] != 2 || k >= 2 || i >= shape[0] {
            return None;
        }
        let per: usize = shape[1..].iter().product::<usize>() / 2;
        let at = (i * 2 + k) * per;
        if at + per > len {
            return None;
        }
        match self.rec.arrays.get(key)? {
            Array::F64 { data, .. } => Some(data[at..at + per].to_vec()),
            Array::I64 { data, .. } => Some(data[at..at + per].iter().map(|&v| v as f64).collect()),
            Array::Nested(_) => None,
        }
    }
}

/// The swing of a cycle as `_cycles` fixes it (from its `swing` event and the `WRECK` record).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwingPose {
    pub ps: [f64; 2],
    pub bot: [f64; 2],
    pub u: [f64; 2],
    pub l: f64,
    pub th: f64,
    pub ts0: f64,
    pub ts1: f64,
    pub tl0: f64,
    pub p0: [f64; 2],
}

/// One WRECKING BALL cycle (`_cycles`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cycle {
    pub t0: f64,
    pub t_hook: f64,
    pub cut: f64,
    pub end: f64,
    pub sw: Option<SwingPose>,
}

/// A CABLE SNAP, from its `snap_aim` event: `(t, k, spot, ts, t1, r)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snap {
    pub t0: f64,
    pub spot: [f64; 2],
    pub ts: f64,
    pub t1: f64,
    pub rad: f64,
}

/// BARI SAX's cycles and snaps, from the run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wreckage {
    /// Its side.
    pub k: usize,
    /// Its weapon is a wrecking ball.
    pub wrecker: bool,
    /// It has a `snap` ability.
    pub snapper: bool,
    /// `CYCLES`.
    pub cycles: Vec<Cycle>,
    /// `SNAPS` of its own.
    pub snaps: Vec<Snap>,
}

impl Wreckage {
    /// The derivations for side `k` with the roster entry `cfg` (its own, or a copied one's).
    pub fn new(src: &dyn Src, k: usize, cfg: &FighterCfg) -> Wreckage {
        let w = cfg.raw.get("weapon");
        let wrecker = w.and_then(|w| w.get("kind")).and_then(|v| v.as_str()) == Some("wrecking");
        let snapper = [&cfg.unlock1, &cfg.unlock2, &cfg.low]
            .iter()
            .any(|a| a.kind.name() == "snap");
        let lift16 = w.and_then(|w| w.get("lift16")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let snaps = src
            .events()
            .iter()
            .filter(|e| e.kind == "snap_aim" && e.who() == Some(k))
            .map(|e| Snap {
                t0: e.t,
                spot: [e.f(3), e.f(4)],
                ts: e.f(5),
                t1: e.f(6),
                rad: e.f(7),
            })
            .collect();
        Wreckage {
            k,
            wrecker,
            snapper,
            cycles: cycles(src, k, lift16, wrecker),
            snaps,
        }
    }

    /// `cycle_at(t, tail)`: the cycle on screen at `t` (from its cable drop until the block is back up).
    pub fn cycle_at(&self, t: f64, tail: f64) -> Option<&Cycle> {
        self.cycles.iter().find(|c| c.t0 <= t && t < c.end + tail)
    }

    /// `snap_at(t)`: the CABLE SNAP on at `t`, hoisting (`true`) or falling, and its progress.
    pub fn snap_at(&self, t_ko: f64, t: f64) -> Option<(&Snap, bool, f64)> {
        for q in &self.snaps {
            if q.t0 <= t && t < q.t1 && q.t0 <= t_ko {
                if t < q.ts {
                    return Some((q, true, (t - q.t0) / py_max(1e-6, q.ts - q.t0)));
                }
                return Some((q, false, (t - q.ts) / py_max(1e-6, q.t1 - q.ts)));
            }
        }
        None
    }

    /// `air_pose(t)`: `(p, h)` of the bari off the canvas at `t`: the swing or the hoist, CABLE SNAP, the drop on the
    /// ropes.
    pub fn air_pose(&self, src: &dyn Src, t: f64) -> Option<([f64; 2], f64)> {
        let k = self.k;
        if !self.wrecker && !self.snapper {
            return None;
        }
        if self.snap_at(src.t_ko(), t).is_some() {
            let i = src.sim_i(t);
            return Some((src.p(i, k), src.row("SNAP", i, k).map_or(0.0, |r| r[1])));
        }
        if self.wrecker {
            if let Some((q, h, _)) = wreck_pose(self.cycle_at(t, 0.0), t) {
                return Some((q, h));
            }
            let i = src.sim_i(t);
            if src.t_fight() <= t
                && t < src.t_ko()
                && let Some(row) = src.row("WRECK", i, k)
                && row[0] as i64 == 4
            {
                return Some((src.p(i, k), VERT * row[1]));
            }
        }
        None
    }

    /// `bari_over(t)` as the part sees it: the centre and the height of the bari while it is up (above 0.02) and its
    /// footprint reaches the rope line, from its own air pose (the picture state's `p` and `lift` of a ball no other
    /// look changes).
    pub fn over(&self, src: &dyn Src, t: f64) -> Option<([f64; 2], f64)> {
        if !self.wrecker || !(src.t_fight() <= t && t < src.t_hs()) {
            return None;
        }
        let (p, h) = self.air_pose(src, py_min(t, src.t_ko() - 1e-4))?;
        if h > 0.02 && py_max(p[0].abs(), p[1].abs()) > A - R * 1.05 {
            return Some((p, h));
        }
        None
    }
}

/// `_rope_wobbles()`: when the bari lifts off a rope it pressed, the rope springs back: a rope event there, `(t,
/// side, x, z, 3.2)`, frame by frame from FIGHT! to the K.O. `over(t)` is `bari_over`.
pub fn wobbles(t_fight: f64, t_ko: f64, over: impl Fn(f64) -> Option<([f64; 2], f64)>) -> Vec<(f64, usize, f64, f64)> {
    let mut out = Vec::new();
    let mut prev: [Option<[f64; 2]>; 4] = [None; 4];
    // f0, f1 = int(T.T_FIGHT * FPS), int(T_KO * FPS)
    let (f0, f1) = ((t_fight * 30.0) as i64, (t_ko * 30.0) as i64);
    for f in f0..=f1 {
        let t = f as f64 / 30.0;
        let mut cur: [Option<[f64; 2]>; 4] = [None; 4];
        if let Some((c, hb)) = over(t) {
            for (s, &(ax, sg)) in SIDES.iter().enumerate() {
                if c[ax] * sg > A - R * 1.05 && hb < ROPE_TOP {
                    cur[s] = Some(c);
                }
            }
        }
        for s in 0..4 {
            if let (Some(c), None) = (prev[s], cur[s]) {
                out.push((t, s, c[0], c[1]));
            }
        }
        prev = cur;
    }
    out
}

/// `_cycles()`: every cycle: the cable drop, the hook, the hoist, the swing, cut short where CABLE SNAP takes over.
fn cycles(src: &dyn Src, k: usize, lift16: f64, wrecker: bool) -> Vec<Cycle> {
    let ev = src.events();
    let t_ko = src.t_ko();
    let lift = if wrecker { lift16 * src.s16() } else { 0.0 };
    let mut out = Vec::new();
    for e in ev {
        if e.kind != "cable" || e.t > t_ko || e.who() != Some(k) {
            continue;
        }
        let (t0, t_hook) = (e.t, e.f(3));
        let sw = ev
            .iter()
            .find(|q| q.kind == "swing" && q.who() == Some(k) && t_hook - 1e-6 <= q.t && q.t <= t_hook + lift + 0.05);
        let cut = ev
            .iter()
            .filter(|q| (q.kind == "cable" || q.kind == "snap_aim") && q.who() == Some(k) && q.t > t0)
            .map(|q| q.t)
            .fold(t_ko + 1e-3, py_min);
        let mut c = Cycle {
            t0,
            t_hook,
            cut,
            end: py_min(t_hook, cut),
            sw: None,
        };
        if let Some(q) = sw
            && q.t < cut
        {
            let (ps, p1, bot) = ([q.f(3), q.f(4)], [q.f(5), q.f(6)], [q.f(7), q.f(8)]);
            let d = [p1[0] - ps[0], p1[1] - ps[1]];
            let n = (d[0] * d[0] + d[1] * d[1]).sqrt() + 1e-9;
            let u = [d[0] / n, d[1] / n];
            let i = src.sim_i(q.t - 0.5 / 30.0);
            let row = src.row("WRECK", i, k).unwrap_or_default();
            let p0 = if row.len() > 6 && (row[0] == 2.0 || row[0] == 3.0) {
                [row[5], row[6]]
            } else {
                ps
            };
            let (ts0, ts1) = (q.f(11), q.f(12));
            c.sw = Some(SwingPose {
                ps,
                bot,
                u,
                l: q.f(9),
                th: q.f(10),
                ts0,
                ts1,
                tl0: ts0 - lift,
                p0,
            });
            c.end = py_min(ts1, cut);
        }
        out.push(c);
    }
    out
}

/// `wreck_pose(c, t)`: `(p, h, swinging)` of the ball in cycle `c` at `t`: hoisted back, or the pendulum.
pub fn wreck_pose(c: Option<&Cycle>, t: f64) -> Option<([f64; 2], f64, bool)> {
    let c = c?;
    if t < c.t0 || t >= c.end {
        return None;
    }
    let sw = c.sw?;
    if t < sw.tl0 {
        return None;
    }
    let pi = std::f64::consts::PI;
    if t < sw.ts0 {
        let uu = (t - sw.tl0) / py_max(1e-6, sw.ts0 - sw.tl0);
        let e = uu * uu * (3.0 - 2.0 * uu);
        let p = [
            sw.p0[0] + (sw.ps[0] - sw.p0[0]) * e,
            sw.p0[1] + (sw.ps[1] - sw.p0[1]) * e,
        ];
        return Some((p, VERT * sw.l * (1.0 - dmath::cos(sw.th)) * e, false));
    }
    let uu = py_min(1.0, (t - sw.ts0) / py_max(1e-6, sw.ts1 - sw.ts0));
    let phi = -sw.th * dmath::cos(pi * uu);
    let s = sw.l * dmath::sin(phi);
    let p = [sw.bot[0] + sw.u[0] * s, sw.bot[1] + sw.u[1] * s];
    Some((p, VERT * sw.l * (1.0 - dmath::cos(phi)), true))
}
