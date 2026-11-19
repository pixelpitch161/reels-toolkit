//! Ability kinds `whip` (ACOUSTIC's FINGERPICK), `sweep` (ACOUSTIC's BIG STRUM): `fight.py` `trigger` 2872 to 2873;
//! `ab_x33` 566 to 577 and the whips and sweeps of `surf_rake_step` (630 to 670), which `kinds::rake::surf_rake_step`
//! calls. The third kind of the branch, `spout` (578 to 583, 671 to 699: UKULELE's UKE-NADO), chases the enemy.
//!
//! Python keeps both on the fight (`self.whips`, `self.sweeps`); here they are [`State`], `Fight::x33`. No copier
//! runs this code: `whip` and `sweep` are not in IDENTITY CRISIS's `COPYABLE` (2802), and the copy of the rake as a
//! signature weapon (`start_copy` 2817 to 2821) only logs its `copy` event (it plants or fires only an anchor or a
//! mortar). The owner is read from each whip and sweep all the same. The generator is never drawn.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;
use bf_core::roster::kinds::rake::sweep_front;

use crate::abilities::param;
use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::roster::AbilityCfg;

/// One string of a FINGERPICK flicking out (`Wp['live']`'s dict).
#[derive(Clone, Debug, PartialEq)]
pub struct Live {
    /// Its place in the whip, from 0 (an int in the `whip` event).
    pub j: i64,
    /// When it was emitted.
    pub t0: f64,
    /// Its aim (radians): at the enemy then.
    pub ang: f64,
    /// Whether it has hit.
    pub hit: bool,
}

/// One FINGERPICK (`self.whips`' dict).
#[derive(Clone, Debug, PartialEq)]
pub struct Whip {
    /// Who picks.
    pub owner: usize,
    /// When it started.
    pub t0: f64,
    /// Strings to emit (`ab['n']`).
    pub n: f64,
    /// Seconds from the first string to the last (`ab['beats'] * beat`).
    pub span: f64,
    /// Strings emitted so far.
    pub emitted: i64,
    /// A string's length beyond the surface.
    pub len: f64,
    /// Damage of a string.
    pub dmg: f64,
    /// Its knockback.
    pub knock: f64,
    /// Seconds a string takes to reach out (and as many to snap back).
    pub t_ext: f64,
    /// The strings still out.
    pub live: Vec<Live>,
}

/// One BIG STRUM (`self.sweeps`' dict).
#[derive(Clone, Debug, PartialEq)]
pub struct Sweep {
    /// Who strums.
    pub owner: usize,
    /// Where the ball was.
    pub c: V2,
    /// Toward the enemy then (unit).
    pub u: V2,
    /// When it started.
    pub t0: f64,
    /// Its length in seconds.
    pub dur: f64,
    /// How far beyond the surface the front goes.
    pub reach: f64,
    /// Its damage.
    pub dmg: f64,
    /// Its knockback.
    pub knock: f64,
    /// Whether it has hit.
    pub hit: bool,
}

/// One UKE-NADO (`self.spouts`' dict, ability kind `spout`).
#[derive(Clone, Debug, PartialEq)]
pub struct Spout {
    /// Who sent it.
    pub owner: usize,
    /// Where it is.
    pub p: V2,
    /// Where it heads (radians).
    pub ang: f64,
    /// When it started.
    pub t0: f64,
    /// Seconds it lives.
    pub life: f64,
    /// Units a second it moves.
    pub speed: f64,
    /// Radians a second it turns at most.
    pub turn: f64,
    /// Its radius.
    pub radius: f64,
    /// Seconds between two hits.
    pub tick: f64,
    /// Damage of a hit.
    pub dmg: f64,
    /// Hits at most.
    pub max_ticks: f64,
    /// Knockback of a hit.
    pub knock: f64,
    /// Hits so far.
    pub ticks: i64,
    /// When it last hit.
    pub last: f64,
    /// Steps so far.
    pub n: i64,
}

/// One point of a waterspout's path (`self.x33_log`'s tuple `(round(t, 5), owner, x, y)`, written every 4th step of
/// a spout, 690 to 691).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpoutPoint {
    /// `round(t, 5)`.
    pub t: f64,
    /// Who sent it.
    pub owner: usize,
    /// Where it was.
    pub p: V2,
}

/// This ability's per-fight state (`Fight::x33`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.whips`.
    pub whips: Vec<Whip>,
    /// `self.sweeps`.
    pub sweeps: Vec<Sweep>,
    /// `self.spouts` (UKULELE's UKE-NADO).
    pub spouts: Vec<Spout>,
    /// `self.x33_log`: the waterspouts' path, which `run()` returns as `X33_SPOUT` when it holds anything (6220 to
    /// 6221).
    pub x33_log: Vec<SpoutPoint>,
}

/// `trigger`, kinds `whip` and `sweep` (2872 to 2873): `ab_x33(k, ab, kind)` (566 to 577), `kind` being the ability's
/// kind. A whip only queues its strings (the first is emitted in this step's `surf_rake_step`); a sweep starts at
/// once with its event `sweep`: x, y, the aim, the reach, its seconds.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg, kind: &str) {
    let (bp, op) = (f.balls[k].p, f.balls[1 - k].p);
    let d = geom::sub(op, bp);
    let aim = dmath::atan2(d[1], d[0]);
    let beat = f.rules.beat;
    match kind {
        "whip" => f.x33.whips.push(Whip {
            owner: k,
            t0: f.t,
            n: param(ab, "n"),
            span: param(ab, "beats") * beat,
            emitted: 0,
            len: param(ab, "len"),
            dmg: param(ab, "dmg"),
            knock: param(ab, "knock"),
            t_ext: ab.get("t_ext").unwrap_or(0.07),
            live: Vec::new(),
        }),
        "sweep" => {
            let dur = param(ab, "beats") * beat;
            let reach = param(ab, "reach");
            f.x33.sweeps.push(Sweep {
                owner: k,
                c: bp,
                u: [dmath::cos(aim), dmath::sin(aim)],
                t0: f.t,
                dur,
                reach,
                dmg: param(ab, "dmg"),
                knock: param(ab, "knock"),
                hit: false,
            });
            let data = vec![
                Value::Float(bp[0]),
                Value::Float(bp[1]),
                Value::Float(aim),
                Value::Float(reach),
                Value::Float(dur),
            ];
            f.ev("sweep", k as i64, data);
        }
        // 578 to 583: `spout` (milestone 4, UKULELE's UKE-NADO)
        "spout" => start_spout(f, k, ab),
        other => panic!("ab_x33 has no branch for {other}"),
    }
}

/// `ab_x33`'s `spout` branch (578 to 583): UKE-NADO, a waterspout 1.2 R in front of the ball toward the enemy that
/// sweeps the ring after it. Event `spout`: x, y, the aim.
pub fn start_spout(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let (bp, op) = (f.balls[k].p, f.balls[1 - k].p);
    let d = geom::sub(op, bp);
    let aim = dmath::atan2(d[1], d[0]);
    let p = geom::add(bp, geom::scale([dmath::cos(aim), dmath::sin(aim)], f.rules.r * 1.2));
    f.x33.spouts.push(Spout {
        owner: k,
        p,
        ang: aim,
        t0: f.t,
        life: param(ab, "life"),
        speed: param(ab, "speed"),
        turn: param(ab, "turn"),
        radius: param(ab, "radius"),
        tick: param(ab, "tick"),
        dmg: param(ab, "dmg"),
        max_ticks: param(ab, "max_ticks"),
        knock: param(ab, "knock"),
        ticks: 0,
        last: -9.0,
        n: 0,
    });
    f.ev(
        "spout",
        k as i64,
        vec![Value::Float(p[0]), Value::Float(p[1]), Value::Float(aim)],
    );
}

/// Whether `self.whips`, `self.sweeps` or `self.spouts` holds anything: the lists of the test before
/// `surf_rake_step` (`run()` 6197).
pub fn live(f: &Fight) -> bool {
    !f.x33.whips.is_empty() || !f.x33.sweeps.is_empty() || !f.x33.spouts.is_empty()
}

/// Whether `self.spouts` holds anything (the test before [`spouts_step`]).
pub fn spouts_live(f: &Fight) -> bool {
    !f.x33.spouts.is_empty()
}

/// The spouts of `surf_rake_step(dt)` (671 to 699), after the sweeps; `kinds::rake::surf_rake_step` calls it. Each
/// spout turns toward the enemy (at most `turn * dt`), moves, bounces off the ropes, logs its place every 4th step,
/// and within `radius + R` of the enemy hits it every `tick` s (`hit` with `e[7] = 'spout'`, `e[8]` the tick; a knock
/// spun round it); it ends after its life or its last tick (event `spout_end`: x, y).
pub fn spouts_step(f: &mut Fight, dt: f64) {
    let (r, a_) = (f.rules.r, f.rules.a);
    let mut done = vec![false; f.x33.spouts.len()];
    for (i, gone) in done.iter_mut().enumerate() {
        let o = 1 - f.x33.spouts[i].owner;
        let op = f.balls[o].p;
        let t = f.t;
        let sp = &mut f.x33.spouts[i];
        let d = geom::sub(op, sp.p);
        let want = dmath::atan2(d[1], d[0]);
        sp.ang += geom::np_clip(geom::wrap(want - sp.ang), -sp.turn * dt, sp.turn * dt);
        sp.p = geom::add(
            sp.p,
            geom::scale([dmath::cos(sp.ang), dmath::sin(sp.ang)], sp.speed * dt),
        );
        let lim = a_ - sp.radius;
        for ax in 0..2 {
            if sp.p[ax].abs() > lim {
                sp.p[ax] = lim.copysign(sp.p[ax]) * 2.0 - sp.p[ax];
                let mut v = [dmath::cos(sp.ang), dmath::sin(sp.ang)];
                v[ax] = -v[ax];
                sp.ang = dmath::atan2(v[1], v[0]);
            }
        }
        sp.n += 1;
        let sp = sp.clone();
        if sp.n % 4 == 0 {
            f.x33.x33_log.push(SpoutPoint {
                t: py::round_nd(t, 5),
                owner: sp.owner,
                p: sp.p,
            });
        }
        let dist = geom::hypot(d);
        if dist < sp.radius + r && t - sp.last >= sp.tick - 1e-9 && (sp.ticks as f64) < sp.max_ticks {
            let ticks = sp.ticks + 1;
            f.x33.spouts[i].last = t;
            f.x33.spouts[i].ticks = ticks;
            let rel = geom::div(d, py::py_max(dist, 1e-9));
            let tang = [-rel[1], rel[0]];
            let h = geom::sub(op, geom::scale(rel, r));
            f.damage(sp.owner, o, sp.dmg, "spout", h, ticks);
            f.knock(o, geom::add(geom::scale(tang, 0.9), geom::scale(rel, 0.4)), sp.knock);
        }
        let sp = &f.x33.spouts[i];
        if f.t - sp.t0 >= sp.life || (sp.ticks as f64) >= sp.max_ticks {
            *gone = true;
            let (owner, p) = (sp.owner, sp.p);
            f.ev("spout_end", owner as i64, vec![Value::Float(p[0]), Value::Float(p[1])]);
        }
    }
    let mut it = done.into_iter();
    f.x33.spouts.retain(|_| !it.next().unwrap_or(false));
}

/// `out['X33_SPOUT'] = self.x33_log` (`run()` 6220 to 6221): the waterspouts' path as the recording's `X33_SPOUT`,
/// called when the log holds anything. A recording's arrays are per frame, so the log is a ragged list of `frames`
/// rows: row `i` holds the points `[t, owner, x, y]` logged after frame `i - 1` up to frame `i` (the last row takes
/// the rest). `None` for a fight that did not record.
pub(crate) fn spout_log(f: &Fight, frames: usize) -> Option<bf_core::recording::Array> {
    if frames == 0 {
        return None;
    }
    let fps = f64::from(f.rules.fps);
    let mut rows = vec![Vec::new(); frames];
    for q in &f.x33.x33_log {
        let j = ((q.t - f.rules.t_fight) * fps - 1e-6).ceil().max(0.0) as usize;
        let row = [q.t, q.owner as f64, q.p[0], q.p[1]].map(Value::Float);
        rows[j.min(frames - 1)].push(Value::List(row.to_vec()));
    }
    Some(bf_core::recording::Array::Nested(Value::List(
        rows.into_iter().map(Value::List).collect(),
    )))
}

/// The whips and sweeps of `surf_rake_step` (630 to 670), after its strokes.
pub fn step(f: &mut Fight) {
    whips_step(f);
    sweeps_step(f);
}

/// 630 to 658: each whip emits the strings that are due (event `whip`: its place, the aim, the length), each string
/// out grows to `len` in `t_ext` s and hits the enemy once within R + 0.05 of it (`hit` with `e[7] = 'whip'`, `e[8]`
/// its place; a knock along it); strings are dropped after 2 `t_ext`, whips when every string is out and back.
fn whips_step(f: &mut Fight) {
    let r = f.rules.r;
    for i in 0..f.x33.whips.len() {
        let owner = f.x33.whips[i].owner;
        let o = 1 - owner;
        // while Wp['emitted'] < Wp['n']
        loop {
            let wp = &f.x33.whips[i];
            if (wp.emitted as f64) >= wp.n {
                break;
            }
            let j = wp.emitted;
            // te = Wp['t0'] + Wp['span'] * j / max(1, Wp['n'] - 1)
            let te = wp.t0 + wp.span * j as f64 / py::py_max(1.0, wp.n - 1.0);
            if f.t + 1e-9 < te {
                break;
            }
            let d = geom::sub(f.balls[o].p, f.balls[owner].p);
            let ang = dmath::atan2(d[1], d[0]);
            let t = f.t;
            let wp = &mut f.x33.whips[i];
            wp.live.push(Live {
                j,
                t0: t,
                ang,
                hit: false,
            });
            let len = wp.len;
            wp.emitted += 1;
            f.ev(
                "whip",
                owner as i64,
                vec![Value::Int(j), Value::Float(ang), Value::Float(len)],
            );
        }
        for li in 0..f.x33.whips[i].live.len() {
            let wp = &f.x33.whips[i];
            let lw = wp.live[li].clone();
            let tau = f.t - lw.t0;
            if lw.hit || tau > wp.t_ext {
                continue;
            }
            let (len, t_ext, dmg, knock) = (wp.len, wp.t_ext, wp.dmg, wp.knock);
            let u = [dmath::cos(lw.ang), dmath::sin(lw.ang)];
            let p0 = geom::add(f.balls[owner].p, geom::scale(u, r));
            let l = len * py::py_min(1.0, tau / t_ext);
            let rel = geom::sub(f.balls[o].p, p0);
            // a = float(np.clip(rel @ u, 0.0, L))
            let a = geom::np_clip(geom::dot(rel, u), 0.0, l);
            if geom::hypot(geom::sub(rel, geom::scale(u, a))) < r + 0.05 {
                f.x33.whips[i].live[li].hit = true;
                let h = geom::add(p0, geom::scale(u, a));
                f.damage(owner, o, dmg, "whip", h, lw.j);
                f.knock(o, u, knock);
            }
        }
        let t = f.t;
        let wp = &mut f.x33.whips[i];
        let t_ext = wp.t_ext;
        wp.live.retain(|lw| t - lw.t0 < 2.0 * t_ext);
    }
    f.x33
        .whips
        .retain(|wp| (wp.emitted as f64) < wp.n || !wp.live.is_empty());
}

/// 659 to 670: each sweep's front crosses the ring; the first step the enemy's centre is within -1.2 R to R of it,
/// it hits once (`hit` with `e[7] = 'sweep'` on the enemy's near side; a knock along the sweep). A sweep is dropped
/// after its time.
fn sweeps_step(f: &mut Fight) {
    let r = f.rules.r;
    for i in 0..f.x33.sweeps.len() {
        let sw = f.x33.sweeps[i].clone();
        let tau = f.t - sw.t0;
        if !sw.hit {
            let o = 1 - sw.owner;
            let op = f.balls[o].p;
            // a = float((O.p - Sw['c']) @ Sw['u']) - self.sweep_front(tau, Sw['dur'], Sw['reach'], R)
            let a = geom::dot(geom::sub(op, sw.c), sw.u) - sweep_front(tau, sw.dur, sw.reach, r);
            if -r * 1.2 <= a && a <= r {
                f.x33.sweeps[i].hit = true;
                let h = geom::sub(op, geom::scale(sw.u, r));
                f.damage(sw.owner, o, sw.dmg, "sweep", h, 0);
                f.knock(o, sw.u, sw.knock);
            }
        }
    }
    let t = f.t;
    f.x33.sweeps.retain(|sw| t - sw.t0 < sw.dur + 1e-9);
}
