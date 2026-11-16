//! Weapon kind `swarm`, SOPRANO SAX's HORNET SWARM, and its abilities KICK THE HIVE (`hive`), BUZZ OFF (`buzz`) and
//! QUEEN (`queen`). SOPRANO SAX is a fighter made from scratch in the Studio (2026-11-16): there is no Python kit
//! reference, this module is the source of truth (`docs/superpowers/specs/2026-11-16-soprano-sax-fighter-design.md`).
//!
//! **HORNET SWARM.** `start` hornets orbit the fighter, growing to `max`. Every `every` beats (from 16th `off16`), when
//! it is not stunned, hornets peel off (one, plus one for every two the swarm has grown, never more than are orbiting)
//! and chase the enemy at `speed`, steering toward it at `steer` a second, for up to `life` beats. A chaser that
//! touches the enemy stings (`dmg`, `knock`, kind `sting`) and flies home, and the swarm grows by one. A blocked or
//! tired chaser flies home with no growth. Events `hornet` (launched, orbiting left), `swarm_grow` (the new size).
//!
//! **KICK THE HIVE** (`hive`): every orbiting hornet (at least 3, so a copied one works without a swarm) charges at
//! once for the ability's duration, each stinging once (`dmg`, kind `hive`). Event `hive` (how many).
//!
//! **BUZZ OFF** (`buzz`): for its duration the swarm wraps the fighter: the shared shield timer blocks hits on it, and
//! an enemy that touches it is stung (`dmg`, `knock`, kind `buzz`) at most twice a beat. Event `buzz` (seconds).
//!
//! **QUEEN** (`queen`): a queen hornet chases the enemy at `speed` for its duration, stinging (`dmg`, `knock`, kind
//! `queen`) at most once every `gap` beats and bouncing off after each sting. Event `queen` (seconds).
//!
//! The record key `HRN` is (frames, 2, 33) floats per ball: the swarm's size, how many orbit, BUZZ OFF's seconds
//! left, then up to [`SLOTS`] hornets in the air: x, y, what (1 chaser, 2 hive, 3 queen; plus 10 flying home; NaN
//! empty). None of this code draws from the fight's generator.

use bf_core::dmath;
use bf_core::event::Value;

use crate::abilities::{ab_dur, param};
use crate::fight::{Fight, radius, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, SwarmCfg, WeaponCfg};

/// NaN, for an empty slot of the record.
const NAN: f64 = f64::NAN;

/// The hornets in the air the record keeps per ball.
pub const SLOTS: usize = 10;

/// What a hornet in the air is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeeKind {
    /// A weapon chaser (it came from the orbit and goes back to it).
    Chaser,
    /// A KICK THE HIVE charger (from the orbit when the fighter has one).
    Hive,
    /// The queen.
    Queen,
}

impl BeeKind {
    /// The record's code.
    pub fn code(self) -> f64 {
        match self {
            BeeKind::Chaser => 1.0,
            BeeKind::Hive => 2.0,
            BeeKind::Queen => 3.0,
        }
    }
}

/// A hornet in the air.
#[derive(Clone, Debug, PartialEq)]
pub struct Bee {
    /// The fighter it flies for.
    pub owner: usize,
    /// What it is.
    pub kind: BeeKind,
    /// Where it is.
    pub p: V2,
    /// Its velocity.
    pub v: V2,
    /// When it gives up the hunt.
    pub t_end: f64,
    /// Whether it is flying home.
    pub home: bool,
    /// Whether it came from the orbit (and rejoins it).
    pub orbit: bool,
    /// Its speed.
    pub speed: f64,
    /// Its turn rate.
    pub steer: f64,
    /// A sting's damage and knock.
    pub dmg: f64,
    /// A sting's knock.
    pub knock: f64,
    /// Its radius for the contact test.
    pub r: f64,
    /// The queen's next allowed sting (others sting once).
    pub next: f64,
    /// Seconds between the queen's stings.
    pub gap: f64,
    /// The damage kind.
    pub hit_kind: &'static str,
    /// The ability's name, or "".
    pub name: String,
}

/// Weapon kind `swarm`'s per-fight state (`Fight::swarm`), with its abilities' hornets and BUZZ OFF's timers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// Each fighter's swarm size (0 without a swarm).
    pub total: [i64; 2],
    /// Each fighter's hornets away from the orbit.
    pub away: [i64; 2],
    /// The hornets in the air.
    pub bees: Vec<Bee>,
    /// When each fighter's BUZZ OFF ends.
    pub buzz_end: [f64; 2],
    /// Each fighter's BUZZ OFF sting damage and knock.
    pub buzz_hit: [[f64; 2]; 2],
    /// When each fighter's BUZZ OFF may sting next.
    pub buzz_next: [f64; 2],
}

/// Fighter `k`'s weapon is a swarm.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Swarm(_))
}

/// Whether anything of this module is under way (a swarm, a hornet in the air, a BUZZ OFF).
pub fn live(f: &Fight) -> bool {
    is(f, 0) || is(f, 1) || !f.swarm.bees.is_empty() || f.t < f.swarm.buzz_end[0] || f.t < f.swarm.buzz_end[1]
}

/// The swarm's settings.
///
/// # Panics
/// When fighter `k`'s weapon is not a swarm.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a SwarmCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Swarm(w) => w,
        other => panic!("fighter {k} ({}) has no swarm: {other:?}", cfg.name),
    }
}

/// A unit vector along `d`, or `fallback` when `d` is (nearly) zero.
fn unit(d: V2, fallback: V2) -> V2 {
    let n = geom::hypot(d);
    if n < 1e-9 { fallback } else { geom::div(d, n) }
}

/// Where orbiting hornet `i` of `n` sits at time `t` (for launches; the look draws the same circle).
pub fn orbit_at(p: V2, r: f64, i: i64, n: i64, t: f64) -> V2 {
    let a = t * 4.0 + 2.0 * dmath::PI * i as f64 / n.max(1) as f64;
    let d = r + 0.22;
    [p[0] + d * dmath::cos(a), p[1] + d * dmath::sin(a)]
}

/// The swarm's setup: `start` hornets, none away.
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    f.swarm.total[k] = w.start as i64;
    f.swarm.away[k] = 0;
}

/// Launches `n` hornets of kind `kind` for fighter `k` from around its ball.
#[allow(clippy::too_many_arguments)]
fn launch(
    f: &mut Fight,
    k: usize,
    n: i64,
    kind: BeeKind,
    orbit: bool,
    hunt: f64,
    hit: (f64, f64, f64, f64),
    name: &str,
) {
    let (speed, steer, dmg, knock) = hit;
    let o = 1 - k;
    let (bp, op) = (f.balls[k].p, f.balls[o].p);
    let r = radius(f, k);
    let total = f.swarm.total[k].max(n);
    let toward = unit(geom::sub(op, bp), [1.0, 0.0]);
    for i in 0..n {
        let p = orbit_at(bp, r, i, total, f.t);
        let out = unit(geom::sub(p, bp), toward);
        // leave the orbit sideways and swing round onto the enemy
        let v = geom::scale(unit(geom::add(out, toward), toward), speed);
        let bee_r = if kind == BeeKind::Queen { 0.24 } else { 0.12 };
        f.swarm.bees.push(Bee {
            owner: k,
            kind,
            p,
            v,
            t_end: f.t + hunt,
            home: false,
            orbit,
            speed,
            steer,
            dmg,
            knock,
            r: if kind == BeeKind::Chaser && is(f, k) {
                weapon(f, k).bee_r
            } else {
                bee_r
            },
            next: f.t,
            gap: 0.0,
            hit_kind: match kind {
                BeeKind::Chaser => "sting",
                BeeKind::Hive => "hive",
                BeeKind::Queen => "queen",
            },
            name: name.to_string(),
        });
    }
    if orbit {
        f.swarm.away[k] += n;
    }
}

/// The swarm's 16th grid: a launch on the grid when not stunned and some hornets orbit.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    if stunned(f, k) {
        return;
    }
    let main16 = bf_core::py::round_half_even(4.0 * w.every) as i64;
    let off = w.off16 as i64;
    if k16 < off || bf_core::py::imod_py(k16 - off, main16) != 0 {
        return;
    }
    let orbiting = f.swarm.total[k] - f.swarm.away[k];
    let grown = f.swarm.total[k] - w.start as i64;
    let n = (1 + grown / 2).min(orbiting);
    if n <= 0 {
        return;
    }
    let beat = f.rules.beat;
    launch(
        f,
        k,
        n,
        BeeKind::Chaser,
        true,
        w.life * beat,
        (w.speed, w.steer, w.dmg, w.knock),
        "",
    );
    f.ev("hornet", k as i64, vec![Value::Int(n), Value::Int(orbiting - n)]);
}

/// KICK THE HIVE's trigger: every orbiting hornet (at least 3) charges.
pub fn start_hive(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let dur = ab_dur(f, ab);
    let dmg = param(ab, "dmg");
    let (speed, steer, orbit, n) = if is(f, k) {
        let w = weapon(f, k);
        let orbiting = f.swarm.total[k] - f.swarm.away[k];
        if orbiting >= 3 {
            (w.speed * 1.15, w.steer * 1.3, true, orbiting)
        } else {
            (w.speed * 1.15, w.steer * 1.3, false, 3)
        }
    } else {
        (6.5, 7.0, false, 3)
    };
    launch(f, k, n, BeeKind::Hive, orbit, dur, (speed, steer, dmg, 0.25), &ab.name);
    f.ev("hive", k as i64, vec![Value::Int(n)]);
}

/// BUZZ OFF's trigger: the shield timer, and the swarm stings what touches it.
pub fn start_buzz(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let dur = ab_dur(f, ab);
    let b = &mut f.balls[k];
    b.shield = geom::py_max(b.shield, dur);
    f.swarm.buzz_end[k] = f.t + dur;
    f.swarm.buzz_hit[k] = [param(ab, "dmg"), param(ab, "knock")];
    f.swarm.buzz_next[k] = f.t;
    f.ev("buzz", k as i64, vec![Value::Float(dur)]);
}

/// QUEEN's trigger: a queen hornet hunts for the ability's duration.
pub fn start_queen(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let dur = ab_dur(f, ab);
    let gap = param(ab, "gap") * f.rules.beat;
    let hit = (param(ab, "speed"), 5.0, param(ab, "dmg"), param(ab, "knock"));
    launch(f, k, 1, BeeKind::Queen, false, dur, hit, &ab.name);
    if let Some(q) = f.swarm.bees.last_mut() {
        q.gap = gap;
    }
    f.ev("queen", k as i64, vec![Value::Float(dur)]);
}

/// The hornets fly on by one step: hunting, stinging, flying home; BUZZ OFF stings what touches its fighter.
pub fn step(f: &mut Fight, dt: f64) {
    let a = f.rules.a - 0.05;
    let mut i = 0;
    while i < f.swarm.bees.len() {
        let mut b = f.swarm.bees[i].clone();
        let (k, o) = (b.owner, 1 - b.owner);
        let (kp, op) = (f.balls[k].p, f.balls[o].p);
        let target = if b.home { kp } else { op };
        let want = geom::scale(
            unit(geom::sub(target, b.p), b.v),
            b.speed * if b.home { 1.4 } else { 1.0 },
        );
        let blend = (b.steer * dt).min(1.0);
        b.v = geom::add(b.v, geom::scale(geom::sub(want, b.v), blend));
        b.p = geom::add(b.p, geom::scale(b.v, dt));
        b.p = [geom::np_clip(b.p[0], -a, a), geom::np_clip(b.p[1], -a, a)];
        if b.home {
            if geom::hypot(geom::sub(kp, b.p)) <= radius(f, k) + 0.1 {
                if b.orbit {
                    f.swarm.away[k] = (f.swarm.away[k] - 1).max(0);
                }
                f.swarm.bees.remove(i);
                continue;
            }
        } else if geom::hypot(geom::sub(op, b.p)) < radius(f, o) + b.r && f.t >= b.next {
            let landed = f.damage(k, o, b.dmg, b.hit_kind, b.p, 0);
            if landed {
                let away = geom::sub(op, b.p);
                f.knock(o, away, b.knock);
            }
            if b.kind == BeeKind::Queen {
                // bounce off and come round again
                b.v = geom::scale(unit(geom::sub(b.p, op), [1.0, 0.0]), b.speed);
                b.next = f.t + b.gap;
            } else {
                b.home = true;
                if landed && b.kind == BeeKind::Chaser && is(f, k) {
                    let max = weapon(f, k).max as i64;
                    if f.swarm.total[k] < max {
                        f.swarm.total[k] += 1;
                        f.ev("swarm_grow", k as i64, vec![Value::Int(f.swarm.total[k])]);
                    }
                }
            }
        } else if f.t >= b.t_end {
            if b.kind == BeeKind::Queen {
                f.swarm.bees.remove(i);
                continue;
            }
            b.home = true;
        }
        f.swarm.bees[i] = b;
        i += 1;
    }
    // BUZZ OFF's armour stings
    let half = 0.5 * f.rules.beat;
    for k in 0..2 {
        if f.t >= f.swarm.buzz_end[k] || f.t < f.swarm.buzz_next[k] {
            continue;
        }
        let o = 1 - k;
        let (kp, op) = (f.balls[k].p, f.balls[o].p);
        let d = geom::sub(op, kp);
        if geom::hypot(d) < radius(f, k) + radius(f, o) + 0.2 {
            let [dmg, knock] = f.swarm.buzz_hit[k];
            let at = geom::add(kp, geom::scale(unit(d, [1.0, 0.0]), radius(f, k)));
            if f.damage(k, o, dmg, "buzz", at, 0) {
                f.knock(o, d, knock);
            }
            f.swarm.buzz_next[k] = f.t + half;
        }
    }
}

/// The record keys this kind brings: `HRN` when a fighter has a swarm or might copy its abilities.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) { &["HRN"] } else { &[] }
}

/// This kind's rows of one frame (`HRN`).
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    const W: usize = 3 + 3 * SLOTS;
    let mut row = [NAN; 2 * W];
    for j in 0..2 {
        let o = W * j;
        row[o] = f.swarm.total[j] as f64;
        row[o + 1] = (f.swarm.total[j] - f.swarm.away[j]).max(0) as f64;
        row[o + 2] = geom::py_max(0.0, f.swarm.buzz_end[j] - f.t);
        for (s, b) in f.swarm.bees.iter().filter(|b| b.owner == j).take(SLOTS).enumerate() {
            row[o + 3 + 3 * s] = b.p[0];
            row[o + 4 + 3 * s] = b.p[1];
            row[o + 5 + 3 * s] = b.kind.code() + if b.home { 10.0 } else { 0.0 };
        }
    }
    out.push_f64("HRN", &[2, W], &row);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_spreads_hornets_round_the_ball() {
        let a = orbit_at([0.0, 0.0], 0.5, 0, 4, 0.0);
        let b = orbit_at([0.0, 0.0], 0.5, 2, 4, 0.0);
        assert!((geom::hypot(a) - 0.72).abs() < 1e-9);
        assert!((a[0] + b[0]).abs() < 1e-9 && (a[1] + b[1]).abs() < 1e-9);
    }
}
