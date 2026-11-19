//! Weapon kind `tune`, OBOE's TUNING A: `fight.py` 2425 to 2525 (`tune_setup`, `_detune`, `tune_tick`, `tune_step`),
//! and the ability `endless` (`trigger` 2884 to 2888).
//!
//! Every `every` beats the oboe sounds its A as a narrow tuning beam. The aim locks `lock16` 16ths before the shot
//! (at the enemy's spot then, plus `lead` x its motion until the shot) and the beam fires `off16` 16ths after the bar
//! line: a capsule `len` beyond the surface, `width` wide, live `hold` seconds, hitting once (`dmg`, `knock`). A
//! landed beam knocks the enemy OUT OF TUNE (`detune`) for `dur_beats` at the oboe's depth (`depth0`, plus `grow` per
//! landed beam, up to `depth_max`): its hits deal x (1 - `weak` x depth) (`Fight::damage`), and each of the oboe's
//! notes on the `sting16` grid stings it for `sting` x depth. Between shots the bell turns to the enemy at `track`
//! radians a second (`th` is the aim).
//!
//! ENDLESS NOTE (ability kind `endless`): the beam held for the ability's time (normal shots pause), its aim turning
//! to the enemy at the ability's `track`; on every 8th the enemy touches it: `dmg`, and out of tune again at the
//! oboe's depth.
//!
//! The state Python hangs on the tuner's ball (`depth`, `tune_lock`, `tbeams`, `endless`) is [`State`], one entry per
//! ball; the out-of-tune status itself (`detune_t`, `detune_d`, `detune_w`) is on every `Ball`, where `damage` and
//! the timers read it. None of this code draws from the fight's generator.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::{ab_dur, param};
use crate::fight::{Fight, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::abilities::detune::DetuneCfg;
use crate::roster::kinds::tune::TuneCfg;
use crate::roster::{AbilityCfg, WeaponCfg};

/// One live beam of `B.tbeams`: `dict(o=o, d=u, t1=self.t + w['hold'], hit=False)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Beam {
    /// Where it leaves the ball.
    pub o: V2,
    /// Its unit direction.
    pub d: V2,
    /// When it goes out.
    pub t1: f64,
    /// Whether it has hit (a beam hits once).
    pub hit: bool,
}

/// `B.endless`: `dict(ab=ab, until=...)`, the held beam of ENDLESS NOTE.
#[derive(Clone, Debug, PartialEq)]
pub struct Endless {
    /// The ability that holds it (its `len`, `width`, `dmg`, `track`).
    pub ab: AbilityCfg,
    /// When it ends.
    pub until: f64,
}

/// What `tune_setup` and `trigger` hang on one ball.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BallTune {
    /// `B.depth`: how deep its next detune is.
    pub depth: f64,
    /// `B.tune_lock`: the locked aim of the next shot, if locked.
    pub tune_lock: Option<f64>,
    /// `B.tbeams`: its live beams.
    pub tbeams: Vec<Beam>,
    /// `B.endless`: its held beam, if any.
    pub endless: Option<Endless>,
}

/// Weapon kind `tune`'s per-fight state (`Fight::tune`): what `fight.py` hangs on each ball for this kind.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// By ball.
    pub balls: [BallTune; 2],
}

/// `B.w['kind'] == 'tune'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Tune(_))
}

/// The tuner's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k` is no tuner.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a TuneCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Tune(w) => w,
        other => panic!("fighter {k} ({}) has no tune weapon: {other:?}", cfg.name),
    }
}

/// `math.atan2(d[1], d[0]) % (2 * math.pi)`.
fn aim_to(d: V2) -> f64 {
    py::fmod_py(dmath::atan2(d[1], d[0]), 2.0 * dmath::PI)
}

/// `tune_setup(k)` (2436 to 2446), from `run()` 5797 to 5799
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    f.tune.balls[k] = BallTune {
        depth: w.depth0,
        tune_lock: None,
        tbeams: Vec::new(),
        endless: None,
    };
    let d = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    f.balls[k].th = aim_to(d);
    for x in &mut f.balls {
        (x.detune_t, x.detune_d, x.detune_w) = (0.0, 0.0, 0.0);
    }
}

/// `tune_tick(k, k16)` (2452 to 2495), from `run()` 6020 to 6024 (no stun test in the caller)
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    if stunned(f, k) {
        f.tune.balls[k].tune_lock = None;
        return;
    }
    let r = f.rules.r;
    let beat = f.rules.beat;
    if let Some(endless) = &f.tune.balls[k].endless {
        if f.t + 1e-9 >= endless.until {
            f.ev("endless_end", k as i64, vec![]);
            f.tune.balls[k].endless = None;
        } else if py::imod_py(k16, 2) == 0 {
            // ENDLESS NOTE: on the 8ths, the held beam bites
            let (len, width, dmg) = (
                param(&endless.ab, "len"),
                param(&endless.ab, "width"),
                param(&endless.ab, "dmg"),
            );
            let (b_p, b_th, o_p) = (f.balls[k].p, f.balls[k].th, f.balls[1 - k].p);
            let u = [dmath::cos(b_th), dmath::sin(b_th)];
            // o = B.p + u * (self.R.R + 0.04)
            let o = geom::add(b_p, geom::scale(u, r + 0.04));
            let rel = geom::sub(o_p, o);
            // s_ = min(ab['len'], max(0.0, float(rel @ u)))
            let s = py::py_min(len, py::py_max(0.0, geom::dot(rel, u)));
            let off = geom::sub(rel, geom::scale(u, s));
            if geom::hypot(off) < r + width / 2.0 {
                // hx, hy = (O.p - u * self.R.R).tolist()
                let h = geom::sub(o_p, geom::scale(u, r));
                if f.damage(k, 1 - k, dmg, "endless", h, 0) {
                    let depth = f.tune.balls[k].depth;
                    detune(f, 1 - k, k, depth, w.dur_beats * beat, w.weak);
                }
            }
        }
        return;
    }
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    let off16 = w.off16 as i64;
    let ph = py::imod_py(k16 - off16, main16);
    let lock = w.lock16 as i64;
    if lock > 0 && ph == py::imod_py(main16 - lock, main16) {
        let tl = lock as f64 * beat / 4.0;
        let (b, o) = (&f.balls[k], &f.balls[1 - k]);
        // d = (O.p + O.v * tl * w.get('lead', 0.0)) - (B.p + B.v * tl)
        let lead = geom::scale(geom::scale(o.v, tl), w.lead);
        let d = geom::sub(geom::add(o.p, lead), geom::add(b.p, geom::scale(b.v, tl)));
        let a = aim_to(d);
        f.tune.balls[k].tune_lock = Some(a);
        f.ev("tune_lock", k as i64, vec![Value::Float(a)]);
    }
    if ph == 0 && k16 >= off16 {
        let a = match f.tune.balls[k].tune_lock {
            Some(a) => a,
            None => aim_to(geom::sub(f.balls[1 - k].p, f.balls[k].p)),
        };
        let u = [dmath::cos(a), dmath::sin(a)];
        let o = geom::add(f.balls[k].p, geom::scale(u, r + 0.04));
        let t1 = f.t + w.hold;
        f.tune.balls[k].tbeams.push(Beam {
            o,
            d: u,
            t1,
            hit: false,
        });
        f.balls[k].th = a;
        let data = vec![
            Value::Float(o[0]),
            Value::Float(o[1]),
            Value::Float(a),
            Value::Float(w.len),
            Value::Float(w.width),
        ];
        f.ev("tune", k as i64, data);
        f.tune.balls[k].tune_lock = None;
    }
    // the oboe's notes sting the detuned enemy
    let o = &f.balls[1 - k];
    if py::imod_py(k16, w.sting16 as i64) == 0 && o.detune_t > 0.0 {
        let (dmg, at) = (w.sting * o.detune_d, o.p);
        f.damage(k, 1 - k, dmg, "sting", at, 0);
    }
}

/// `tune_step(k, dt)` (2496 to 2525), from `run()` 6150 to 6151
pub fn step(f: &mut Fight, k: usize, dt: f64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let beat = f.rules.beat;
    let o_p = f.balls[1 - k].p;
    match f.tune.balls[k].tune_lock {
        // locked for the shot
        Some(lock) => f.balls[k].th = lock,
        None => {
            let rel = geom::sub(o_p, f.balls[k].p);
            let want = dmath::atan2(rel[1], rel[0]);
            let rate = match &f.tune.balls[k].endless {
                Some(endless) => param(&endless.ab, "track"),
                None => w.track,
            };
            let step = rate * dt;
            let b = &mut f.balls[k];
            // B.th = (B.th + max(-step, min(step, _wrap(want - B.th)))) % (2 * math.pi)
            let turn = py::py_max(-step, py::py_min(step, geom::wrap(want - b.th)));
            b.th = py::fmod_py(b.th + turn, 2.0 * dmath::PI);
        }
    }
    // Python walks `B.tbeams`; nothing in the loop adds or removes a beam, so an index walk is the same
    for i in 0..f.tune.balls[k].tbeams.len() {
        let bm = &f.tune.balls[k].tbeams[i];
        if bm.hit {
            continue;
        }
        let d = bm.d;
        let rel = geom::sub(o_p, bm.o);
        // s_ = min(w['len'], max(0.0, float(rel @ Bm['d'])))
        let s = py::py_min(w.len, py::py_max(0.0, geom::dot(rel, d)));
        let off = geom::sub(rel, geom::scale(d, s));
        let dist = geom::hypot(off);
        if dist < r + w.width / 2.0 {
            f.tune.balls[k].tbeams[i].hit = true;
            // n = off / dist if dist > 1e-6 else -Bm['d']
            let n = if dist > 1e-6 {
                geom::div(off, dist)
            } else {
                [-d[0], -d[1]]
            };
            // hx, hy = (O.p - n * R).tolist()
            let h = geom::sub(o_p, geom::scale(n, r));
            let landed = f.damage(k, 1 - k, w.dmg, "tune", h, 0);
            // self.knock(1 - k, Bm['d'] + 0.3 * n, w['knock'])
            f.knock(1 - k, geom::add(d, geom::scale(n, 0.3)), w.knock);
            if landed {
                let depth = f.tune.balls[k].depth;
                detune(f, 1 - k, k, depth, w.dur_beats * beat, w.weak);
                // B.depth = min(w.get('depth_max', 1.0), B.depth + w['grow'])
                f.tune.balls[k].depth = py::py_min(w.depth_max, depth + w.grow);
            }
        }
    }
    let t = f.t;
    f.tune.balls[k].tbeams.retain(|bm| t < bm.t1 - 1e-9);
}

/// `_detune(vic, att, depth, dur, weak)` (2447 to 2451), from `effects_step` 3069 to 3071 (a ring with `detune` that
/// landed: `dur` is `detune['dur_beats'] * beat`), from a landed beam and from the held beam's bite. Event `detune`:
/// the victim, then the attacker, the depth and the seconds.
pub fn detune(f: &mut Fight, vic: usize, att: usize, depth: f64, dur: f64, weak: f64) {
    let v = &mut f.balls[vic];
    (v.detune_t, v.detune_d, v.detune_w) = (dur, depth, weak);
    let data = vec![Value::Int(att as i64), Value::Float(depth), Value::Float(dur)];
    f.ev("detune", vic as i64, data);
}

/// `effects_step` 3069 to 3071, a ring with `detune` that landed on `vic`: `dd_ = W['detune']; self._detune(o,
/// W['owner'], dd_['depth'], dd_['dur_beats'] * self.R.beat, dd_['weak'])` (it reads the option, then calls
/// [`detune`]): `dur = dur_beats * beat` is computed here, as `effects_step` does.
pub fn wave_detune(f: &mut Fight, vic: usize, att: usize, dd: &DetuneCfg) {
    let dur = dd.dur_beats * f.rules.beat;
    detune(f, vic, att, dd.depth, dur, dd.weak);
}

/// `trigger`, kind `endless` (2884 to 2888): `tune_lock`, `tbeams`, `endless`, event `endless`
pub fn start_endless(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + ab_dur(f, ab);
    let b = &mut f.tune.balls[k];
    b.tune_lock = None;
    b.tbeams = Vec::new();
    b.endless = Some(Endless { ab: ab.clone(), until });
    f.ev("endless", k as i64, vec![Value::Float(until)]);
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5801 to 5802: `DET` and `TUNE`, when a
/// fighter is a tuner.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) {
        &["DET", "TUNE"]
    } else {
        &[]
    }
}

/// This kind's rows of one frame: `snap`, `DET` and `TUNE` (5550 to 5554), into `out` (only the keys [`record_keys`]
/// gave): `DET` is each ball's (seconds left out of tune, depth), `TUNE` each ball's (a beam live 0 / 1, the held
/// beam 0 / 1).
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let [b0, b1] = &f.balls;
    out.push_f64("DET", &[2, 2], &[b0.detune_t, b0.detune_d, b1.detune_t, b1.detune_d]);
    let flag = |on: bool| if on { 1.0 } else { 0.0 };
    let [t0, t1] = &f.tune.balls;
    let row = [
        flag(!t0.tbeams.is_empty()),
        flag(t0.endless.is_some()),
        flag(!t1.tbeams.is_empty()),
        flag(t1.endless.is_some()),
    ];
    out.push_f64("TUNE", &[2, 2], &row);
}

#[cfg(test)]
mod tests;
