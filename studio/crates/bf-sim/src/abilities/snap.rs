//! Ability kind `snap` (BARI SAX's CABLE SNAP): `fight.py` `trigger` 2919 to 2920; 3787 to 3856 (`start_snap`,
//! `snap_step`).
//!
//! The crane hoists the ball (out of a swing, if one is on: the cable lets go of that) up to `height` and carries it
//! over the spot where the enemy will be in `tele16` 16ths (where a hop lands; round its whirl if it is anchored; else
//! `lead` x its motion; fixed then) for the first `hoist` of that time; then the cable snaps and the ball falls
//! straight down (free fall) and crashes there: an enemy within `radius` beyond contact takes `dmg` and `knock` away
//! from the crater (unless it sails over it: up in a hop past `air_edge`). Then it rolls on at the enemy. Events
//! `snap_aim`, `cable_snap`, `crater`; recorded as `SNAP`.
//!
//! Python hangs `wsnap` on the ball; here it is [`State`], `Fight::snap`, indexed by fighter. A copied ability
//! (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here assumes which side has the kind. None of
//! this code draws from the generator.

use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::kinds::{mortar, wrecking};
use crate::record::Frames;
use crate::roster::{AbilityCfg, AbilityKind};

/// A CABLE SNAP under way (`B.wsnap`, the dict `start_snap` makes; `ab`'s numbers are copied in).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wsnap {
    /// Where it was hoisted from.
    pub p0: V2,
    /// The spot it falls on.
    pub tgt: V2,
    /// The start.
    pub t0: f64,
    /// The snap.
    pub ts: f64,
    /// The crash.
    pub t1: f64,
    /// The height it was hoisted from.
    pub h0: f64,
    /// Its height now.
    pub h: f64,
    /// `ab['height']`.
    pub height: f64,
    /// `ab['radius']`.
    pub radius: f64,
    /// `ab['dmg']`.
    pub dmg: f64,
    /// `ab['knock']`.
    pub knock: f64,
    /// `ab.get('air_edge', B.w.get('air_edge', 0.0))`.
    pub air_edge: f64,
    /// Whether the cable has snapped.
    pub snapped: bool,
}

/// This ability's per-fight state (`Fight::snap`): each fighter's `B.wsnap`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.wsnap`, by fighter.
    pub wsnap: [Option<Wsnap>; 2],
}

/// `trigger`, kind `snap` (2919 to 2920): `start_snap(k, ab)` (3787 to 3806). Event `snap_aim`.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let s16 = f.rules.beat / 4.0;
    let t_ = param(ab, "tele16") * s16;
    // h0 = W_.get('h', 0.0) if W_ is not None else 0.0
    let h0 = f.wrecking.wreck[k].and_then(|w| w.h).unwrap_or(0.0);
    f.wrecking.wreck[k] = None;
    let lead = ab.get("lead").unwrap_or(1.0);
    let o = 1 - k;
    let tgt = if let Some(h) = f.tiptoe.hop[o] {
        h.p1
    } else if f.balls[o].anc.is_some() {
        // #40: a whirling (anchored) enemy: lead it round its whirl
        mortar::predict(f, o, t_ * lead)
    } else {
        // O.p + O.v * T_ * lead
        let (op, ov) = (f.balls[o].p, f.balls[o].v);
        [op[0] + ov[0] * t_ * lead, op[1] + ov[1] * t_ * lead]
    };
    let lim = f.rules.a - f.rules.r;
    let tgt = [geom::np_clip(tgt[0], -lim, lim), geom::np_clip(tgt[1], -lim, lim)];
    let ts = f.t + t_ * ab.get("hoist").unwrap_or(0.6);
    let air_edge = ab.get("air_edge").unwrap_or_else(|| wrecking::air_edge(f, k));
    let radius = param(ab, "radius");
    f.snap.wsnap[k] = Some(Wsnap {
        p0: f.balls[k].p,
        tgt,
        t0: f.t,
        ts,
        t1: f.t + t_,
        h0,
        h: h0,
        height: param(ab, "height"),
        radius,
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        air_edge,
        snapped: false,
    });
    f.balls[k].air = true;
    let data = vec![
        Value::Float(tgt[0]),
        Value::Float(tgt[1]),
        Value::Float(ts),
        Value::Float(f.t + t_),
        Value::Float(radius),
    ];
    f.ev("snap_aim", k as i64, data);
}

/// `snap_step(k, dt)` (3807 to 3856), from `run()` 6169 to 6170: hoisted up and over the spot, then the cable snaps
/// (`cable_snap`), the ball falls and crashes (`crater`).
pub fn snap_step(f: &mut Fight, k: usize, _dt: f64) {
    let Some(mut sn) = f.snap.wsnap[k] else {
        return;
    };
    if f.t < sn.ts {
        // hoisted up and over the spot
        let uu = (f.t - sn.t0) / py::py_max(1e-6, sn.ts - sn.t0);
        let e = uu * uu * (3.0 - 2.0 * uu);
        let d = geom::sub(sn.tgt, sn.p0);
        f.balls[k].p = geom::add(sn.p0, geom::scale(d, e));
        sn.h = sn.h0 + (sn.height - sn.h0) * e;
        let n = geom::hypot(d);
        if n > 1e-6 {
            f.balls[k].v = geom::scale(geom::div(d, n), 0.3);
        }
        f.snap.wsnap[k] = Some(sn);
        return;
    }
    if !sn.snapped {
        sn.snapped = true;
        f.snap.wsnap[k] = Some(sn);
        f.ev("cable_snap", k as i64, vec![]);
    }
    let ss = py::py_min(1.0, (f.t - sn.ts) / py::py_max(1e-6, sn.t1 - sn.ts));
    // free fall
    sn.h = sn.height * (1.0 - ss * ss);
    f.balls[k].p = sn.tgt;
    f.snap.wsnap[k] = Some(sn);
    if ss < 1.0 {
        return;
    }
    let r = f.rules.r;
    let o = 1 - k;
    let rel = geom::sub(f.balls[o].p, f.balls[k].p);
    let dist = geom::hypot(rel);
    let n = if dist > 1e-6 { geom::div(rel, dist) } else { [1.0, 0.0] };
    let hit = dist - 2.0 * r <= sn.radius && !wrecking::sails_over(f, o, sn.air_edge);
    if hit {
        let at = geom::sub(f.balls[o].p, geom::scale(n, r));
        f.damage(k, o, sn.dmg, "crash", at, 0);
        f.knock(o, n, sn.knock);
    }
    let p = f.balls[k].p;
    let data = vec![
        Value::Float(p[0]),
        Value::Float(p[1]),
        Value::Float(sn.radius),
        Value::Int(i64::from(hit)),
    ];
    f.ev("crater", k as i64, data);
    f.balls[k].v = geom::scale(n, f.rules.v);
    f.balls[k].air = false;
    f.snap.wsnap[k] = None;
}

/// The record keys this ability adds (Python's `rec` gets them at setup): `run()` 5849 to 5850: `SNAP`, when a fighter
/// has a `snap` ability.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| crate::abilities::has(f, k, &AbilityKind::Snap)) {
        &["SNAP"]
    } else {
        &[]
    }
}

/// This ability's rows of one frame: `snap`, `SNAP` (5588 to 5594): per ball, while a CABLE SNAP is under way: 1, its
/// height, the spot, the start, the snap, the crash and its radius (zeros otherwise).
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut row = [0.0; 16];
    for (j, sn) in f.snap.wsnap.iter().enumerate() {
        if let Some(s) = sn {
            row[8 * j..8 * j + 8].copy_from_slice(&[1.0, s.h, s.tgt[0], s.tgt[1], s.t0, s.ts, s.t1, s.radius]);
        }
    }
    // not one of the kit's array keys (`story.REC_KEYS`): a ragged list in a recording, each ball's row of eight floats
    let rows = row
        .chunks(8)
        .map(|r| Value::List(r.iter().copied().map(Value::Float).collect()));
    out.push_nested("SNAP", Value::List(rows.collect()));
}
