//! Weapon kind `tether`, SYNTH's PATCH CABLES: `fight.py` 1572 to 1660 (`_tether`, `tether_beat`, `tether_16`,
//! `tether_step`).
//!
//! Every `every` beats, while fewer cables are plugged in (or flying) than its slots, the synth fires a plug straight at
//! the enemy; a plug that reaches the enemy jabs it and plugs a cable in for `life_beats` beats (+`grow` slots per landed
//! plug, up to `n_max`). On the 16th grid, every `volt16` 16ths (a haste divides it), a voltage pulse leaves down every
//! plugged cable and hits on arrival while its cable is still in. The leash: while any cable is in, the enemy cannot
//! get farther than `length` (it bounces off the end) unless it is flung harder than `snap` x the cables: then they all
//! snap. A rewinding enemy is intangible to all of it. `Ball::n_vol` holds the slots (recorded, `NV`).
//!
//! Python hangs `B.tc` on the ball the first time any of the three is called (`_tether`); here it is
//! [`State::balls`]. None of this code draws from the generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::fight::{Fight, radius, rewinding};
use crate::geom::{self, V2};
use crate::roster::{TetherCfg, WeaponCfg};

/// A plug in flight (`tc['plugs']`).
#[derive(Clone, Debug, PartialEq)]
pub struct Plug {
    /// Its id (the cable it becomes).
    pub id: i64,
    /// Where it is.
    pub p: V2,
    /// Its velocity.
    pub v: V2,
}

/// A plugged cable (`tc['cables']`).
#[derive(Clone, Debug, PartialEq)]
pub struct Cable {
    /// Its id (its plug's).
    pub id: i64,
    /// When it unplugs.
    pub t1: f64,
}

/// A voltage pulse on its way (`tc['volts']`).
#[derive(Clone, Debug, PartialEq)]
pub struct Volt {
    /// The cable it runs down.
    pub cid: i64,
    /// When it arrives.
    pub t_arr: f64,
    /// Its pulse number.
    pub n: i64,
}

/// One ball's cables (`B.tc`).
#[derive(Clone, Debug, PartialEq)]
pub struct Tc {
    /// The plugs in flight.
    pub plugs: Vec<Plug>,
    /// The plugged cables.
    pub cables: Vec<Cable>,
    /// The pulses on their way.
    pub volts: Vec<Volt>,
    /// The next plug's id.
    pub next: i64,
    /// The slots (a float: `grow` may add half a cable).
    pub slots: f64,
    /// The next pulse's number.
    pub pulse: i64,
}

/// This kind's per-fight state (`Fight::tether`): each ball's `B.tc`, made on first use.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.tc` of each ball (`None` until `_tether` makes it).
    pub balls: [Option<Tc>; 2],
}

/// `B.w['kind'] == 'tether'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Tether(_))
}

/// The tether's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a tether.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a TetherCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Tether(w) => w,
        other => panic!("fighter {k} ({}) has no tether weapon: {other:?}", cfg.name),
    }
}

/// `_tether(B)` (1572 to 1576): the ball's cables, made on first use (`slots = float(n0)`, `B.n_vol = slots`).
fn tc(f: &mut Fight, k: usize) {
    if f.tether.balls[k].is_none() {
        let n0 = weapon(f, k).n0;
        f.tether.balls[k] = Some(Tc {
            plugs: Vec::new(),
            cables: Vec::new(),
            volts: Vec::new(),
            next: 0,
            slots: n0,
            pulse: 0,
        });
        f.balls[k].n_vol = n0;
    }
}

fn state<'s>(f: &'s mut Fight, k: usize) -> &'s mut Tc {
    f.tether.balls[k].as_mut().expect("_tether made it")
}

/// The ball's tether slots, if it has cables (`getattr(B, 'tc', None)`: SAWTOOTH counts its arms by them).
pub fn slots(f: &Fight, k: usize) -> Option<f64> {
    f.tether.balls[k].as_ref().map(|t| t.slots)
}

/// `tether_beat(k, kb)` (1578 to 1593), from `run()` 5944 to 5945: every `every` beats, while fewer cables are plugged
/// in (or flying) than its slots, the synth fires a plug at the opponent (straight, `plug_speed`).
pub fn beat(f: &mut Fight, k: usize, kb: i64) {
    let w = weapon(f, k);
    tc(f, k);
    let t = f.tether.balls[k].as_ref().expect("made");
    // kb % w['every'] (an int or a float in Python: either way nonzero means "not this beat")
    if py::fmod_py(kb as f64, w.every) != 0.0 || (t.cables.len() + t.plugs.len()) as i64 >= (t.slots + 1e-9) as i64 {
        return;
    }
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let a = dmath::atan2(d[1], d[0]);
    let u = [dmath::cos(a), dmath::sin(a)];
    let pos = geom::add(f.balls[k].p, geom::scale(u, f.rules.r + 0.06));
    let st = state(f, k);
    let id = st.next;
    st.plugs.push(Plug {
        id,
        p: pos,
        v: geom::scale(u, w.plug_speed),
    });
    let data = vec![
        Value::Int(id),
        Value::Float(pos[0]),
        Value::Float(pos[1]),
        Value::Float(a),
        Value::Float(w.plug_speed),
    ];
    f.ev("plug_fire", k as i64, data);
    state(f, k).next += 1;
}

/// `tether_16(k, k16)` (1594 to 1607), from `run()` 5981 to 5986 (its own counter, `next16c`) after the caller's stun
/// test: every `volt16` 16ths (a haste divides it) a pulse leaves down every plugged cable; it arrives after the
/// distance between the balls / `volt_speed`.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    tc(f, k);
    // k16 % max(1, int(round(w['volt16'] / self.ab_mult(k)))): half to even; a huge quotient saturates
    let n = (py::round_half_even(w.volt16 / f.ab_mult(k)) as i64).max(1);
    if state(f, k).cables.is_empty() || k16 % n != 0 {
        return;
    }
    let dist = geom::hypot(geom::sub(f.balls[1 - k].p, f.balls[k].p));
    let t_arr = f.t + dist / w.volt_speed;
    let st = state(f, k);
    let pulse = st.pulse;
    let ids: Vec<i64> = st.cables.iter().map(|c| c.id).collect();
    for &cid in &ids {
        state(f, k).volts.push(Volt { cid, t_arr, n: pulse });
        f.ev("volt", k as i64, vec![Value::Int(cid), Value::Int(pulse)]);
    }
    state(f, k).pulse += 1;
}

/// `tether_step(k, dt)` (1608 to 1660), from `run()` 6129 to 6130: the plugs fly and land or miss, the cables unplug
/// when their time is up, the pulses arrive, and the leash holds or snaps.
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    tc(f, k);
    let o = 1 - k;
    let a = f.rules.a;
    let r_o = radius(f, o);
    let ghost = rewinding(f, o);
    // the plugs (Python iterates a copy of the list and removes from the original)
    let plugs = state(f, k).plugs.clone();
    for pl in plugs {
        let p = geom::add(pl.p, geom::scale(pl.v, dt));
        if let Some(q) = state(f, k).plugs.iter_mut().find(|q| q.id == pl.id) {
            q.p = p;
        }
        if py::py_max(p[0].abs(), p[1].abs()) > a {
            state(f, k).plugs.retain(|q| q.id != pl.id);
            f.ev(
                "plug_miss",
                k as i64,
                vec![Value::Int(pl.id), Value::Float(p[0]), Value::Float(p[1])],
            );
        } else if !ghost && geom::hypot(geom::sub(f.balls[o].p, p)) < r_o + w.plug_r {
            state(f, k).plugs.retain(|q| q.id != pl.id);
            if f.damage(k, o, w.plug_dmg, "plug", p, pl.id) {
                let t1 = f.t + w.life_beats * f.rules.beat;
                let st = state(f, k);
                st.cables.push(Cable { id: pl.id, t1 });
                st.slots = py::py_min(w.n_max, st.slots + w.grow);
                let slots = st.slots;
                f.balls[k].n_vol = slots;
                f.ev(
                    "plug",
                    k as i64,
                    vec![Value::Int(pl.id), Value::Float(p[0]), Value::Float(p[1])],
                );
            }
        }
    }
    // the cables whose time is up
    let cables = state(f, k).cables.clone();
    for c in cables {
        if f.t >= c.t1 {
            state(f, k).cables.retain(|q| q.id != c.id);
            f.ev("unplug", k as i64, vec![Value::Int(c.id), Value::Int(0)]);
        }
    }
    // the pulses that arrive (hit while their cable is still in)
    let live: Vec<i64> = state(f, k).cables.iter().map(|c| c.id).collect();
    let volts = state(f, k).volts.clone();
    for (i, v) in volts.iter().enumerate() {
        if f.t + 1e-9 >= v.t_arr {
            // tc['volts'].remove(V): the first equal entry
            let st = state(f, k);
            if let Some(j) = st.volts.iter().position(|q| q == v) {
                st.volts.remove(j);
            }
            let _ = i;
            if live.contains(&v.cid) && !ghost {
                let at = f.balls[o].p;
                f.damage(k, o, w.volt_dmg, "volt", at, v.n);
            }
        }
    }
    // the leash
    if !state(f, k).cables.is_empty() && !ghost {
        let rel = geom::sub(f.balls[o].p, f.balls[k].p);
        let dist = geom::hypot(rel);
        if dist > w.length {
            let n = geom::div(rel, dist);
            let n_cables = state(f, k).cables.len();
            if f.balls[o].boost > w.snap * n_cables as f64 {
                let op = f.balls[o].p;
                let data = vec![Value::Int(n_cables as i64), Value::Float(op[0]), Value::Float(op[1])];
                f.ev("snap", k as i64, data);
                let ids: Vec<i64> = state(f, k).cables.iter().map(|c| c.id).collect();
                for id in ids {
                    f.ev("unplug", k as i64, vec![Value::Int(id), Value::Int(1)]);
                }
                let st = state(f, k);
                st.cables.clear();
                st.volts.clear();
            } else {
                let bp = f.balls[k].p;
                let ob = &mut f.balls[o];
                ob.p = geom::add(bp, geom::scale(n, w.length));
                let vn = geom::dot(ob.v, n);
                if vn > 0.0 {
                    ob.v = geom::sub(ob.v, geom::scale(n, 2.0 * vn));
                    let op = ob.p;
                    f.ev("leash", k as i64, vec![Value::Float(op[0]), Value::Float(op[1])]);
                }
            }
        }
    }
}
