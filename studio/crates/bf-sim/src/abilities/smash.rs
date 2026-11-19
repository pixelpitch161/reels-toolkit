//! Ability kind `smash` (ELECTRIC's the smash): `fight.py` `trigger` 2939 to 2940; 3923 to 3964 (`start_smash`,
//! `splinters_step`); the smashed timer and its `rebuild` event (`run()` 6104 to 6107, in `schedule.rs`).
//!
//! The fighter smashes itself on the floor: a ring of kind `smash` (`speed`, `radius`, `dmg`, `knock`, through the
//! shared waves) and `n` splinters flying out from it (the first aimed at the enemy, the rest spread round, each
//! turned by up to 0.25 rad from a generator of its own, not the fight's), each hitting once for `splinter_dmg`
//! (knock `splinter_knock`) or flying on to the ropes. It then lies in pieces for `rebuild_beats` beats (`smashed`:
//! still, intangible, no clash, nothing started; the shared engine code tests it). A copied ability (IDENTITY CRISIS)
//! runs this code for the copier's ball, so nothing here assumes which side has the kind.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;
use bf_core::rng::Generator;

use crate::abilities::param;
use crate::fight::{Fight, Wave};
use crate::geom::{self, V2};
use crate::roster::AbilityCfg;

/// One splinter of `self.splinters`.
#[derive(Clone, Debug, PartialEq)]
pub struct Splinter {
    /// `owner`: the fighter that smashed.
    pub owner: usize,
    /// `j`: its number.
    pub j: i64,
    /// `o`: where it started.
    pub o: V2,
    /// `u`: its unit heading.
    pub u: V2,
    /// `t0`: when it started.
    pub t0: f64,
    /// `speed`, `dmg`, `knock`.
    pub speed: f64,
    /// See `speed`.
    pub dmg: f64,
    /// See `speed`.
    pub knock: f64,
}

/// This ability's per-fight state (`Fight::smash`): `self.splinters` (empty before the first smash, which Python's
/// `getattr(self, 'splinters', None)` reads as false just as an empty list).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The splinters still flying, in the order they left.
    pub splinters: Vec<Splinter>,
}

/// `trigger`, kind `smash`: `trigger` 2939 to 2940, `start_smash(k, ab)` 3923 to 3946. Events `smash` (x, y, the
/// radius, the seconds in pieces, the splinters) and one `splinter` each (its number, angle and speed).
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let bp = f.balls[k].p;
    let d = geom::sub(f.balls[1 - k].p, bp);
    let aim = dmath::atan2(d[1], d[0]);
    let smashed = param(ab, "rebuild_beats") * f.rules.beat;
    f.balls[k].smashed = smashed;
    let (radius, n) = (param(ab, "radius"), param(ab, "n"));
    let data = vec![
        Value::Float(bp[0]),
        Value::Float(bp[1]),
        Value::Float(radius),
        Value::Float(smashed),
        // int(ab['n']): toward zero (the roster keeps it finite)
        Value::Int(n as i64),
    ];
    f.ev("smash", k as i64, data);
    f.waves.push(Wave {
        owner: k,
        c: bp,
        r: f.rules.r,
        speed: param(ab, "speed"),
        rmax: radius,
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        kind: "smash".to_string(),
        hit: false,
        name: ab.name.clone(),
        t0: f.t,
        slow: None,
        stun: None,
        detune: None,
        grow: false,
        clears: true,
    });
    // np.random.default_rng([int(self.seed), 44, int(round(self.t * 1000))]): not the fight's own stream
    let ms = py::round_half_even(f.t * 1000.0) as u64;
    let mut rng = Generator::from_seq(&[f.seed, 44, ms]);
    let n = n as i64;
    let (speed, dmg) = (param(ab, "splinter_speed"), param(ab, "splinter_dmg"));
    let knock = ab.get("splinter_knock").unwrap_or(0.4);
    for j in 0..n {
        // a = aim + 2 * math.pi * j / n + (rng.uniform(-0.25, 0.25) if j else 0.0)
        let turn = if j != 0 { rng.uniform(-0.25, 0.25) } else { 0.0 };
        let a = aim + 2.0 * dmath::PI * j as f64 / n as f64 + turn;
        let u = [dmath::cos(a), dmath::sin(a)];
        f.smash.splinters.push(Splinter {
            owner: k,
            j,
            o: bp,
            u,
            t0: f.t,
            speed,
            dmg,
            knock,
        });
        f.ev(
            "splinter",
            k as i64,
            vec![Value::Int(j), Value::Float(a), Value::Float(speed)],
        );
    }
}

/// Whether `self.splinters` holds anything: the test before `splinters_step` (`run()` 6193).
pub fn live(f: &Fight) -> bool {
    !f.smash.splinters.is_empty()
}

/// `splinters_step(dt)` (3948 to 3964), from `run()` 6193 to 6194: each splinter where it has flown to; one within
/// R + 0.08 of the enemy's centre hits it (the hit `splinter`, sub its number, at the enemy's near side; knocked
/// along its heading) and ends; one at the ropes ends. Event `splinter_end`: its number, where, and 1 for a hit.
pub fn splinters_step(f: &mut Fight, _dt: f64) {
    let (a, r) = (f.rules.a, f.rules.r);
    let mut list = std::mem::take(&mut f.smash.splinters);
    let mut done = vec![false; list.len()];
    for (i, sp) in list.iter().enumerate() {
        // p = Sp['o'] + Sp['u'] * Sp['speed'] * (self.t - Sp['t0'])
        let p = geom::add(sp.o, geom::scale(geom::scale(sp.u, sp.speed), f.t - sp.t0));
        let o = 1 - sp.owner;
        let op = f.balls[o].p;
        if geom::hypot(geom::sub(op, p)) < r + 0.08 {
            let h = geom::sub(op, geom::scale(sp.u, r));
            f.damage(sp.owner, o, sp.dmg, "splinter", h, sp.j);
            f.knock(o, sp.u, sp.knock);
            done[i] = true;
            let data = vec![Value::Int(sp.j), Value::Float(h[0]), Value::Float(h[1]), Value::Int(1)];
            f.ev("splinter_end", sp.owner as i64, data);
        } else if p[0].abs() >= a || p[1].abs() >= a {
            done[i] = true;
            let data = vec![Value::Int(sp.j), Value::Float(p[0]), Value::Float(p[1]), Value::Int(0)];
            f.ev("splinter_end", sp.owner as i64, data);
        }
    }
    let mut keep = done.iter().map(|d| !d);
    list.retain(|_| keep.next().unwrap_or(true));
    // nothing in the loop adds splinters (damage and knock start no ability)
    debug_assert!(f.smash.splinters.is_empty());
    f.smash.splinters = list;
}
