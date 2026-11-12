//! Weapon kind `drill`, SOUSAPHONE's DRILL SET: `fight.py` 4758 to 5005 (`drill_setup`, `drill_blocked`, `_drill_grow`,
//! `drill_tick`, `_drill_blast`, `_capsule_hit`, `_trample_baskets`, `drill_step`, `start_flash`, `start_front`), and
//! the abilities `flash_spin` and `company_front` (`trigger` 2977 to 2982).
//!
//! The marcher never glides (it is a walker: planted between sets, it only slides when knocked). Every `every` beats
//! on the 16th grid (haste divides it) it marches a drill set: `steps` steps along one straight line toward where the
//! enemy will be at the blast, every footfall on the grid. While it marches nothing knocks it and it gives no way in
//! a clash; it tramples the enemy on contact and every enemy basket near its rim. `halt16` 16ths after the last
//! footfall the bell blast fires straight ahead: a cone that hits once. A landed trample or blast grows the blast.
//! HORN FLASH spins the blast once round the horn; COMPANY FRONT marches a wall across the whole ring, a step a beat.
//!
//! Python hangs the state on the ball: `B.march` is [`March`] in `Ball::march`; `B.blast`, `B.flash` and `B.front`
//! are in [`State`] (`Fight::drill`), by fighter. The blast's length is the ball's `blade_len`.
//!
//! Draws from the fight's generator, in Python's order: one `uniform(-1, 1)` per set started (the line's jitter,
//! 4839), one `uniform()` per HORN FLASH (the spin's direction, 4990).

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, stunned};
use crate::geom::{self, V2, py_max, py_min};
use crate::record::Frames;
use crate::roster::{AbilityCfg, DrillCfg, WeaponCfg};

/// The bell blast of a set (`B.blast`, `_drill_blast` 4859 to 4866): a cone from `o` along `d`.
#[derive(Clone, Debug, PartialEq)]
pub struct Blast {
    /// The bell: where the cone starts.
    pub o: V2,
    /// The cone's direction, the set's line.
    pub d: V2,
    /// The cone's length (the ball's `blade_len` when it fired).
    pub len: f64,
    /// Its width at the bell.
    pub w: f64,
    /// Its width at the far end.
    pub w1: f64,
    /// When it fired.
    pub t0: f64,
    /// When it ends.
    pub t1: f64,
    /// Whether it has hit (it hits once).
    pub hit: bool,
}

/// HORN FLASH in progress (`B.flash`, `start_flash` 4985 to 4993): the blast sweeping once round the horn.
#[derive(Clone, Debug, PartialEq)]
pub struct Flash {
    /// When it started.
    pub t0: f64,
    /// When the turn is complete.
    pub t1: f64,
    /// The angle it starts at: facing away from the enemy.
    pub a0: f64,
    /// The way it turns: 1.0 or -1.0, drawn.
    pub dir: f64,
    /// The lane's length.
    pub len: f64,
    /// The lane's width.
    pub w: f64,
    /// Its damage.
    pub dmg: f64,
    /// Its knockback.
    pub knock: f64,
    /// Whether it has hit (it hits once).
    pub hit: bool,
}

/// COMPANY FRONT in progress (`B.front`, `start_front` 4995 to 5004): a wall across the ring, marching along axis
/// `ax` from `a` to `b`. Python keeps the ability's dict (`Fr['ab']`) and reads `thick`, `dmg` and `knock` from it
/// every step; they are the same numbers, copied here when the wall starts.
#[derive(Clone, Debug, PartialEq)]
pub struct Front {
    /// `ab.get('thick', 0.35)`: the wall's thickness.
    pub thick: f64,
    /// `ab['dmg']`.
    pub dmg: f64,
    /// `ab['knock']`.
    pub knock: f64,
    /// The axis the wall marches along: 0 (x) or 1 (y).
    pub ax: usize,
    /// The way it marches along that axis: 1.0 or -1.0.
    pub sg: f64,
    /// Where it starts: the rope behind the marcher.
    pub a: f64,
    /// Where it ends: the rope beyond the enemy.
    pub b: f64,
    /// When it started.
    pub t0: f64,
    /// When it ends.
    pub t1: f64,
    /// Its steps, one a beat.
    pub n: i64,
    /// The steps taken so far.
    pub i: i64,
    /// Where the wall is.
    pub pos: f64,
    /// Whether it has hit (it hits once).
    pub hit: bool,
}

/// Weapon kind `drill`'s per-fight state (`Fight::drill`): what `fight.py` hangs on the ball besides `march`, by
/// fighter. All `None` for a fighter that is not a driller, and between a driller's blasts and abilities.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.blast`.
    pub blast: [Option<Blast>; 2],
    /// `B.flash`.
    pub flash: [Option<Flash>; 2],
    /// `B.front`.
    pub front: [Option<Front>; 2],
}

/// A set's phase (`M['phase']`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// `'march'`: stepping along the line.
    March,
    /// `'halt'`: planted at the line's end, waiting for the blast.
    Halt,
}

/// `B.march` while a drill set marches (`drill_tick` 4853 to 4854): `Ball::march`.
#[derive(Clone, Debug, PartialEq)]
pub struct March {
    /// Marching, or halted before the blast.
    pub phase: Phase,
    /// Where the set started.
    pub p0: V2,
    /// Where it halts.
    pub p1: V2,
    /// The line's direction.
    pub u: V2,
    /// Its steps.
    pub n: i64,
    /// The footfalls so far.
    pub i: i64,
    /// When it started.
    pub t0: f64,
    /// The time of the last footfall.
    pub t1: f64,
    /// The time of each footfall, on the grid.
    pub t_steps: Vec<f64>,
    /// When the blast fires.
    pub t_blast: f64,
    /// The time of the set's last trample (-9.0 before the first).
    pub last: f64,
    /// 1 when the set started under haste (DOUBLE TIME), else 0.
    pub kind: i64,
}

/// `B.w['kind'] == 'drill'`: fighter `k`'s weapon is this kind.
/// It is also Python's `hasattr(B, 'march')` (`trigger` 2978, 2981; `snap` 5648): only `drill_setup` gives a ball
/// `march`.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Drill(_))
}

/// The driller's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k` is not a driller.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a DrillCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Drill(w) => w,
        other => panic!("fighter {k} ({}) is not a driller: {other:?}", cfg.name),
    }
}

/// `drill_setup(k)` (4781 to 4790), from `run()` 5900 to 5902: a walker, not marching, nothing in the air, and the
/// blast at its `base` length.
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let b = &mut f.balls[k];
    b.walker = true;
    b.walking = false;
    b.march = None;
    b.blade_len = w.base;
    f.drill.blast[k] = None;
    f.drill.flash[k] = None;
    f.drill.front[k] = None;
}

/// `drill_blocked(k)` (4791 to 4793): stunned (or sunk in shadow), held in an embrace, or in pieces.
pub fn blocked(f: &Fight, k: usize) -> bool {
    stunned(f, k) || f.balls[k].grabbed || f.balls[k].smashed > 0.0
}

/// `_drill_grow(k)` (4795 to 4797): a landed hit lengthens the blast by `grow`, up to `max_len`.
fn grow(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let b = &mut f.balls[k];
    b.blade_len = py_min(w.max_len, b.blade_len + w.grow);
}

/// `max(1, int(round(x / m)))`: a count of 16ths under the haste factor `m`, at least 1.
fn sixteenths(x: f64, m: f64) -> i64 {
    (py::round_half_even(x / m) as i64).max(1)
}

/// `drill_tick(k, k16)` (4799 to 4858), from `run()` 6086 to 6090, on every 16th `k16` of the fight.
///
/// While a set runs: a blocked marcher calls it off (`drill_off`); each footfall due is announced (`drill_step`: its
/// number and where it lands); after the last one the ball stands at the line's end (`drill_halt`); and when the
/// blast is due it fires ([`blast`]) and the set is over. Otherwise, on the set grid, a new set starts (`drill`:
/// from, to, start, last footfall, steps, 1 under haste).
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let s16 = f.rules.beat / 4.0;
    if f.balls[k].march.is_some() {
        if blocked(f, k) {
            f.balls[k].march = None;
            f.balls[k].walking = false;
            f.ev("drill_off", k as i64, vec![]);
            return;
        }
        // self.t + self.R.DT + 1e-9
        let now = f.t + f.rules.dt + 1e-9;
        let mut m = f.balls[k].march.take().expect("checked above");
        if m.phase == Phase::March {
            // footfalls on the grid
            while m.i < m.n && now >= m.t_steps[m.i as usize] {
                // q = M['p0'] + (M['p1'] - M['p0']) * (M['i'] + 1) / M['n']
                let (i1, n) = ((m.i + 1) as f64, m.n as f64);
                let q = [
                    m.p0[0] + (m.p1[0] - m.p0[0]) * i1 / n,
                    m.p0[1] + (m.p1[1] - m.p0[1]) * i1 / n,
                ];
                m.i += 1;
                f.ev(
                    "drill_step",
                    k as i64,
                    vec![Value::Int(m.i), Value::Float(q[0]), Value::Float(q[1])],
                );
            }
            if m.i >= m.n {
                f.balls[k].p = m.p1;
                m.phase = Phase::Halt;
                f.balls[k].walking = false;
                let p = f.balls[k].p;
                f.ev("drill_halt", k as i64, vec![Value::Float(p[0]), Value::Float(p[1])]);
            }
        }
        if m.phase == Phase::Halt && now >= m.t_blast {
            blast(f, k, m.u);
            // B.march = None
        } else {
            f.balls[k].march = Some(m);
        }
        return;
    }
    if blocked(f, k) {
        return;
    }
    let m = f.ab_mult(k);
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    // (k16 - int(w.get('off16', 0))) % max(1, int(round(main16 / m)))
    if py::imod_py(k16 - w.off16 as i64, sixteenths(main16 as f64, m)) != 0 {
        return;
    }
    let st16 = sixteenths(w.step16, m);
    let n = w.steps as i64;
    let dur = (n * st16) as f64 * s16;
    let halt = sixteenths(w.halt16, m) as f64 * s16;
    let t_bl = dur + halt;
    let lim = f.rules.a - r;
    let (bp, bv) = (f.balls[k].p, f.balls[k].v);
    let (op, ov) = (f.balls[1 - k].p, f.balls[1 - k].v);
    // tgt = np.clip(O.p + O.v * t_bl * w.get('lead', 0.0), -lim, lim): the prediction stays inside the ropes
    let tgt = [
        geom::np_clip(op[0] + ov[0] * t_bl * w.lead, -lim, lim),
        geom::np_clip(op[1] + ov[1] * t_bl * w.lead, -lim, lim),
    ];
    let d = geom::sub(tgt, bp);
    let dist = geom::hypot(d);
    let u = if dist > 1e-6 {
        geom::div(d, dist)
    } else {
        geom::div(bv, geom::hypot(bv) + 1e-9)
    };
    let j = geom::radians(f.rng.uniform(-1.0, 1.0) * w.jitter);
    let (cj, sj) = (dmath::cos(j), dmath::sin(j));
    let u = [u[0] * cj - u[1] * sj, u[0] * sj + u[1] * cj];
    // stop a radius short of the ropes
    let mut ln = w.span;
    if let Some(stop) = w.stop {
        // halt `stop` short of the enemy's spot at the blast
        ln = py_min(ln, py_max(w.min_len, dist - stop));
    }
    for ax in 0..2 {
        if u[ax].abs() > 1e-9 {
            let room = ((if u[ax] > 0.0 { lim } else { -lim }) - bp[ax]) / u[ax];
            ln = py_min(ln, py_max(0.0, room));
        }
    }
    let p1 = [bp[0] + u[0] * ln, bp[1] + u[1] * ln];
    // its heading
    f.balls[k].v = geom::scale(u, f.rules.v);
    // t_steps = [self.t + (i + 1) * st16 * s16 for i in range(n)]
    let t = f.t;
    let t_steps: Vec<f64> = (0..n).map(|i| t + ((i + 1) * st16) as f64 * s16).collect();
    let t1 = *t_steps
        .last()
        .expect("a set has at least one step (the roster refuses fewer)");
    let t_blast = t1 + halt;
    let kind = i64::from(m > 1.0);
    f.balls[k].march = Some(March {
        phase: Phase::March,
        p0: bp,
        p1,
        u,
        n,
        i: 0,
        t0: t,
        t1,
        t_steps,
        t_blast,
        last: -9.0,
        kind,
    });
    f.balls[k].walking = true;
    let data = vec![
        Value::Float(bp[0]),
        Value::Float(bp[1]),
        Value::Float(p1[0]),
        Value::Float(p1[1]),
        Value::Float(t),
        Value::Float(t1),
        Value::Int(n),
        Value::Int(kind),
    ];
    f.ev("drill", k as i64, data);
}

/// `_drill_blast(k, u)` (4859 to 4866): the bell blast fires along `u` from just past the ball's rim, as long as the
/// ball's `blade_len`, live for `hold` s. Event `blast`: the bell, the angle, the length, the two widths.
fn blast(f: &mut Fight, k: usize, u: V2) {
    let w = weapon(f, k);
    let b = &f.balls[k];
    let reach = f.rules.r + 0.04;
    let o = [b.p[0] + u[0] * reach, b.p[1] + u[1] * reach];
    let a = dmath::atan2(u[1], u[0]);
    let len = b.blade_len;
    let w1 = w.spread();
    f.drill.blast[k] = Some(Blast {
        o,
        d: u,
        len,
        w: w.width,
        w1,
        t0: f.t,
        t1: f.t + w.hold,
        hit: false,
    });
    let data = vec![
        Value::Float(o[0]),
        Value::Float(o[1]),
        Value::Float(a),
        Value::Float(len),
        Value::Float(w.width),
        Value::Float(w1),
    ];
    f.ev("blast", k as i64, data);
}

/// `_capsule_hit(o, d, ln, wd, p, wd1)` (4868 to 4874): whether a ball at `p` touches a beam from `o` along `d`,
/// `ln` long, `wd` wide at `o` widening to `wd1` at its far end (a cone; `None` = `wd` all along). Returns the
/// verdict and the offset from the beam's axis to `p`.
fn capsule_hit(f: &Fight, o: V2, d: V2, ln: f64, wd: f64, p: V2, wd1: Option<f64>) -> (bool, V2) {
    let rel = geom::sub(p, o);
    let s = py_min(ln, py_max(0.0, geom::dot(rel, d)));
    let off = [rel[0] - d[0] * s, rel[1] - d[1] * s];
    let w = match wd1 {
        None => wd,
        Some(wd1) => wd + (wd1 - wd) * s / py_max(ln, 1e-9),
    };
    (geom::hypot(off) < f.rules.r + w / 2.0, off)
}

/// `_trample_baskets(k, c, r)` (4876 to 4887): every enemy basket whose cobra is within `r` of `c` is gone (the
/// striker drops new ones as usual). Event `trample_basket`: the basket's id and where its cobra was. Nothing
/// happens when the enemy has no baskets (`getattr(O, 'baskets', None)`).
fn trample_baskets(f: &mut Fight, k: usize, c: V2, r: f64) {
    trample(f, k, |cp| geom::hypot(geom::sub(cp, c)) < r);
}

/// The loop `_trample_baskets` and COMPANY FRONT's wall (4972 to 4980) share: the enemy's baskets that `gone` says
/// are trampled leave the floor, each with its `trample_basket` event, in the floor's order.
fn trample(f: &mut Fight, k: usize, gone: impl Fn(V2) -> bool) {
    let Some(st) = f.striker[1 - k].as_mut() else {
        return;
    };
    if st.baskets.is_empty() {
        return;
    }
    let mut out = Vec::new();
    st.baskets.retain(|bk| {
        let hit = gone(bk.cp);
        if hit {
            out.push((bk.id, bk.cp));
        }
        !hit
    });
    for (id, cp) in out {
        let data = vec![Value::Int(id), Value::Float(cp[0]), Value::Float(cp[1])];
        f.ev("trample_basket", k as i64, data);
    }
}

/// `drill_step(k, dt)` (4889 to 4983), from `run()` 6187 to 6188, every step.
///
/// The march: the ball's place on the line, one eased step per footfall; the enemy within reach is trampled at most
/// once every `cool` s (`trample`: `trample_dmg`, knocked on ahead and out of the line; a landed one grows the
/// blast; event `drill_trample` either way), and the enemy's baskets near the rim are trampled. The blast: hits
/// once (`blast`; event `blast_hit`), ends after its hold. HORN FLASH: the lane turns once round, hits once
/// (`flash`; event `flash_hit`). COMPANY FRONT: the wall steps once a beat (`front_step`), hits once (`front`;
/// event `front_hit`), tramples the baskets it reaches and ends (`front_end`).
pub fn step(f: &mut Fight, k: usize, _dt: f64) {
    let w = weapon(f, k);
    let r = f.rules.r;
    let t = f.t;
    if let Some(m) = f.balls[k].march.as_ref().filter(|m| m.phase == Phase::March) {
        // position: one eased step per footfall (lift, swing, plant), all along the one line
        let n = m.n as f64;
        let s16 = (m.t1 - m.t0) / n;
        let fl = py_min(n, py_max(0.0, (t - m.t0) / py_max(1e-9, s16)));
        let i = (m.n - 1).min(fl as i64) as f64;
        let fr = fl - i;
        let fr = fr * fr * (3.0 - 2.0 * fr);
        let g = (i + fr) / n;
        let (u, last) = (m.u, m.last);
        let p = [m.p0[0] + (m.p1[0] - m.p0[0]) * g, m.p0[1] + (m.p1[1] - m.p0[1]) * g];
        f.balls[k].p = p;
        let rel = geom::sub(f.balls[1 - k].p, p);
        let dist = geom::hypot(rel);
        if dist < 2.0 * r + 0.04 && t - last >= w.cool {
            f.balls[k].march.as_mut().expect("marching").last = t;
            let mut side = [-u[1], u[0]];
            if geom::dot(side, rel) < 0.0 {
                side = [-side[0], -side[1]];
            }
            // hx, hy = (B.p + rel / max(dist, 1e-6) * R)
            let md = py_max(dist, 1e-6);
            let h = [p[0] + rel[0] / md * r, p[1] + rel[1] / md * r];
            if f.damage(k, 1 - k, w.trample_dmg, "trample", h, 0) {
                // bulldozed on ahead (fw) / out of the line
                let fw = w.trample_fwd;
                let away = [fw * u[0] + (1.0 - fw) * side[0], fw * u[1] + (1.0 - fw) * side[1]];
                f.knock(1 - k, away, w.trample_knock);
                grow(f, k);
            }
            f.ev("drill_trample", k as i64, vec![Value::Float(h[0]), Value::Float(h[1])]);
        }
        trample_baskets(f, k, p, r + w.basket_r);
    }
    if let Some(bl) = f.drill.blast[k].clone() {
        if !bl.hit {
            let op = f.balls[1 - k].p;
            let (hit, off) = capsule_hit(f, bl.o, bl.d, bl.len, bl.w, op, Some(bl.w1));
            if hit {
                f.drill.blast[k].as_mut().expect("live").hit = true;
                let dist = geom::hypot(off);
                let n = if dist > 1e-6 {
                    geom::div(off, dist)
                } else {
                    [-bl.d[0], -bl.d[1]]
                };
                let h = [op[0] - n[0] * r, op[1] - n[1] * r];
                if f.damage(k, 1 - k, w.dmg, "blast", h, 0) {
                    f.knock(1 - k, [bl.d[0] + 0.3 * n[0], bl.d[1] + 0.3 * n[1]], w.knock);
                    grow(f, k);
                }
                f.ev("blast_hit", k as i64, vec![Value::Float(h[0]), Value::Float(h[1])]);
            }
        }
        if t >= bl.t1 {
            f.drill.blast[k] = None;
        }
    }
    if let Some(fl) = f.drill.flash[k].clone() {
        let u = py_min(1.0, (t - fl.t0) / py_max(1e-9, fl.t1 - fl.t0));
        let a = fl.a0 + 2.0 * dmath::PI * u * fl.dir;
        let d = [dmath::cos(a), dmath::sin(a)];
        let bp = f.balls[k].p;
        let o = [bp[0] + d[0] * (r + 0.04), bp[1] + d[1] * (r + 0.04)];
        if !fl.hit {
            let op = f.balls[1 - k].p;
            let (hit, _off) = capsule_hit(f, o, d, fl.len, fl.w, op, None);
            if hit {
                f.drill.flash[k].as_mut().expect("live").hit = true;
                let rel = geom::sub(op, bp);
                // hx, hy = (O.p - rel / max(float(np.hypot(*rel)), 1e-6) * R)
                let md = py_max(geom::hypot(rel), 1e-6);
                let h = [op[0] - rel[0] / md * r, op[1] - rel[1] / md * r];
                if f.damage(k, 1 - k, fl.dmg, "flash", h, 0) {
                    f.knock(1 - k, rel, fl.knock);
                }
                f.ev("flash_hit", k as i64, vec![Value::Float(h[0]), Value::Float(h[1])]);
            }
        }
        if u >= 1.0 {
            f.drill.flash[k] = None;
        }
    }
    if let Some(mut fr) = f.drill.front[k].clone() {
        let bt = f.rules.beat;
        let n = fr.n as f64;
        let fl = py_min(n, py_max(0.0, (t - fr.t0) / bt));
        let i = (fr.n - 1).min(fl as i64) as f64;
        let e = fl - i;
        let e = e * e * (3.0 - 2.0 * e);
        let pos = fr.a + (fr.b - fr.a) * ((i + e) / n);
        if (fl as i64) > fr.i {
            fr.i = fl as i64;
            f.ev("front_step", k as i64, vec![Value::Int(fr.i), Value::Float(pos)]);
        }
        fr.pos = pos;
        let (ax, sg) = (fr.ax, fr.sg);
        let reach = r + fr.thick / 2.0;
        let op = f.balls[1 - k].p;
        let meets = !fr.hit && (op[ax] - pos).abs() < reach;
        if meets {
            fr.hit = true;
        }
        // the wall's state stands on the ball before the hit, as Python's dict does
        let (dmg, knock, t1) = (fr.dmg, fr.knock, fr.t1);
        f.drill.front[k] = Some(fr);
        if meets {
            if f.damage(k, 1 - k, dmg, "front", op, 0) {
                let mut nn = [0.0, 0.0];
                nn[ax] = sg;
                f.knock(1 - k, nn, knock);
            }
            f.ev("front_hit", k as i64, vec![Value::Float(op[0]), Value::Float(op[1])]);
        }
        trample(f, k, |cp| (cp[ax] - pos).abs() < reach);
        if t >= t1 {
            f.drill.front[k] = None;
            f.ev("front_end", k as i64, vec![]);
        }
    }
}

/// `trigger`, kind `flash_spin` (2977 to 2979): `if hasattr(B, 'march'): start_flash(k, ab)` (4985 to 4993). Only a
/// driller has `march` ([`is`]); on any other ball the ability does nothing.
///
/// The lane starts facing away from the enemy, so the sweep comes round; it turns once over `beats` beats, the way
/// drawn from the generator. Event `horn_flash`: the starting angle, the end time, the lane's length.
pub fn start_flash(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    if !is(f, k) {
        return;
    }
    let rel = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let a0 = dmath::atan2(rel[1], rel[0]) + dmath::PI;
    let t1 = f.t + param(ab, "beats") * f.rules.beat;
    let dir = if f.rng.uniform(0.0, 1.0) < 0.5 { 1.0 } else { -1.0 };
    let len = f.balls[k].blade_len * ab.get("len_k").unwrap_or(1.0);
    f.drill.flash[k] = Some(Flash {
        t0: f.t,
        t1,
        a0,
        dir,
        len,
        w: param(ab, "width"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        hit: false,
    });
    let data = vec![Value::Float(a0), Value::Float(t1), Value::Float(len)];
    f.ev("horn_flash", k as i64, data);
}

/// `trigger`, kind `company_front` (2980 to 2982): `if hasattr(B, 'march'): start_front(k, ab)` (4995 to 5004). Only
/// a driller has `march` ([`is`]); on any other ball the ability does nothing.
///
/// The wall marches along whichever axis parts the two balls most, from the rope behind the marcher to the rope
/// beyond the enemy, one step a beat for `int(beats)` beats. Event `company_front`: the axis, the way, the start and
/// the end time.
pub fn start_front(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    if !is(f, k) {
        return;
    }
    let a = f.rules.a;
    let rel = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let ax = if rel[0].abs() >= rel[1].abs() { 0 } else { 1 };
    let sg = if rel[ax] >= 0.0 { 1.0 } else { -1.0 };
    let n = param(ab, "beats") as i64;
    let t1 = f.t + n as f64 * f.rules.beat;
    f.drill.front[k] = Some(Front {
        thick: ab.get("thick").unwrap_or(0.35),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        ax,
        sg,
        a: -sg * a,
        b: sg * a,
        t0: f.t,
        t1,
        n,
        i: 0,
        pos: -sg * a,
        hit: false,
    });
    let data = vec![
        Value::Int(ax as i64),
        Value::Float(sg),
        Value::Float(f.t),
        Value::Float(t1),
    ];
    f.ev("company_front", k as i64, data);
}

/// The record keys this kind adds (Python's `rec` gets them at setup, `run()` 5904 to 5905): `DRILL`, when a
/// fighter is a driller.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if is(f, 0) || is(f, 1) { &["DRILL"] } else { &[] }
}

/// This kind's rows of one frame: `snap`, `DRILL` (5643 to 5670), a `(2, 16)` float row, NaN where nothing is live
/// (and all NaN for a fighter that is not a driller). `studio/docs/recording-format.md` 6.4 lists the columns.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut dr = [f64::NAN; 32];
    for j in 0..2 {
        if !is(f, j) {
            continue;
        }
        let row = &mut dr[j * 16..(j + 1) * 16];
        let b = &f.balls[j];
        row[0] = match &b.march {
            None => 0.0,
            Some(m) if m.phase == Phase::March => 1.0,
            Some(_) => 2.0,
        };
        if let Some(m) = &b.march {
            row[1] = (f.t - m.t0) / py_max(1e-9, m.t1 - m.t0);
            row[2] = dmath::atan2(m.u[1], m.u[0]);
            row[14] = m.i as f64;
        }
        row[3] = b.blade_len;
        if let Some(bl) = &f.drill.blast[j] {
            row[4] = (f.t - bl.t0) / py_max(1e-9, bl.t1 - bl.t0);
            row[5] = bl.len;
            row[6] = bl.w;
            row[12] = bl.o[0];
            row[13] = bl.o[1];
            row[2] = dmath::atan2(bl.d[1], bl.d[0]);
        }
        if let Some(fl) = &f.drill.flash[j] {
            let u = py_min(1.0, (f.t - fl.t0) / py_max(1e-9, fl.t1 - fl.t0));
            row[7] = fl.a0 + 2.0 * dmath::PI * u * fl.dir;
            row[8] = fl.len;
        }
        if let Some(fr) = &f.drill.front[j] {
            row[9] = fr.pos;
            row[10] = fr.ax as f64;
            row[11] = fr.sg;
            row[15] = (f.t - fr.t0) / py_max(1e-9, fr.t1 - fr.t0);
        }
    }
    out.push_f64("DRILL", &[2, 16], &dr);
}

#[cfg(test)]
mod tests {
    use bf_core::recording::Array;

    use super::*;
    use crate::fight::{Setup, run};
    use crate::roster::FighterCfg;
    use crate::testing::{episode_rules, fight, fighter, roster};

    /// SOUSAPHONE (left) against FLUTE under the example episode's rules.
    fn marching(roster: &[FighterCfg], seed: u64) -> Setup<'_> {
        Setup {
            fighters: [fighter(roster, "SOUSAPHONE"), fighter(roster, "FLUTE")],
            rules: episode_rules(),
            seed,
        }
    }

    #[test]
    fn a_set_is_eight_footfalls_on_the_8ths_then_a_halt_and_a_blast() {
        let roster = roster();
        let setup = marching(&roster, 1);
        let (s16, dt) = (setup.rules.beat / 4.0, setup.rules.dt);
        let rec = run(&setup, false).expect("the fight runs");
        let sets = rec.events.iter().filter(|e| e.kind == "drill" && e.who() == Some(0));
        assert!(sets.count() >= 3, "the marcher marches a set every two bars");
        let at = rec.events.iter().position(|e| e.kind == "drill").expect("a set");
        let set = &rec.events[at];
        // the first set starts with the fight, on the bar line: 8 steps, not hasted
        assert_eq!((set.i(9), set.i(10)), (8, 0));
        assert!((set.t - setup.rules.t_fight).abs() <= dt + 1e-5, "{}", set.t);
        assert!((set.f(7) - set.t).abs() < 1e-5, "the set starts when its event says");
        assert!(
            (set.f(8) - (set.f(7) + 16.0 * s16)).abs() < 1e-9,
            "8 steps of 2 sixteenths"
        );
        // its footfalls come in order, each on its 8th (within a step of the clock)
        let rest = &rec.events[at + 1..];
        let end = rest
            .iter()
            .position(|e| e.who() == Some(0) && matches!(e.kind.as_str(), "blast" | "drill_off"))
            .expect("the set ends with a blast or is called off");
        let steps: Vec<_> = rest[..end].iter().filter(|e| e.kind == "drill_step").collect();
        for (j, e) in steps.iter().enumerate() {
            assert_eq!(e.i(3), j as i64 + 1);
            let due = set.f(7) + (j as f64 + 1.0) * 2.0 * s16;
            assert!(
                (e.t - due).abs() <= dt + 1e-6,
                "footfall {} at {} for {due}",
                j + 1,
                e.t
            );
        }
        if rest[end].kind == "blast" {
            assert_eq!(steps.len(), 8);
            let halt = rest[..end].iter().find(|e| e.kind == "drill_halt").expect("a halt");
            // the ball stands at the line's end, and the blast starts just past its rim, along the line
            assert_eq!((halt.f(3), halt.f(4)), (set.f(5), set.f(6)));
            let blast = &rest[end];
            assert!((blast.t - (set.f(8) + 2.0 * s16)).abs() <= dt + 1e-6);
            let (dx, dy) = (set.f(5) - set.f(3), set.f(6) - set.f(4));
            assert!(
                (blast.f(5) - dmath::atan2(dy, dx)).abs() < 1e-9,
                "the blast fires along the line"
            );
            let reach = geom::hypot([blast.f(3) - halt.f(3), blast.f(4) - halt.f(4)]);
            assert!((reach - (setup.rules.r + 0.04)).abs() < 1e-9);
            // the blast is as long as the roster's base, plus what the set's tramples grew it by
            assert!((2.0..=3.6).contains(&blast.f(6)));
            assert_eq!((blast.f(7), blast.f(8)), (0.5, 1.6));
        }
    }

    #[test]
    fn the_record_has_a_drill_row_for_the_marcher_alone() {
        let roster = roster();
        let rec = run(&marching(&roster, 2), true).expect("the fight runs");
        let Some(Array::F64 { shape, data }) = rec.arrays.get("DRILL") else {
            panic!("a drill fight records DRILL as floats")
        };
        assert_eq!(shape[1..], [2, 16]);
        assert!(shape[0] > 100);
        let mut seen = [false; 3];
        for frame in data.chunks(32) {
            let (mine, other) = frame.split_at(16);
            assert!(other.iter().all(|v| v.is_nan()), "FLUTE is no driller");
            let phase = mine[0];
            assert!(phase == 0.0 || phase == 1.0 || phase == 2.0);
            seen[phase as usize] = true;
            // between sets: no progress, no footfalls; in a set: both
            assert_eq!(mine[1].is_nan(), phase == 0.0);
            assert_eq!(mine[14].is_nan(), phase == 0.0);
            // the blast's length: from the base, grown by landed hits, never past max_len
            assert!((2.0..=3.6).contains(&mine[3]));
            // a live blast has its progress, length, width and origin together
            let live = !mine[4].is_nan();
            assert!((0.0..1.0).contains(&mine[4]) || !live);
            for col in [5, 6, 12, 13] {
                assert_eq!(!mine[col].is_nan(), live, "column {col}");
            }
        }
        assert!(seen.iter().all(|&s| s), "it stands, marches and halts: {seen:?}");
        // a fight without a driller records no DRILL
        let other = Setup {
            fighters: [fighter(&roster, "FLUTE"), fighter(&roster, "TROMBONE")],
            rules: episode_rules(),
            seed: 2,
        };
        assert!(!run(&other, true).expect("runs").arrays.contains_key("DRILL"));
    }

    #[test]
    fn a_marcher_is_a_planted_walker_that_cannot_be_knocked_mid_set() {
        let roster = roster();
        let mut f = fight(&marching(&roster, 3), false);
        setup(&mut f, 0);
        assert!(f.balls[0].walker && !f.balls[0].walking && f.balls[0].march.is_none());
        assert_eq!(f.balls[0].blade_len, 2.0);
        f.t = f.rules.t_fight;
        tick(&mut f, 0, 0);
        let m = f.balls[0].march.clone().expect("a set starts on the first 16th");
        assert_eq!((m.phase, m.n, m.i, m.kind, m.last), (Phase::March, 8, 0, 0, -9.0));
        assert!(f.balls[0].walking);
        assert_eq!(f.events.last().map(|e| e.kind.as_str()), Some("drill"));
        // marching: a knock does not turn it
        let v = f.balls[0].v;
        f.knock(0, [0.0, 1.0], 2.0);
        assert_eq!((f.balls[0].v, f.balls[0].boost), (v, 0.0));
        // the next 16ths are no new set
        tick(&mut f, 0, 1);
        assert_eq!(f.events.iter().filter(|e| e.kind == "drill").count(), 1);
        // a stunned marcher calls the set off: no blast
        f.balls[0].stun = 1.0;
        assert!(blocked(&f, 0));
        tick(&mut f, 0, 2);
        assert!(f.balls[0].march.is_none() && !f.balls[0].walking);
        assert_eq!(f.events.last().map(|e| e.kind.as_str()), Some("drill_off"));
        // and starts none while it lasts
        tick(&mut f, 0, 32);
        assert_eq!(f.events.iter().filter(|e| e.kind == "drill").count(), 1);
    }

    #[test]
    fn the_blast_hits_inside_its_cone_only() {
        let roster = roster();
        let f = fight(&marching(&roster, 3), false);
        let r = f.rules.r;
        let (o, d) = ([0.0, 0.0], [1.0, 0.0]);
        // 2 long, 0.5 wide at the bell, 1.6 at the far end
        let hit = |p: V2| capsule_hit(&f, o, d, 2.0, 0.5, p, Some(1.6)).0;
        assert!(hit([1.0, 0.0]) && hit([2.0 + r, 0.0]));
        assert!(
            !hit([2.0 + r + 0.81, 0.0]),
            "past the far end by more than its half width"
        );
        // beside the bell the cone is narrow, at its far end wide
        assert!(!hit([0.0, r + 0.26]) && hit([0.0, r + 0.24]));
        assert!(hit([2.0, r + 0.79]) && !hit([2.0, r + 0.81]));
        // HORN FLASH's lane keeps one width
        let lane = |p: V2| capsule_hit(&f, o, d, 2.0, 0.5, p, None).0;
        assert!(lane([2.0, r + 0.24]) && !lane([2.0, r + 0.26]));
    }

    #[test]
    fn the_march_and_the_wall_trample_the_enemys_baskets() {
        let roster = roster();
        let mut f = fight(&marching(&roster, 3), false);
        crate::kinds::striker::setup(&mut f, 1);
        setup(&mut f, 0);
        f.t = f.rules.t_fight;
        let basket = |id: i64, at: V2| crate::kinds::striker::Basket {
            id,
            home: at,
            cp: at,
            t_drop: 0.0,
            strike: None,
            down: None,
        };
        let floor = |f: &Fight| -> Vec<i64> {
            let st = f.striker[1].as_ref().expect("a striker");
            st.baskets.iter().map(|b| b.id).collect()
        };
        f.striker[1].as_mut().expect("a striker").baskets =
            vec![basket(0, [0.5, 0.0]), basket(1, [2.0, 2.0]), basket(2, [-0.3, 0.4])];
        // within r + basket_r of a point: gone, in the floor's order, each with its event
        let reach = f.rules.r + 0.2;
        trample_baskets(&mut f, 0, [0.0, 0.0], reach);
        assert_eq!(floor(&f), [1]);
        let gone: Vec<(i64, f64, f64)> = f
            .events
            .iter()
            .filter(|e| e.kind == "trample_basket")
            .map(|e| (e.i(3), e.f(4), e.f(5)))
            .collect();
        assert_eq!(gone, [(0, 0.5, 0.0), (2, -0.3, 0.4)]);
        assert!(f.events.iter().all(|e| e.who() == Some(0)), "the marcher's events");
        // COMPANY FRONT's wall takes the basket it reaches
        let ab = &fighter(&roster, "SOUSAPHONE").low;
        f.balls[0].p = [-2.0, 0.0];
        f.balls[1].p = [2.0, 2.0];
        start_front(&mut f, 0, ab);
        let fr = f.drill.front[0].clone().expect("the wall");
        assert_eq!((fr.ax, fr.sg, fr.a, fr.b, fr.n), (0, 1.0, -f.rules.a, f.rules.a, 8));
        // six and a half beats in, the wall is past x = 2
        f.t += 6.5 * f.rules.beat;
        let dt = f.rules.dt;
        step(&mut f, 0, dt);
        assert!(floor(&f).is_empty());
        let kinds: Vec<&str> = f.events.iter().rev().take(4).map(|e| e.kind.as_str()).collect();
        assert_eq!(
            kinds,
            ["trample_basket", "front_hit", "hit", "front_step"],
            "newest first"
        );
        // a fighter without baskets: nothing to trample
        f.striker[1] = None;
        trample_baskets(&mut f, 0, [0.0, 0.0], 9.0);
    }

    #[test]
    fn the_abilities_do_nothing_on_a_ball_that_is_not_a_driller() {
        let roster = roster();
        let sousaphone = fighter(&roster, "SOUSAPHONE");
        let plain = Setup {
            fighters: [fighter(&roster, "FLUTE"), fighter(&roster, "TROMBONE")],
            rules: episode_rules(),
            seed: 5,
        };
        let (mut f, mut untouched) = (fight(&plain, false), fight(&plain, false));
        // `if hasattr(B, 'march')`: no event, no state, no draw
        start_flash(&mut f, 0, &sousaphone.unlock1);
        start_front(&mut f, 0, &sousaphone.low);
        assert!(f.events.is_empty() && f.drill == State::default());
        assert_eq!(f.rng.random(), untouched.rng.random());
        // on the marcher: HORN FLASH draws its way once and starts facing away from the enemy
        let mut f = fight(&marching(&roster, 5), false);
        let mut same = fight(&marching(&roster, 5), false);
        setup(&mut f, 0);
        f.balls[0].p = [0.0, 0.0];
        f.balls[1].p = [1.0, 0.0];
        start_flash(&mut f, 0, &sousaphone.unlock1);
        let fl = f.drill.flash[0].clone().expect("the flash");
        let way = if same.rng.uniform(0.0, 1.0) < 0.5 { 1.0 } else { -1.0 };
        assert_eq!((fl.a0, fl.dir, fl.len, fl.w), (dmath::PI, way, 2.0, 0.5));
        assert_eq!(fl.t1, f.t + f.rules.beat);
        assert_eq!(f.rng.random(), same.rng.random(), "one draw");
        let e = f.events.last().expect("the event");
        assert_eq!(
            (e.kind.as_str(), e.f(3), e.f(4), e.f(5)),
            ("horn_flash", fl.a0, fl.t1, 2.0)
        );
    }
}

/// The kind against `fight.py` on hand-built state: every expected number is the output of
/// `tools/studio_export/unit_checks/drill_unit_vectors.py` (floats as Python's `float.hex()`), compared to the bit.
#[cfg(test)]
mod kit_vectors {
    use bf_core::event::{Event, Value};

    use super::*;
    use crate::fight::Setup;
    use crate::kinds::striker;
    use crate::roster::FighterCfg;
    use crate::testing::{episode_rules, fight, fighter, roster};

    /// Python's `float.hex()` of a normal number or zero.
    fn hx(x: f64) -> String {
        if x == 0.0 {
            return if x.is_sign_negative() { "-0x0.0p+0" } else { "0x0.0p+0" }.to_string();
        }
        let bits = x.to_bits();
        let exp = ((bits >> 52) & 0x7ff) as i64 - 1023;
        let sign = if x < 0.0 { "-" } else { "" };
        format!("{sign}0x1.{:013x}p{exp:+}", bits & ((1 << 52) - 1))
    }

    fn vec(v: &[f64]) -> String {
        let parts: Vec<String> = v.iter().map(|&x| hx(x)).collect();
        format!("[{}]", parts.join(", "))
    }

    /// An event as the script prints it: the time, the kind, then every field.
    fn line(e: &Event) -> String {
        let mut out = vec![hx(e.t), e.kind.clone()];
        for v in &e.fields {
            out.push(match v {
                Value::Int(n) => n.to_string(),
                Value::Float(x) => hx(*x),
                Value::Str(s) => s.clone(),
                other => format!("{other:?}"),
            });
        }
        out.join(" ")
    }

    fn events(f: &Fight, from: usize) -> Vec<String> {
        f.events[from..].iter().map(line).collect()
    }

    /// `new()`: SOUSAPHONE (left) against FLUTE, seed 392, the example episode's rules, both kinds set up, at
    /// 12.5 s.
    fn new<'a>(roster: &'a [FighterCfg]) -> Fight<'a> {
        let mut f = fight(
            &Setup {
                fighters: [fighter(roster, "SOUSAPHONE"), fighter(roster, "FLUTE")],
                rules: episode_rules(),
                seed: 392,
            },
            false,
        );
        setup(&mut f, 0);
        striker::setup(&mut f, 1);
        f.t = 12.5;
        f
    }

    fn place(f: &mut Fight, bp: V2, op: V2, ov: V2) {
        f.balls[0].p = bp;
        f.balls[1].p = op;
        f.balls[1].v = ov;
    }

    fn march(f: &Fight) -> March {
        f.balls[0].march.clone().expect("a set")
    }

    fn baskets(f: &mut Fight, at: &[V2]) {
        f.striker[1].as_mut().expect("a striker").baskets = at
            .iter()
            .enumerate()
            .map(|(j, &c)| striker::Basket {
                id: j as i64,
                home: c,
                cp: c,
                t_drop: 0.0,
                strike: None,
                down: None,
            })
            .collect();
    }

    fn floor(f: &Fight) -> Vec<i64> {
        let st = f.striker[1].as_ref().expect("a striker");
        st.baskets.iter().map(|b| b.id).collect()
    }

    /// The wall at a time: the beats in, its position, its steps so far, whether it has hit, the enemy's HP, the
    /// baskets left and the step's events.
    type WallStep = (
        f64,
        &'static str,
        i64,
        bool,
        &'static str,
        &'static [i64],
        &'static [&'static str],
    );

    const T_STEPS: [&str; 8] = [
        "0x1.972d2d2d2d2d3p+3",
        "0x1.9e5a5a5a5a5a6p+3",
        "0x1.a587878787878p+3",
        "0x1.acb4b4b4b4b4bp+3",
        "0x1.b3e1e1e1e1e1ep+3",
        "0x1.bb0f0f0f0f0f1p+3",
        "0x1.c23c3c3c3c3c4p+3",
        "0x1.c969696969696p+3",
    ];

    fn hexes(v: &[f64]) -> Vec<String> {
        v.iter().map(|&x| hx(x)).collect()
    }

    #[test]
    fn a_set_starts_on_its_grid_with_the_kits_line() {
        let roster = roster();
        // A: off the grid nothing; on it the line, the footfalls, the blast time, the heading, one draw
        let mut f = new(&roster);
        place(&mut f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
        tick(&mut f, 0, 5);
        assert!(f.balls[0].march.is_none() && f.events.is_empty());
        tick(&mut f, 0, 32);
        let m = march(&f);
        assert_eq!(vec(&m.p0), "[-0x1.0000000000000p+0, 0x1.0000000000000p-1]");
        assert_eq!(vec(&m.p1), "[0x1.7380211abb718p+0, -0x1.821a35f308b82p+0]");
        assert_eq!(vec(&m.u), "[0x1.8c0d3da33708bp-1, -0x1.447ac960124f4p-1]");
        assert_eq!((m.n, m.kind), (8, 0));
        assert_eq!(hexes(&m.t_steps), T_STEPS);
        assert_eq!(hx(m.t_blast), "0x1.d096969696969p+3");
        assert_eq!(vec(&f.balls[0].v), "[0x1.016f01aa16f8ep+1, -0x1.a5d2d2967e33dp+0]");
        assert_eq!(
            events(&f, 0),
            [
                "0x1.9000000000000p+3 drill 0 -0x1.0000000000000p+0 0x1.0000000000000p-1 0x1.7380211abb718p+0 \
              -0x1.821a35f308b82p+0 0x1.9000000000000p+3 0x1.c969696969696p+3 8 0"
            ]
        );
        assert_eq!(hx(f.rng.uniform(0.0, 1.0)), "0x1.9ae5e6ad6ce20p-4");

        // B: under haste the set is every bar, on the 16ths
        let mut f = new(&roster);
        place(&mut f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
        f.balls[0].haste = 1.0;
        f.balls[0].haste_mult = 2.0;
        tick(&mut f, 0, 16);
        let m = march(&f);
        assert_eq!(vec(&m.p1), "[0x1.01bb3212ad5dep+0, -0x1.0e1a943b387b3p+0]");
        assert_eq!(vec(&m.u), "[0x1.94b4fb0657bafp-1, -0x1.399e386c17ca2p-1]");
        assert_eq!((m.n, m.kind), (8, 1));
        assert_eq!(
            hexes(&m.t_steps),
            [
                "0x1.9396969696969p+3",
                "0x1.972d2d2d2d2d3p+3",
                "0x1.9ac3c3c3c3c3cp+3",
                "0x1.9e5a5a5a5a5a6p+3",
                "0x1.a1f0f0f0f0f0fp+3",
                "0x1.a587878787878p+3",
                "0x1.a91e1e1e1e1e2p+3",
                "0x1.acb4b4b4b4b4bp+3"
            ]
        );
        assert_eq!(hx(m.t_blast), "0x1.b04b4b4b4b4b4p+3");
        assert_eq!(vec(&f.balls[0].v), "[0x1.070f3cc41f6ccp+1, -0x1.97b416261eed3p+0]");
        assert_eq!(
            events(&f, 0),
            [
                "0x1.9000000000000p+3 drill 0 -0x1.0000000000000p+0 0x1.0000000000000p-1 0x1.01bb3212ad5dep+0 \
              -0x1.0e1a943b387b3p+0 0x1.9000000000000p+3 0x1.acb4b4b4b4b4bp+3 8 1"
            ]
        );

        // C: a line cut short by the ropes, at its minimum length behind a near enemy
        let mut f = new(&roster);
        place(&mut f, [2.1, 2.2], [2.3, 2.35], [0.5, 2.5]);
        tick(&mut f, 0, 0);
        let m = march(&f);
        assert_eq!(vec(&m.p0), "[0x1.0cccccccccccdp+1, 0x1.199999999999ap+1]");
        assert_eq!(vec(&m.p1), "[0x1.31996aa8cd509p+1, 0x1.3333333333333p+1]");
        assert_eq!(vec(&m.u), "[0x1.a44d00ee52e0fp-1, 0x1.24638bde3ae89p-1]");
        assert_eq!(hexes(&m.t_steps), T_STEPS);
        assert_eq!(vec(&f.balls[0].v), "[0x1.11320d67b5df0p+1, 0x1.7c1b02a0e62e6p+0]");
    }

    #[test]
    fn the_march_tramples_with_the_kits_numbers() {
        // D
        let roster = roster();
        let mut f = new(&roster);
        place(&mut f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
        tick(&mut f, 0, 0);
        let n = f.events.len();
        let dt = f.rules.dt;
        f.t = 12.5 + 0.3;
        step(&mut f, 0, dt);
        assert_eq!(vec(&f.balls[0].p), "[-0x1.3989433f2e85ap-1, 0x1.759b00e0431a8p-3]");
        assert_eq!(f.events.len(), n);
        f.t = 12.5 + 0.4;
        let bp = f.balls[0].p;
        f.balls[1].p = [bp[0] + 0.9, bp[1] + 0.35];
        f.balls[1].v = [-2.0, 1.0];
        assert_eq!(hx(f.balls[1].hp), "0x1.9000000000000p+6");
        step(&mut f, 0, dt);
        assert_eq!(vec(&f.balls[0].p), "[-0x1.b2375b75afea2p-2, 0x1.ccc64f3a7b4e0p-6]");
        assert_eq!(hx(f.balls[1].hp), "0x1.7333333333333p+6");
        assert_eq!(vec(&f.balls[1].v), "[0x1.1f860e15ed9a0p+1, -0x1.4f2d70b47e181p+0]");
        assert_eq!(hx(f.balls[1].boost), "0x1.4cccccccccccdp+0");
        assert_eq!(hx(f.balls[0].blade_len), "0x1.0f5c28f5c28f6p+1");
        assert_eq!(hx(march(&f).last), "0x1.9cccccccccccdp+3");
        assert_eq!(
            events(&f, n),
            [
                "0x1.9cccccccccccdp+3 hit 0 1 0x1.ccccccccccccdp+2 0x1.0c52c09acc18cp-4 0x1.80075f605fea4p-2 trample 0",
                "0x1.9cccccccccccdp+3 drill_trample 0 0x1.0c52c09acc18cp-4 0x1.80075f605fea4p-2"
            ]
        );
        // within `cool`: no second trample
        let n = f.events.len();
        f.t += dt;
        step(&mut f, 0, dt);
        assert_eq!(f.events.len(), n);
        assert_eq!(hx(f.balls[1].hp), "0x1.7333333333333p+6");
    }

    #[test]
    fn the_footfalls_the_halt_and_the_blast_are_the_kits() {
        // E
        let roster = roster();
        let mut f = new(&roster);
        place(&mut f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
        tick(&mut f, 0, 0);
        let m = march(&f);
        let n = f.events.len();
        f.t = m.t_steps[1];
        tick(&mut f, 0, 4);
        assert_eq!(
            events(&f, n),
            [
                "0x1.9e5a5b9628cbdp+3 drill_step 0 1 -0x1.631ff7b95123ap-1 0x1.fde5ca0cf747ep-3",
                "0x1.9e5a5b9628cbdp+3 drill_step 0 2 -0x1.8c7fdee5448e8p-2 -0x1.0d1af9845c100p-9"
            ]
        );
        let now = march(&f);
        assert_eq!((now.i, now.phase, f.balls[0].walking), (2, Phase::March, true));
        let n = f.events.len();
        f.t = m.t_steps[7];
        tick(&mut f, 0, 16);
        assert_eq!(
            events(&f, n),
            [
                "0x1.c9696e58a32f4p+3 drill_step 0 3 -0x1.4aff395f9b570p-4 -0x1.032750ec8d144p-2",
                "0x1.c9696e58a32f4p+3 drill_step 0 4 0x1.ce00846aedc60p-3 -0x1.021a35f308b82p-1",
                "0x1.c9696e58a32f4p+3 drill_step 0 5 0x1.106029616a4dep-1 -0x1.82a0c36fcae62p-1",
                "0x1.c9696e58a32f4p+3 drill_step 0 6 0x1.ad4031a8192a4p-1 -0x1.0193a876468a2p+0",
                "0x1.c9696e58a32f4p+3 drill_step 0 7 0x1.25101cf764034p+0 -0x1.41d6ef34a7a12p+0",
                "0x1.c9696e58a32f4p+3 drill_step 0 8 0x1.7380211abb718p+0 -0x1.821a35f308b82p+0",
                "0x1.c9696e58a32f4p+3 drill_halt 0 0x1.7380211abb718p+0 -0x1.821a35f308b82p+0"
            ]
        );
        assert_eq!((march(&f).phase, f.balls[0].walking), (Phase::Halt, false));
        assert_eq!(vec(&f.balls[0].p), "[0x1.7380211abb718p+0, -0x1.821a35f308b82p+0]");
        let n = f.events.len();
        f.t = m.t_blast;
        f.balls[0].blade_len = 2.24;
        tick(&mut f, 0, 18);
        // the blast's event: every field the kit's bits but its angle, an output of `math.atan2`, where `dmath::atan2`
        // is one step off the C runtime's here (the kit: -0x1.5f6e95e3ac9c7p-1)
        let e = &f.events[n];
        assert_eq!(f.events.len(), n + 1);
        assert_eq!((hx(e.t).as_str(), e.kind.as_str()), ("0x1.d09691a75cd0cp+3", "blast"));
        assert_eq!(
            hexes(&[e.f(3), e.f(4), e.f(6), e.f(7), e.f(8)]),
            [
                "0x1.f23cafb55c6a1p+0",
                "-0x1.e9ef951c0456ap+0",
                "0x1.1eb851eb851ecp+1",
                "0x1.0000000000000p-1",
                "0x1.999999999999ap+0"
            ]
        );
        let kit_angle = -f64::from_bits(0x3fe5_f6e9_5e3a_c9c7);
        assert_eq!(hx(kit_angle), "-0x1.5f6e95e3ac9c7p-1");
        assert!(
            (e.f(5).to_bits() as i64 - kit_angle.to_bits() as i64).abs() <= 1,
            "{}",
            hx(e.f(5))
        );
        assert!(f.balls[0].march.is_none());
        let bl = f.drill.blast[0].clone().expect("the blast");
        assert_eq!(vec(&bl.o), "[0x1.f23cafb55c6a1p+0, -0x1.e9ef951c0456ap+0]");
        assert_eq!(vec(&bl.d), "[0x1.8c0d3da33708bp-1, -0x1.447ac960124f4p-1]");
        assert_eq!(
            hexes(&[bl.len, bl.w, bl.w1, bl.t1]),
            [
                "0x1.1eb851eb851ecp+1",
                "0x1.0000000000000p-1",
                "0x1.999999999999ap+0",
                "0x1.d896969696969p+3"
            ]
        );
        // the blast hits a ball 1.5 along it and 0.8 to its side
        let nn = [-bl.d[1], bl.d[0]];
        f.balls[1].p = [
            bl.o[0] + bl.d[0] * 1.5 + nn[0] * 0.8,
            bl.o[1] + bl.d[1] * 1.5 + nn[1] * 0.8,
        ];
        f.balls[1].v = [0.4, 2.5];
        let n = f.events.len();
        let dt = f.rules.dt;
        step(&mut f, 0, dt);
        assert_eq!(hx(f.balls[1].hp), "0x1.3000000000000p+6");
        assert_eq!(vec(&f.balls[1].v), "[0x1.496fcf974433ep+1, -0x1.798d05aff0816p-2]");
        assert_eq!(hx(f.balls[1].boost), "0x1.4cccccccccccdp+0");
        assert_eq!(hx(f.balls[0].blade_len), "0x1.2e147ae147ae2p+1");
        assert!(f.drill.blast[0].as_ref().is_some_and(|b| b.hit));
        assert_eq!(
            events(&f, n),
            [
                "0x1.d09691a75cd0cp+3 hit 0 1 0x1.8000000000000p+4 0x1.9ddca5d64a290p+1 -0x1.5ad85fc9dfe22p+1 blast 0",
                "0x1.d09691a75cd0cp+3 blast_hit 0 0x1.9ddca5d64a290p+1 -0x1.5ad85fc9dfe22p+1"
            ]
        );
        let n = f.events.len();
        f.t = bl.t1;
        step(&mut f, 0, dt);
        assert!(f.drill.blast[0].is_none() && f.events.len() == n);
    }

    #[test]
    fn the_cone_test_and_the_baskets_are_the_kits() {
        // F: o = (0.25, -0.5), d at 0.6 rad, 2.24 long, 0.5 wide at the bell, 1.6 at the far end or 0.5 all along
        let roster = roster();
        let mut f = new(&roster);
        let (o, d) = ([0.25, -0.5], [0.8253356149096783, 0.5646424733950354]);
        let cases: [(V2, bool, bool, &str); 5] = [
            ([1.0, 0.2], true, true, "[-0x1.64c0b3dd626e0p-4, 0x1.04bb65b02216cp-3]"),
            ([2.4, 1.9], true, false, "[0x1.347a6a00386a8p-2, 0x1.229c86065a254p+0]"),
            (
                [0.1, -1.4],
                false,
                false,
                "[-0x1.3333333333333p-3, -0x1.cccccccccccccp-1]",
            ),
            ([2.9, 0.9], true, true, "[0x1.9a3d35001c354p-1, 0x1.14e43032d12a0p-3]"),
            ([0.25, -0.5], true, true, "[0x0.0p+0, 0x0.0p+0]"),
        ];
        // The verdicts are the kit's. The offset is the kit's bits when `rel @ d` is added as numpy adds it on the
        // reference PC, with a fused multiply-add (shown here); the engine's dot products are plain sums (ruling
        // M3-R30), which moves the offset by a few last bits of a small difference.
        for (p, cone, lane, off) in cases {
            let rel = geom::sub(p, o);
            let s = py_min(2.24, py_max(0.0, dmath::fma(rel[1], d[1], rel[0] * d[0])));
            assert_eq!(
                vec(&[rel[0] - d[0] * s, rel[1] - d[1] * s]),
                off,
                "numpy's offset at {p:?}"
            );
            for (wd1, want) in [(Some(1.6), cone), (None, lane)] {
                let (hit, got) = capsule_hit(&f, o, d, 2.24, 0.5, p, wd1);
                assert_eq!(hit, want, "{p:?} {wd1:?}");
                let kit = [rel[0] - d[0] * s, rel[1] - d[1] * s];
                assert!(
                    (got[0] - kit[0]).abs() < 1e-15 && (got[1] - kit[1]).abs() < 1e-15,
                    "{p:?}: {got:?}"
                );
            }
        }
        // G: the cobras within r of c go, in the floor's order (the fourth is a hair outside)
        baskets(
            &mut f,
            &[[0.5, 0.0], [2.0, 2.0], [-0.3, 0.4], [0.0, 0.8000000000000002]],
        );
        let reach = f.rules.r + 0.2;
        trample_baskets(&mut f, 0, [0.0, 0.0], reach);
        assert_eq!(floor(&f), [1, 3]);
        assert_eq!(
            events(&f, 0),
            [
                "0x1.9000000000000p+3 trample_basket 0 0 0x1.0000000000000p-1 0x0.0p+0",
                "0x1.9000000000000p+3 trample_basket 0 2 -0x1.3333333333333p-2 0x1.999999999999ap-2"
            ]
        );
    }

    #[test]
    fn horn_flash_is_the_kits() {
        // H
        let roster = roster();
        let mut f = new(&roster);
        place(&mut f, [-0.4, 0.3], [0.9, 1.1], [1.0, 1.0]);
        let ab = &fighter(&roster, "SOUSAPHONE").unlock1;
        f.balls[0].blade_len = 2.36;
        start_flash(&mut f, 0, ab);
        let fl = f.drill.flash[0].clone().expect("the flash");
        assert_eq!(
            hexes(&[fl.a0, fl.t1, fl.dir, fl.len, fl.w]),
            [
                "0x1.d8bc56aa942b9p+1",
                "0x1.9e5a5a5a5a5a6p+3",
                "-0x1.0000000000000p+0",
                "0x1.2e147ae147ae1p+1",
                "0x1.0000000000000p-1"
            ]
        );
        assert_eq!(
            events(&f, 0),
            ["0x1.9000000000000p+3 horn_flash 0 0x1.d8bc56aa942b9p+1 0x1.9e5a5a5a5a5a6p+3 0x1.2e147ae147ae1p+1"]
        );
        let (beat, dt) = (f.rules.beat, f.rules.dt);
        f.t = 12.5 + 0.1 * beat;
        step(&mut f, 0, dt);
        assert!(f.drill.flash[0].as_ref().is_some_and(|fl| !fl.hit) && f.events.len() == 1);
        f.t = 12.5 + 0.5 * beat;
        step(&mut f, 0, dt);
        assert!(f.drill.flash[0].as_ref().is_some_and(|fl| fl.hit));
        assert_eq!(hx(f.balls[1].hp), "0x1.6800000000000p+6");
        assert_eq!(vec(&f.balls[1].v), "[0x1.13027d12d88cfp+1, 0x1.76d5ff6d38b05p+0]");
        assert_eq!(hx(f.balls[1].boost), "0x1.3333333333333p+0");
        assert_eq!(
            events(&f, 1),
            [
                "0x1.972d234eb9a17p+3 hit 0 1 0x1.4000000000000p+3 0x1.8e57544f54868p-2 0x1.923280576cb74p-1 flash 0",
                "0x1.972d234eb9a17p+3 flash_hit 0 0x1.8e57544f54868p-2 0x1.923280576cb74p-1"
            ]
        );
        f.t = fl.t1;
        step(&mut f, 0, dt);
        assert!(f.drill.flash[0].is_none() && f.events.len() == 3);
        // the draw after the flash's own is the kit's next one (one draw taken)
        assert_eq!(hx(f.rng.uniform(0.0, 1.0)), "0x1.9ae5e6ad6ce20p-4");
    }

    #[test]
    fn company_front_is_the_kits() {
        // I
        let roster = roster();
        let mut f = new(&roster);
        place(&mut f, [0.6, 1.9], [-0.2, -1.4], [1.0, 1.0]);
        let ab = &fighter(&roster, "SOUSAPHONE").low;
        start_front(&mut f, 0, ab);
        let fr = f.drill.front[0].clone().expect("the wall");
        assert_eq!((fr.ax, fr.n), (1, 8));
        assert_eq!(
            hexes(&[fr.sg, fr.a, fr.b, fr.t1]),
            [
                "-0x1.0000000000000p+0",
                "0x1.8000000000000p+1",
                "-0x1.8000000000000p+1",
                "0x1.0169696969696p+4"
            ]
        );
        assert_eq!(
            events(&f, 0),
            ["0x1.9000000000000p+3 company_front 0 1 -0x1.0000000000000p+0 0x1.9000000000000p+3 0x1.0169696969696p+4"]
        );
        baskets(&mut f, &[[0.5, 2.4], [2.0, 0.9], [-0.3, -2.9]]);
        let (beat, dt) = (f.rules.beat, f.rules.dt);
        let steps: [WallStep; 4] = [
            (
                0.3,
                "0x1.6b4395810624dp+1",
                0,
                false,
                "0x1.9000000000000p+6",
                &[1, 2],
                &["0x1.944e50c5eb314p+3 trample_basket 0 0 0x1.0000000000000p-1 0x1.3333333333333p+1"],
            ),
            (
                1.25,
                "0x1.1100000000000p+1",
                1,
                false,
                "0x1.9000000000000p+6",
                &[1, 2],
                &["0x1.a1f0ed3d859c9p+3 front_step 0 1 0x1.1100000000000p+1"],
            ),
            (
                2.7,
                "0x1.d2f1a9fbe76c8p-1",
                2,
                false,
                "0x1.9000000000000p+6",
                &[2],
                &[
                    "0x1.b6c0c1fc8f323p+3 front_step 0 2 0x1.d2f1a9fbe76c8p-1",
                    "0x1.b6c0c1fc8f323p+3 trample_basket 0 1 0x1.0000000000000p+1 0x1.ccccccccccccdp-1",
                ],
            ),
            (
                5.6,
                "-0x1.3c6a7ef9db230p+0",
                5,
                true,
                "0x1.2000000000000p+6",
                &[2],
                &[
                    "0x1.e0605681ecd4bp+3 front_step 0 5 -0x1.3c6a7ef9db230p+0",
                    "0x1.e0605681ecd4bp+3 hit 0 1 0x1.c000000000000p+4 -0x1.999999999999ap-3 -0x1.6666666666666p+0 front 0",
                    "0x1.e0605681ecd4bp+3 front_hit 0 -0x1.999999999999ap-3 -0x1.6666666666666p+0",
                ],
            ),
        ];
        for (beats, pos, i, hit, hp, left, evs) in steps {
            let n = f.events.len();
            f.t = 12.5 + beats * beat;
            step(&mut f, 0, dt);
            let now = f.drill.front[0].clone().expect("the wall");
            assert_eq!((hx(now.pos).as_str(), now.i, now.hit), (pos, i, hit), "{beats} beats");
            assert_eq!(hx(f.balls[1].hp), hp, "{beats} beats");
            assert_eq!(floor(&f), left, "{beats} beats");
            assert_eq!(events(&f, n), evs, "{beats} beats");
        }
        assert_eq!(vec(&f.balls[1].v), "[0x1.177ca99f69085p-1, -0x1.4561ec6105b8ap+1]");
        assert_eq!(hx(f.balls[1].boost), "0x1.ccccccccccccdp+0");
        let n = f.events.len();
        f.t = fr.t1;
        step(&mut f, 0, dt);
        assert!(f.drill.front[0].is_none());
        assert_eq!(
            events(&f, n),
            [
                "0x1.01696e58a32f4p+4 front_step 0 7 -0x1.8000000000000p+1",
                "0x1.01696e58a32f4p+4 trample_basket 0 2 -0x1.3333333333333p-2 -0x1.7333333333333p+1",
                "0x1.01696e58a32f4p+4 front_end 0"
            ]
        );
    }
}
