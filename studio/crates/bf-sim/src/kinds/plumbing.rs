//! Weapon kind `plumbing`, CONTRABASSOON's U-BEND, and its three abilities (WATER KEY, HOT PIPES, FIVE AND A HALF
//! METRES). Made from scratch (Nov 16, 2026, `docs/studio/2026-11-16-contrabassoon.md`): Rust is the source.
//!
//! Every `every` beats the fighter lays a U-shaped pipe ahead of the enemy, its bend toward the enemy. Both balls
//! bounce off pipes. An enemy bounce hurts and boosts the enemy; the owner's own bounce boosts the owner. The low
//! move closes a square of tube round the enemy that only the enemy bounces off. None of this draws from the
//! generator.
//!
//! Events: `pipe` (owner; id, then the four points x, y of the polyline), `pipe_off` (owner; id), `pipe_box` (owner;
//! id, centre x, y, half-side), `pipe_box_off` (owner; id), `hot_pipes` (owner; seconds), `spray` (owner; id, then
//! the x, y of every pipe end that sprays). Hits are `pipe`, `spray` and `pipe_box`.

use bf_core::event::Value;
use bf_core::roster::AbilityCfg;

use crate::abilities::{ab_dur, param};
use crate::fight::{Fight, radius, rewinding};
use crate::geom::{self, V2};
use crate::roster::{PlumbingCfg, WeaponCfg};

/// A pipe in the ring: a U (four points) or, for the low move, a closed square (five points).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pipe {
    /// The fighter whose pipe it is.
    pub owner: usize,
    /// Its id (counted per fight).
    pub id: i64,
    /// The polyline's points.
    pub pts: Vec<V2>,
    /// Its centre (where the next pipe keeps away from).
    pub c: V2,
    /// Whether it is the low move's box (only the enemy bounces off it; it ends at `until`).
    pub boxed: bool,
    /// When a box ends.
    pub until: f64,
    /// When it last hurt the enemy.
    pub last_hit: f64,
    /// A box's damage a wall hit.
    pub box_dmg: f64,
    /// A box's seconds between hits.
    pub box_cool: f64,
}

/// A WATER KEY spray in progress.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spray {
    pub owner: usize,
    pub id: i64,
    /// Where it sprays from: the pipe ends when it started (the owner's own ball is added each step).
    pub from: Vec<V2>,
    pub until: f64,
    pub reach: f64,
    pub dmg: f64,
    pub knock: f64,
}

/// This kind's per-fight state (`Fight::plumbing`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    pub pipes: Vec<Pipe>,
    pub sprays: Vec<Spray>,
    /// The next pipe, box or spray id.
    pub next_id: i64,
    /// Each fighter's bounce damage now (grows with every landed bounce).
    pub dmg: [f64; 2],
    /// Until when each fighter's HOT PIPES lasts, and its factor.
    pub hot_until: [f64; 2],
    pub hot_mult: [f64; 2],
}

/// Fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Plumbing(_))
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a PlumbingCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Plumbing(w) => w,
        other => panic!("fighter {k} ({}) has no plumbing weapon: {other:?}", cfg.name),
    }
}

/// Whether anything of this kind is in the ring.
pub fn live(f: &Fight) -> bool {
    !f.plumbing.pipes.is_empty() || !f.plumbing.sprays.is_empty()
}

/// The point of segment `a`-`b` nearest `p`.
fn nearest_on(a: V2, b: V2, p: V2) -> V2 {
    let ab = geom::sub(b, a);
    let l2 = geom::dot(ab, ab);
    if l2 < 1e-18 {
        return a;
    }
    let s = (geom::dot(geom::sub(p, a), ab) / l2).clamp(0.0, 1.0);
    geom::add(a, geom::scale(ab, s))
}

/// The point of polyline `pts` nearest `p`, and its distance.
fn nearest(pts: &[V2], p: V2) -> (V2, f64) {
    let mut best = (pts[0], f64::INFINITY);
    for w in pts.windows(2) {
        let q = nearest_on(w[0], w[1], p);
        let d = geom::hypot(geom::sub(p, q));
        if d < best.1 {
            best = (q, d);
        }
    }
    best
}

/// The U's four points: legs from the bend away along `u`, `gap` apart across `t`.
fn u_points(c: V2, u: V2, w: &PlumbingCfg) -> Vec<V2> {
    let t = [-u[1], u[0]];
    let b1 = geom::sub(c, geom::scale(t, w.gap / 2.0));
    let b2 = geom::add(c, geom::scale(t, w.gap / 2.0));
    vec![
        geom::add(b1, geom::scale(u, w.leg)),
        b1,
        b2,
        geom::add(b2, geom::scale(u, w.leg)),
    ]
}

/// On beat `kb`, every `max(1, round(every / ab_mult))` beats, fighter `k` lays a pipe.
pub fn beat(f: &mut Fight, k: usize, kb: i64) {
    let w = weapon(f, k);
    let n = (w.every / f.ab_mult(k)).round() as i64;
    if kb % n.max(1) == 0 {
        lay_pipe(f, k);
    }
}

/// Lays a pipe ahead of the enemy, at the first spot that touches neither ball, no other pipe and stays in the
/// ring; none when no spot fits. Laying more than `max_pipes` drops the oldest.
pub fn lay_pipe(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let o = 1 - k;
    let a = f.rules.a;
    let ov = f.balls[o].v;
    let on = geom::norm(ov);
    let u = if on > 1e-9 {
        geom::div(ov, on)
    } else {
        let d = geom::sub(f.balls[o].p, f.balls[k].p);
        geom::div(d, geom::norm(d) + 1e-12)
    };
    let t = [-u[1], u[0]];
    let clear = [radius(f, 0) + w.thick + 0.1, radius(f, 1) + w.thick + 0.1];
    let lim = a - w.thick - 0.05;
    let mut spot = None;
    'find: for dl in [0.0, 0.5, -0.5, 1.0, -0.9] {
        for side in [0.0, 0.7, -0.7] {
            let c = geom::add(
                f.balls[o].p,
                geom::add(geom::scale(u, w.lead + dl), geom::scale(t, side)),
            );
            let pts = u_points(c, u, w);
            if pts.iter().any(|q| q[0].abs() > lim || q[1].abs() > lim) {
                continue;
            }
            if (0..2).any(|j| nearest(&pts, f.balls[j].p).1 < clear[j]) {
                continue;
            }
            let crowded = f.plumbing.pipes.iter().any(|pp| {
                if pp.boxed {
                    pts.iter().any(|q| nearest(&pp.pts, *q).1 < 0.3)
                } else {
                    geom::hypot(geom::sub(pp.c, c)) < 1.4
                }
            });
            if crowded {
                continue;
            }
            spot = Some((c, pts));
            break 'find;
        }
    }
    let Some((c, pts)) = spot else {
        return;
    };
    let id = f.plumbing.next_id;
    f.plumbing.next_id += 1;
    let mut data = vec![Value::Int(id)];
    for q in &pts {
        data.push(Value::Float(q[0]));
        data.push(Value::Float(q[1]));
    }
    f.plumbing.pipes.push(Pipe {
        owner: k,
        id,
        pts,
        c,
        last_hit: f64::NEG_INFINITY,
        ..Pipe::default()
    });
    f.ev("pipe", k as i64, data);
    let cap = (w.max_pipes as usize).max(1);
    let mine: Vec<i64> = f
        .plumbing
        .pipes
        .iter()
        .filter(|p| p.owner == k && !p.boxed)
        .map(|p| p.id)
        .collect();
    for &id in &mine[..mine.len().saturating_sub(cap)] {
        f.plumbing.pipes.retain(|p| p.id != id);
        f.ev("pipe_off", k as i64, vec![Value::Int(id)]);
    }
}

/// Whether pipes bounce ball `j` now: not one another step drives or that cannot be knocked.
fn bounces(f: &Fight, j: usize) -> bool {
    let b = &f.balls[j];
    !(b.walking
        || b.swan.is_some()
        || b.strut.is_some()
        || b.dive.is_some()
        || b.air
        || b.held
        || b.smashed > 0.0
        || b.anc.is_some()
        || rewinding(f, j))
}

/// One step: the boxes that are over open, the pipes bounce the balls (and hurt the enemy), the sprays hit.
pub fn step(f: &mut Fight) {
    let t = f.t;
    let ended: Vec<(usize, i64)> = f
        .plumbing
        .pipes
        .iter()
        .filter(|p| p.boxed && t >= p.until)
        .map(|p| (p.owner, p.id))
        .collect();
    for (k, id) in ended {
        f.plumbing.pipes.retain(|p| p.id != id);
        f.ev("pipe_box_off", k as i64, vec![Value::Int(id)]);
    }
    for i in 0..f.plumbing.pipes.len() {
        for j in 0..2 {
            let pipe = &f.plumbing.pipes[i];
            let k = pipe.owner;
            if (pipe.boxed && j == k) || !bounces(f, j) {
                continue;
            }
            let thick = weapon(f, k).thick;
            let r = radius(f, j);
            let p = f.balls[j].p;
            let (q, d) = nearest(&pipe.pts, p);
            if d >= r + thick {
                continue;
            }
            let v = f.balls[j].v;
            let n = if d > 1e-9 {
                geom::div(geom::sub(p, q), d)
            } else {
                geom::scale(v, -1.0 / (geom::norm(v) + 1e-12))
            };
            f.balls[j].p = geom::add(q, geom::scale(n, r + thick));
            let vn = geom::dot(v, n);
            if vn >= 0.0 {
                continue;
            }
            f.balls[j].v = geom::sub(v, geom::scale(n, 2.0 * vn));
            let w = weapon(f, k);
            if j == k {
                let b = &mut f.balls[k];
                b.boost = b.boost.max(w.own_boost);
                continue;
            }
            let pipe = &f.plumbing.pipes[i];
            let (boxed, id, cool) = (pipe.boxed, pipe.id, if pipe.boxed { pipe.box_cool } else { w.cool });
            if t - pipe.last_hit < cool {
                continue;
            }
            f.plumbing.pipes[i].last_hit = t;
            let hot = if t < f.plumbing.hot_until[k] { f.plumbing.hot_mult[k] } else { 1.0 };
            let base = if boxed {
                f.plumbing.pipes[i].box_dmg
            } else if f.plumbing.dmg[k] > 0.0 {
                f.plumbing.dmg[k]
            } else {
                w.dmg
            };
            let kind = if boxed { "pipe_box" } else { "pipe" };
            let landed = f.damage(k, j, base * hot, kind, q, id);
            if landed {
                if !boxed {
                    f.plumbing.dmg[k] = (base + w.dmg_grow).min(w.dmg_max);
                }
                let b = &mut f.balls[j];
                b.boost = b.boost.max(w.knock);
            }
        }
    }
    let mut i = 0;
    while i < f.plumbing.sprays.len() {
        let s = f.plumbing.sprays[i].clone();
        if t >= s.until {
            f.plumbing.sprays.remove(i);
            continue;
        }
        let o = 1 - s.owner;
        let op = f.balls[o].p;
        let ro = radius(f, o);
        let mut from = s.from.clone();
        from.push(f.balls[s.owner].p);
        let src = from.iter().find(|q| geom::hypot(geom::sub(op, **q)) - ro < s.reach).copied();
        if let Some(q) = src {
            f.plumbing.sprays.remove(i);
            if f.damage(s.owner, o, s.dmg, "spray", op, s.id) && s.knock > 0.0 {
                f.knock(o, geom::sub(op, q), s.knock);
            }
            continue;
        }
        i += 1;
    }
}

/// WATER KEY: for `beats` beats (default 1) every pipe end and the fighter's own ball spray; an enemy within
/// `reach` of one takes `dmg` once and is knocked away.
pub fn start_water_key(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let mut from = Vec::new();
    for p in f.plumbing.pipes.iter().filter(|p| p.owner == k && !p.boxed) {
        from.push(p.pts[0]);
        from.push(p.pts[p.pts.len() - 1]);
    }
    let id = f.plumbing.next_id;
    f.plumbing.next_id += 1;
    let mut data = vec![Value::Int(id)];
    for q in &from {
        data.push(Value::Float(q[0]));
        data.push(Value::Float(q[1]));
    }
    let beats = ab.get("beats").unwrap_or(1.0);
    f.plumbing.sprays.push(Spray {
        owner: k,
        id,
        from,
        until: f.t + beats * f.rules.beat,
        reach: param(ab, "reach"),
        dmg: param(ab, "dmg"),
        knock: ab.get("knock").unwrap_or(0.0),
    });
    f.ev("spray", k as i64, data);
}

/// HOT PIPES: for its duration the fighter's pipes hit `mult` times harder.
pub fn start_hot_pipes(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let dur = ab_dur(f, ab);
    f.plumbing.hot_until[k] = f.t + dur;
    f.plumbing.hot_mult[k] = param(ab, "mult");
    f.ev("hot_pipes", k as i64, vec![Value::Float(dur)]);
}

/// FIVE AND A HALF METRES: a square of tube of half-side `half` round the enemy for `beats` beats, kept in the ring
/// (the enemy is always inside it: it is at least its radius from the ropes).
pub fn start_box(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let o = 1 - k;
    let a = f.rules.a;
    let h = param(ab, "half").min(a);
    let p = f.balls[o].p;
    let c = [p[0].clamp(-(a - h), a - h), p[1].clamp(-(a - h), a - h)];
    let pts = vec![
        [c[0] - h, c[1] - h],
        [c[0] + h, c[1] - h],
        [c[0] + h, c[1] + h],
        [c[0] - h, c[1] + h],
        [c[0] - h, c[1] - h],
    ];
    let id = f.plumbing.next_id;
    f.plumbing.next_id += 1;
    f.plumbing.pipes.push(Pipe {
        owner: k,
        id,
        pts,
        c,
        boxed: true,
        until: f.t + param(ab, "beats") * f.rules.beat,
        last_hit: f64::NEG_INFINITY,
        box_dmg: param(ab, "dmg"),
        box_cool: ab.get("cool").unwrap_or(0.2),
    });
    let data = vec![Value::Int(id), Value::Float(c[0]), Value::Float(c[1]), Value::Float(h)];
    f.ev("pipe_box", k as i64, data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_point_of_a_u() {
        let pts = vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
        let (q, d) = nearest(&pts, [0.5, -0.5]);
        assert_eq!(q, [0.5, 0.0]);
        assert!((d - 0.5).abs() < 1e-12);
        let (q, _) = nearest(&pts, [-1.0, 0.5]);
        assert_eq!(q, [0.0, 0.5]);
    }
}
