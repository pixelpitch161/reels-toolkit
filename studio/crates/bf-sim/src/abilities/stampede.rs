//! Ability kind `stampede` (CONTRABASS's THE ELEPHANT): `fight.py` `trigger` 2953 to 2954; 4302 to 4368
//! (`start_stampede`, `stampede_step`).
//!
//! For `beats` beats the fighter stops stepping and charges after the enemy at `speed` x the shared speed (turning at
//! `turn` rad/s, bouncing off the ropes), walking the whole time (no knockback, no grab, it gives no way). On contact
//! it tramples (`dmg`, `knock`, at most every `cool` s) and charges on past (its heading reflected off the contact),
//! and every `stomp16` 16ths its feet shake the floor: a ring of kind `stampede` `stomp_r` beyond its surface
//! (`stomp_dmg`, `stomp_knock`; it clears no projectile). A held fighter breaks free first. Events (`stampede`, its
//! end), (`trample`, where), (`stampede_stomp`, where, the ring's reach), (`stampede_end`). It draws nothing.
//!
//! Python hangs the charge on the ball: `B.stamp` is [`State::stamp`] (`Fight::stampede`), by fighter. A copied
//! stampede (IDENTITY CRISIS) runs this code for the copier's ball; `run()` steps only the fighters that have the
//! kind themselves (`stampeders`, 5872 to 5873), as Python does.

use bf_core::dmath;
use bf_core::event::Value;

use crate::abilities::param;
use crate::fight::{Fight, Wave};
use crate::geom::{self, V2, py_max, py_min};
use crate::kinds::{grapple, walk};
use crate::roster::AbilityCfg;

/// THE ELEPHANT's charge (`B.stamp`, `start_stampede` 4320). Python keeps the ability's dict (`St['ab']`) and reads
/// its numbers every step; they are the same numbers, copied here when the charge starts.
#[derive(Clone, Debug, PartialEq)]
pub struct Stamp {
    /// When it ends.
    pub t1: f64,
    /// When the next stomp is due.
    pub next: f64,
    /// The time of the last trample (-9.0 before the first).
    pub last: f64,
    /// `ab['speed']`: x the shared speed.
    pub speed: f64,
    /// `ab['turn']`: rad/s.
    pub turn: f64,
    /// `ab['dmg']`: a trample's damage.
    pub dmg: f64,
    /// `ab['knock']`: a trample's knockback.
    pub knock: f64,
    /// `ab['cool']`: seconds between two tramples.
    pub cool: f64,
    /// `ab['stomp16']`: 16ths between two stomps.
    pub stomp16: f64,
    /// `ab['stomp_r']`: a stomp's reach beyond the ball's surface.
    pub stomp_r: f64,
    /// `ab.get('stomp_speed', 9.0)`.
    pub stomp_speed: f64,
    /// `ab['stomp_dmg']`.
    pub stomp_dmg: f64,
    /// `ab['stomp_knock']`.
    pub stomp_knock: f64,
    /// `ab['name']`: the stomp ring's name.
    pub name: String,
}

/// This ability's per-fight state (`Fight::stampede`): `B.stamp`, by fighter (`None` outside a charge).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.stamp`.
    pub stamp: [Option<Stamp>; 2],
}

/// `trigger`, kind `stampede`: `trigger` 2953 to 2954, `start_stampede(k, ab)` (4309 to 4321).
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    if f.balls[k].grabbed {
        grapple::release(f, 1 - k, "stampede");
    }
    // `if getattr(B, 'wstep', None) is not None`: a step under way stops where it is (only a walking bass has one)
    if walk::is(f, k) {
        f.walk.wstep[k] = None;
    }
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let n = geom::hypot(d);
    let u: V2 = if n > 1e-6 { geom::div(d, n) } else { [1.0, 0.0] };
    let b = &mut f.balls[k];
    b.v = geom::scale(u, f.rules.v);
    b.walking = true;
    b.boost = 0.0;
    let t1 = f.t + param(ab, "beats") * f.rules.beat;
    f.stampede.stamp[k] = Some(Stamp {
        t1,
        next: f.t,
        last: -9.0,
        speed: param(ab, "speed"),
        turn: param(ab, "turn"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        cool: param(ab, "cool"),
        stomp16: param(ab, "stomp16"),
        stomp_r: param(ab, "stomp_r"),
        stomp_speed: ab.get("stomp_speed").unwrap_or(9.0),
        stomp_dmg: param(ab, "stomp_dmg"),
        stomp_knock: param(ab, "stomp_knock"),
        name: ab.name.clone(),
    });
    f.ev("stampede", k as i64, vec![Value::Float(t1)]);
}

/// `stampede_step(k, dt)` (4323 to 4367), from `run()` 6181 to 6182: the charge ends when its time is up; else it
/// turns toward the enemy (at most `turn * dt`), runs on, bounces off the ropes (a `rope` event, no draw), tramples on
/// contact, and stomps when a stomp is due.
pub fn stampede_step(f: &mut Fight, k: usize, dt: f64) {
    let Some(st) = f.stampede.stamp[k].clone() else {
        return;
    };
    let r = f.rules.r;
    if f.t >= st.t1 {
        f.stampede.stamp[k] = None;
        f.balls[k].walking = false;
        f.ev("stampede_end", k as i64, vec![]);
        return;
    }
    let mut last = st.last;
    let mut next = st.next;
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let want = dmath::atan2(d[1], d[0]);
    let bv = f.balls[k].v;
    let cur = dmath::atan2(bv[1], bv[0]);
    let step = st.turn * dt;
    let a = cur + py_max(-step, py_min(step, geom::wrap(want - cur)));
    let sp = f.rules.v * st.speed;
    let lim = f.rules.a - r;
    {
        let b = &mut f.balls[k];
        b.v = [dmath::cos(a) * sp, dmath::sin(a) * sp];
        b.p = [b.p[0] + b.v[0] * dt, b.p[1] + b.v[1] * dt];
    }
    for ax in 0..2 {
        let b = &mut f.balls[k];
        if b.p[ax].abs() > lim {
            b.p[ax] = lim.copysign(b.p[ax]) * 2.0 - b.p[ax];
            b.v[ax] = -b.v[ax];
            let side = if b.p[ax].is_sign_negative() { -1 } else { 1 };
            let data = vec![
                Value::Int(ax as i64),
                Value::Int(side),
                Value::Float(b.p[0]),
                Value::Float(b.p[1]),
                Value::Float(sp),
            ];
            f.ev("rope", k as i64, data);
        }
    }
    let rel = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let dist = geom::hypot(rel);
    if dist < 2.0 * r + 0.04 && f.t - last >= st.cool {
        let n = if dist > 1e-6 {
            geom::div(rel, dist)
        } else {
            geom::div(f.balls[k].v, sp)
        };
        let bp = f.balls[k].p;
        let h = [bp[0] + n[0] * r, bp[1] + n[1] * r];
        if f.damage(k, 1 - k, st.dmg, "trample", h, 0) {
            f.knock(1 - k, n, st.knock);
            last = f.t;
            f.ev("trample", k as i64, vec![Value::Float(h[0]), Value::Float(h[1])]);
            // it charges on past (no grinding against it)
            let b = &mut f.balls[k];
            let vn = geom::dot(b.v, n);
            if vn > 0.0 {
                b.v = [b.v[0] - 2.0 * vn * n[0], b.v[1] - 2.0 * vn * n[1]];
            }
        }
    }
    if f.t + 1e-9 >= next {
        let rmax = r + st.stomp_r;
        let p = f.balls[k].p;
        f.waves.push(Wave {
            owner: k,
            c: p,
            r,
            speed: st.stomp_speed,
            rmax,
            dmg: st.stomp_dmg,
            knock: st.stomp_knock,
            kind: "stampede".to_string(),
            hit: false,
            name: st.name.clone(),
            t0: f.t,
            slow: None,
            stun: None,
            detune: None,
            grow: false,
            clears: false,
        });
        f.ev(
            "stampede_stomp",
            k as i64,
            vec![Value::Float(p[0]), Value::Float(p[1]), Value::Float(rmax)],
        );
        next += st.stomp16 * f.rules.beat / 4.0;
    }
    if let Some(s) = &mut f.stampede.stamp[k] {
        s.last = last;
        s.next = next;
    }
}
