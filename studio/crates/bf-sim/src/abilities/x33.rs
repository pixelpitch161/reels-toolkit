//! Ability kinds `whip` (ACOUSTIC's FINGERPICK), `sweep` (ACOUSTIC's BIG STRUM): `fight.py` `trigger` 2872 to 2873;
//! `ab_x33` 566 to 577 and the whips and sweeps of `surf_rake_step` (630 to 670), which `kinds::rake::surf_rake_step`
//! calls. The third kind of the branch, `spout` (578 to 583, 671 to 699), is older than the kit's: SLOT.
//!
//! Python keeps both on the fight (`self.whips`, `self.sweeps`); here they are [`State`], `Fight::x33`. A copied
//! ability (IDENTITY CRISIS) runs this code for the copier's ball: nothing here assumes which side has the kind.
//! The generator is never drawn.

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

/// This ability's per-fight state (`Fight::x33`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.whips`.
    pub whips: Vec<Whip>,
    /// `self.sweeps`.
    pub sweeps: Vec<Sweep>,
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
        // 578 to 583: `spout` (an older kind). SLOT.
        other => panic!("ab_x33 has no branch for {other}"),
    }
}

/// Whether `self.whips` or `self.sweeps` holds anything: half of the test before `surf_rake_step` (`run()` 6197;
/// `self.spouts`, the other half's third list, belongs to an older kind).
pub fn live(f: &Fight) -> bool {
    !f.x33.whips.is_empty() || !f.x33.sweeps.is_empty()
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
